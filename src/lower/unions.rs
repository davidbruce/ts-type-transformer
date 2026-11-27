// ---- unions ----

use std::collections::{HashMap, HashSet};

use crate::front::Front;
use crate::ir::*;
use crate::xml::type_key;

use super::generics::{SYNTH_NAME_MAX, unique_with_limit};
use super::hoist::{collect_bodies, compact_hoist_path, free_type_params};
use super::overloads::{mnemonic, name_part};
use super::walk::walk;

// Each union that is not already a declaration's body becomes one: named by
// its members when there are few (`boolean_or_AddEventListenerOptions`),
// else by where it appears.
// `A[] | B[]` is `(A | B)[]`: a value of either is an array, and nothing at
// runtime says of which.
pub(super) fn merge_array_members(decls: &mut [Decl]) {
    walk(decls, &mut |t, _| {
        let Type::Union(parts) = t else { return true };
        if parts.iter().filter(|p| matches!(p, Type::Array(_))).count() < 2 {
            return true;
        }
        let mut rest = Vec::new();
        let mut elems = Vec::new();
        let mut seen = HashSet::new();
        let mut at = None;
        for p in parts.drain(..) {
            match p {
                Type::Array(inner) => {
                    at.get_or_insert(rest.len());
                    let inner = match *inner {
                        Type::Union(us) => us,
                        e => vec![e],
                    };
                    elems.extend(inner.into_iter().filter(|e| seen.insert(type_key(e))));
                }
                p => rest.push(p),
            }
        }
        let elem = if elems.len() == 1 { elems.pop().unwrap() } else { Type::Union(elems) };
        rest.insert(at.unwrap(), Type::Array(Box::new(elem)));
        *t = if rest.len() == 1 { rest.pop().unwrap() } else { Type::Union(rest) };
        true
    });
}

// `"a" | "b" | string` is `string`: an enum of strings beside `string` adds nothing.
pub(super) fn absorb_enums(decls: &mut [Decl]) {
    let string_enums: HashSet<String> = decls.iter().filter_map(|d| if let Decl::Enum(e) = d { Some(e) } else { None }).filter(|e| e.members.iter().all(|m| m.kind == "string")).map(|e| e.name.clone()).collect();
    walk(decls, &mut |t, _| {
        let Type::Union(parts) = t else { return true };
        if parts.iter().any(|p| p.is_prim("string")) {
            parts.retain(|p| !matches!(p, Type::Ref(r) if r.kind == "enum" && string_enums.contains(&r.name)));
            if parts.len() == 1 {
                *t = parts.pop().unwrap();
            }
        }
        true
    });
}

pub(super) fn collect_declared_names(decls: &[Decl], prefix: &str, sep: &str, out: &mut HashSet<String>) {
    for d in decls {
        if let Some(name) = match d {
            Decl::Function(_) | Decl::Var(_) => None,
            _ => Some(d.name()),
        } {
            out.insert(name.to_string());
            if !prefix.is_empty() {
                out.insert(format!("{prefix}_{name}"));
                if sep != "_" {
                    out.insert(format!("{prefix}{sep}{name}"));
                }
            }
        }
        if let Decl::Namespace(ns) = d {
            let next = if prefix.is_empty() { ns.name.clone() } else { format!("{prefix}_{}", ns.name) };
            collect_declared_names(&ns.decls, &next, sep, out);
        }
    }
}

fn collect_value_names(decls: &[Decl], prefix: &str, sep: &str, out: &mut HashSet<String>) {
    for d in decls {
        match d {
            Decl::Var(v) => {
                out.insert(v.name.clone());
                if !prefix.is_empty() {
                    out.insert(format!("{prefix}_{}", v.name));
                    if sep != "_" {
                        out.insert(format!("{prefix}{sep}{}", v.name));
                    }
                }
            }
            Decl::Function(f) => {
                out.insert(f.name.clone());
                if !prefix.is_empty() {
                    out.insert(format!("{prefix}_{}", f.name));
                    if sep != "_" {
                        out.insert(format!("{prefix}{sep}{}", f.name));
                    }
                }
            }
            Decl::Namespace(ns) => {
                let next = if prefix.is_empty() { ns.name.clone() } else { format!("{prefix}_{}", ns.name) };
                collect_value_names(&ns.decls, &next, sep, out);
            }
            _ => {}
        }
    }
}

fn collect_referenced_names(decls: &mut [Decl], out: &mut HashSet<String>) {
    walk(decls, &mut |t, _| {
        if let Type::Ref(r) = t
            && r.kind != "typeParameter"
        {
            out.insert(r.name.clone());
            if let Some(short) = r.name.rsplit('.').next() {
                out.insert(short.to_string());
            }
            if let Some(q) = &r.qualified {
                out.insert(q.clone());
                if let Some(short) = q.rsplit('.').next() {
                    out.insert(short.to_string());
                }
            }
        }
        true
    });
}

// The places where the library wrote a union itself, as the front end made
// them: two members or more besides `null`, none a union alias. Passes expand
// and copy types, so this is taken before them.
pub(super) fn written_unions(decls: &mut [Decl]) -> HashSet<String> {
    let aliases: HashSet<String> = decls.iter().filter_map(|d| if let Decl::Alias(a) = d && matches!(a.ty, Type::Union(_)) { Some(a.name.clone()) } else { None }).collect();
    let mut written = HashSet::new();
    walk(decls, &mut |t, path| {
        if let Type::Union(parts) = &*t {
            let members: Vec<&Type> = parts.iter().filter(|p| !p.is_prim("null") && !p.is_prim("undefined")).collect();
            if members.len() >= 2 && !members.iter().any(|p| matches!(p, Type::Ref(r) if r.kind == "typeAlias" && aliases.contains(&r.name))) {
                written.insert(path.join("\u{1}"));
            }
        }
        true
    });
    written
}

fn pascal_ident(s: &str, sep: &str) -> String {
    s.split(sep)
        .flat_map(|p| p.split('_'))
        .filter(|p| !p.is_empty())
        .map(|p| {
            let mut cs = p.chars();
            cs.next().map(|c| c.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        })
        .collect::<String>()
}

fn describe_atom(t: &Type) -> Option<String> {
    Some(match t {
        Type::Primitive(n) => n.clone(),
        Type::Ref(r) => r.name.replace('.', "_"),
        Type::Array(inner) => format!("{}_array", describe_atom(inner)?),
        Type::Nullable(inner, _) => format!("opt_{}", describe_atom(inner)?),
        Type::Readonly(inner) | Type::Tested(inner, _) => describe_atom(inner)?,
        Type::Literal { value, .. } => name_part(value),
        Type::Function(_) => "fn".into(),
        Type::Constructor(_) => "ctor".into(),
        Type::Tuple(_) => "tuple".into(),
        Type::Erased(_) => "any".into(),
        Type::This => "this".into(),
        Type::Union(_) | Type::Object(_) | Type::Intersection(..) | Type::Op(..) => return None,
    })
}

fn is_builtin_or_prelude(name: &str) -> bool {
    matches!(
        name,
        "String"
            | "Number"
            | "Boolean"
            | "Symbol"
            | "Void"
            | "Any"
            | "Array"
            | "Object"
            | "Date"
            | "Error"
            | "Promise"
            | "PromiseLike"
            | "Map"
            | "Set"
            | "Record"
            | "Result"
            | "Option"
            | "Type"
            | "Unit"
            | "List"
            | "Seq"
            | "Vec"
            | "Box"
            | "Copy"
            | "Clone"
            | "Send"
            | "Sync"
            | "Drop"
            | "Default"
            | "From"
            | "Into"
            | "AsRef"
            | "Iterator"
            | "Fn"
            | "FnMut"
            | "FnOnce"
            | "Self"
            | "Int"
            | "Float"
            | "Bool"
            | "Byte"
            | "Char"
            | "File"
            | "Null"
            | "Undefined"
            | "Never"
            | "Unknown"
    )
}

fn unwrap_ro(mut t: &Type) -> &Type {
    while let Type::Readonly(inner) | Type::Tested(inner, _) = t {
        t = inner;
    }
    t
}

/// Returns a human-readable, `_or_`-free name for common union shapes.
pub(super) fn structural_union_name(parts: &[Type], names: &HashSet<String>, sep: &str) -> Option<String> {
    let opt_arg_for = |obj: &Type| -> Option<String> {
        let Type::Ref(r) = obj else { return None };
        let short = r.name.rsplit('.').next().unwrap_or(&r.name);
        let suffixes = ["Options", "Option", "Config", "Init", "Params", "Formatter", "Mode", "Constraints", "Presets", "Hint", "Signal"];
        if suffixes.iter().any(|s| short.ends_with(s)) {
            let cand = format!("{}Arg", pascal_ident(&r.name.replace('.', "_"), sep));
            if cand.len() <= SYNTH_NAME_MAX && !cand.contains("_or_") {
                return Some(cand);
            }
        }
        None
    };
    let is_simple_prim = |t: &Type| matches!(t, Type::Primitive(p) if matches!(p.as_str(), "boolean" | "string" | "number"));
    if parts.len() == 2 {
        let p0 = unwrap_ro(&parts[0]);
        let p1 = unwrap_ro(&parts[1]);
        let is_promise_like = |t: &Type| matches!(t, Type::Ref(r) if r.name == "PromiseLike" || r.name.ends_with(".PromiseLike"));
        if is_promise_like(p0) && !is_promise_like(p1) {
            let other = describe_atom(p1)?;
            let cand = format!("Async{}", pascal_ident(&other, sep));
            if cand.len() <= SYNTH_NAME_MAX && !cand.contains("_or_") {
                return Some(cand);
            }
        } else if is_promise_like(p1) && !is_promise_like(p0) {
            let other = describe_atom(p0)?;
            let cand = format!("Async{}", pascal_ident(&other, sep));
            if cand.len() <= SYNTH_NAME_MAX && !cand.contains("_or_") {
                return Some(cand);
            }
        }
        let array_pair = match (p0, p1) {
            (other, Type::Array(inner)) | (Type::Array(inner), other) if type_key(unwrap_ro(inner)) == type_key(other) => describe_atom(other),
            _ => None,
        };
        if let Some(atom) = array_pair {
            let cand = format!("Multi{}", pascal_ident(&atom, sep));
            if cand.len() <= SYNTH_NAME_MAX && !cand.contains("_or_") {
                return Some(cand);
            }
        }
        if is_simple_prim(p0) && let Some(cand) = opt_arg_for(p1) {
            return Some(cand);
        }
        if is_simple_prim(p1) && let Some(cand) = opt_arg_for(p0) {
            return Some(cand);
        }
    } else if parts.len() == 3 {
        let unwrapped: Vec<&Type> = parts.iter().map(unwrap_ro).collect();
        let ref_indices: Vec<usize> = unwrapped.iter().enumerate().filter(|(_, p)| matches!(p, Type::Ref(_))).map(|(i, _)| i).collect();
        if ref_indices.len() == 1 {
            let ri = ref_indices[0];
            if unwrapped.iter().enumerate().all(|(i, p)| i == ri || is_simple_prim(p))
                && let Some(cand) = opt_arg_for(unwrapped[ri])
            {
                return Some(cand);
            }
        }
    }
    let atoms_in_order: Vec<String> = parts.iter().map(describe_atom).collect::<Option<Vec<_>>>()?;
    let mut sorted = atoms_in_order.clone();
    sorted.sort();
    let refs: Vec<&str> = sorted.iter().map(String::as_str).collect();
    let canonical = match refs.as_slice() {
        ["number", "string"] => Some("StringNumber".to_string()),
        ["boolean", "string"] => Some("BoolString".to_string()),
        ["boolean", "number"] => Some("NumberBool".to_string()),
        ["string", "symbol"] => Some("StringSymbol".to_string()),
        ["number", "string", "symbol"] => Some(if !names.contains("PropertyKey") { "PropertyKey" } else { "KeyValue" }.to_string()),
        ["boolean", "number", "string"] => Some("PrimitiveValue".to_string()),
        ["Date", "number", "string"] => Some("DateInput".to_string()),
        ["ArrayBuffer", "ArrayBufferView"] => Some(if !names.contains("BufferSource") { "BufferSource" } else { "BinaryData" }.to_string()),
        ["ArrayBuffer", "ArrayBufferView", "string"] => Some("BinaryText".to_string()),
        ["ArrayBuffer", "string"] => Some("BufferText".to_string()),
        ["Blob", "string"] => Some("BlobText".to_string()),
        ["URL", "string"] => Some("UrlInput".to_string()),
        ["Node", "string"] => Some("NodeText".to_string()),
        ["number_array", "string"] => Some("StringNumbers".to_string()),
        ["Float32Array", "number_array"] => Some("Float32Input".to_string()),
        ["Float32Array", "Float64Array"] => Some("FloatArray".to_string()),
        ["Element", "string"] | ["HTMLElement", "string"] => Some("ElementText".to_string()),
        _ => None,
    };
    if canonical.is_some() {
        return canonical;
    }
    if let [a, b] = atoms_in_order.as_slice() {
        let cand = format!("{}{}", pascal_ident(a, sep), pascal_ident(b, sep));
        if !cand.is_empty() && cand.len() <= SYNTH_NAME_MAX && !cand.contains("_or_") {
            return Some(cand);
        }
    }
    None
}

pub(super) fn describe_union_no_or(parts: &[Type]) -> Option<String> {
    if parts.len() <= 3 {
        let name = structural_union_name(parts, &HashSet::new(), "_")?;
        if !name.contains("_or_") {
            return Some(name);
        }
    }
    None
}

fn free_hoisted_collisions(decls: &mut [Decl], sep: &str) {
    let synth_ifaces: HashSet<String> = decls
        .iter()
        .filter_map(|d| if let Decl::Interface(i) = d && i.synthetic { Some(i.name.clone()) } else { None })
        .collect();
    if synth_ifaces.is_empty() {
        return;
    }
    let mut names = HashSet::new();
    collect_declared_names(decls, "", sep, &mut names);
    collect_referenced_names(decls, &mut names);
    let mut bodies = HashSet::new();
    collect_bodies(decls, &mut bodies);
    let mut hits: HashMap<String, bool> = HashMap::new();

    fn scan_type(t: &Type, path: &mut Vec<String>, in_array: bool, sep: &str, bodies: &HashSet<*const Type>, synth_ifaces: &HashSet<String>, hits: &mut HashMap<String, bool>) {
        if let Type::Union(parts) = t {
            if !bodies.contains(&(t as *const Type)) {
                let at = compact_hoist_path(path, sep);
                if synth_ifaces.contains(&at) && parts.iter().any(|p| matches!(p, Type::Ref(r) if r.kind == "interface" && r.name == at)) {
                    hits.entry(at).and_modify(|v| *v |= in_array).or_insert(in_array);
                }
            }
            for p in parts {
                scan_type(p, path, false, sep, bodies, synth_ifaces, hits);
            }
            return;
        }
        match t {
            Type::Ref(r) => r.args.iter().for_each(|a| scan_type(a, path, false, sep, bodies, synth_ifaces, hits)),
            Type::Array(inner) => scan_type(inner, path, true, sep, bodies, synth_ifaces, hits),
            Type::Readonly(inner) | Type::Nullable(inner, _) | Type::Tested(inner, _) => scan_type(inner, path, in_array, sep, bodies, synth_ifaces, hits),
            Type::Tuple(els) => els.iter().for_each(|e| scan_type(&e.ty, path, false, sep, bodies, synth_ifaces, hits)),
            Type::Function(s) | Type::Constructor(s) => scan_sig(s, path, sep, bodies, synth_ifaces, hits),
            Type::Object(ms) => scan_members(ms, path, sep, bodies, synth_ifaces, hits),
            _ => {}
        }
    }
    fn scan_sig(s: &Signature, path: &mut Vec<String>, sep: &str, bodies: &HashSet<*const Type>, synth_ifaces: &HashSet<String>, hits: &mut HashMap<String, bool>) {
        if let Some(t) = &s.this {
            scan_type(t, path, false, sep, bodies, synth_ifaces, hits);
        }
        for p in &s.params {
            path.push(p.name.clone());
            scan_type(&p.ty, path, false, sep, bodies, synth_ifaces, hits);
            path.pop();
        }
        if let Some(r) = &s.ret {
            path.push("result".into());
            scan_type(r, path, false, sep, bodies, synth_ifaces, hits);
            path.pop();
        }
    }
    fn scan_members(ms: &[Member], path: &mut Vec<String>, sep: &str, bodies: &HashSet<*const Type>, synth_ifaces: &HashSet<String>, hits: &mut HashMap<String, bool>) {
        for m in ms {
            let owner = m.from.as_ref().filter(|o| o.starts_with(|c: char| c.is_alphabetic() || c == '_')).zip(path.last_mut()).map(|(o, last)| std::mem::replace(last, o.clone()));
            let seg = match m.kind {
                MemberKind::Call => "call".to_string(),
                MemberKind::Construct => "new".to_string(),
                MemberKind::Index => "index".to_string(),
                _ => {
                    let mut seg = m.js_name.clone().unwrap_or_else(|| m.name.clone());
                    for c in m.sig.iter().flat_map(|s| s.params.iter().filter_map(|p| p.constant.as_deref())) {
                        seg = format!("{seg}_{}", name_part(c));
                    }
                    seg
                }
            };
            path.push(seg);
            if let Some(t) = &m.ty {
                scan_type(t, path, false, sep, bodies, synth_ifaces, hits);
            }
            if let Some(s) = &m.sig {
                scan_sig(s, path, sep, bodies, synth_ifaces, hits);
            }
            path.pop();
            if let (Some(owner), Some(last)) = (owner, path.last_mut()) {
                *last = owner;
            }
        }
    }
    fn scan_decls(ds: &[Decl], path: &mut Vec<String>, sep: &str, bodies: &HashSet<*const Type>, synth_ifaces: &HashSet<String>, hits: &mut HashMap<String, bool>) {
        for d in ds {
            match d {
                Decl::Interface(i) => {
                    path.push(i.name.clone());
                    if let Some(c) = &i.class {
                        scan_members(&c.constructors, path, sep, bodies, synth_ifaces, hits);
                        scan_members(&c.statics, path, sep, bodies, synth_ifaces, hits);
                    }
                    scan_members(&i.members, path, sep, bodies, synth_ifaces, hits);
                    path.pop();
                }
                Decl::Alias(a) => {
                    path.push(a.name.clone());
                    scan_type(&a.ty, path, false, sep, bodies, synth_ifaces, hits);
                    path.pop();
                }
                Decl::Function(f) => {
                    path.push(f.name.clone());
                    scan_sig(&f.sig, path, sep, bodies, synth_ifaces, hits);
                    path.pop();
                }
                Decl::Var(v) => {
                    path.push(v.name.clone());
                    scan_type(&v.ty, path, false, sep, bodies, synth_ifaces, hits);
                    path.pop();
                }
                Decl::Namespace(n) => {
                    path.push(n.name.clone());
                    scan_decls(&n.decls, path, sep, bodies, synth_ifaces, hits);
                    path.pop();
                }
                Decl::Enum(_) => {}
            }
        }
    }

    scan_decls(decls, &mut Vec::new(), sep, &bodies, &synth_ifaces, &mut hits);
    if hits.is_empty() {
        return;
    }
    let mut ordered: Vec<(String, bool)> = hits.into_iter().collect();
    ordered.sort_by(|a, b| a.0.cmp(&b.0));
    let mut renames = Vec::new();
    for (old, in_arr) in ordered {
        names.remove(&old);
        let suffix = if in_arr { "item" } else { "object" };
        let new_name = unique_with_limit(format!("{old}{sep}{suffix}"), SYNTH_NAME_MAX, &mut names);
        renames.push((old, new_name));
    }
    super::names::rename_everywhere(decls, &renames);
}

pub(super) fn name_unions(decls: &mut Vec<Decl>, sep: &str, written: &HashSet<String>) {
    free_hoisted_collisions(decls, sep);
    let mut names: HashSet<String> = HashSet::new();
    collect_declared_names(decls, "", sep, &mut names);
    collect_value_names(decls, "", sep, &mut names);
    collect_referenced_names(decls, &mut names);
    // A union that a declared alias also is uses that name, but one the library
    // wrote as a union there (`string | HTMLElement` beside `type Content =
    // string | HTMLElement`) gets its own name.
    let mut by_key: HashMap<String, String> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Alias(a) = d
            && a.type_params.is_empty()
            && matches!(a.ty, Type::Union(_))
        {
            by_key.entry(union_key(&a.ty)).or_insert_with(|| a.name.clone());
        }
    }
    let declared: HashSet<String> = by_key.keys().cloned().collect();
    // A named union may itself hold unions, inside its arrays: until none is left.
    let mut free_of: HashMap<String, Vec<Ref>> = HashMap::new();
    loop {
        let before = decls.len();
        let replaced = name_unions_once(decls, sep, &mut names, &mut by_key, &declared, written, &mut free_of);
        if decls.len() == before && !replaced {
            break;
        }
    }
}

// A union's key whatever the order of its members: `A | B` is `B | A`.
fn union_key(t: &Type) -> String {
    match t {
        Type::Union(parts) => {
            let mut keys: Vec<String> = parts.iter().map(type_key).collect();
            keys.sort();
            format!("union({})", keys.join("|"))
        }
        t => type_key(t),
    }
}

fn name_unions_once(decls: &mut Vec<Decl>, sep: &str, names: &mut HashSet<String>, by_key: &mut HashMap<String, String>, declared: &HashSet<String>, written: &HashSet<String>, free_of: &mut HashMap<String, Vec<Ref>>) -> bool {
    const GENERIC_WORDS: &[&str] = &[
        "v", "val", "value", "values", "x", "y", "a", "b", "arg", "args", "opt", "opts", "option", "options", "param", "params", "input", "item", "items", "elem", "el", "o", "obj", "t", "c", "s", "n", "k", "p", "r", "e", "d", "f", "fn", "cb", "target", "source", "src", "dst", "dest", "init", "config", "cfg", "props",
    ];
    let extract_word = |path: &[String]| -> Option<(String, bool)> {
        let mut segs = path.iter().rev().filter(|s| !matches!(s.as_str(), "index" | "result" | "return" | "value" | "new") && !s.chars().all(|c| c.is_ascii_digit()));
        let last = segs.next()?;
        let stepped_back = GENERIC_WORDS.contains(&last.as_str());
        let raw = if stepped_back { segs.next()? } else { last };
        let w = if raw.chars().next().is_some_and(|c| c.is_ascii_uppercase()) { raw.rsplit(sep).next().unwrap_or(raw) } else { raw.as_str() };
        let mut w = w.to_string();
        if path.last().map(String::as_str) == Some("result") && !w.to_ascii_lowercase().ends_with("result") {
            w = format!("{w}Result");
        }
        Some((w, stepped_back))
    };
    let camel = |s: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for c in s.chars() {
            if c.is_ascii_uppercase() || out.is_empty() {
                out.push(String::new());
            }
            out.last_mut().unwrap().push(c.to_ascii_lowercase());
        }
        out
    };
    let mut bodies: HashSet<*const Type> = HashSet::new();
    collect_bodies(decls, &mut bodies);
    let mut order: Vec<String> = Vec::new();
    let mut places: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    let mut sample: HashMap<String, Type> = HashMap::new();
    let bucket_of = |t: &Type, parts: &[Type], path: &[String]| -> String {
        let key = union_key(t);
        let own = declared.contains(&key) && written.contains(&path.join("\u{1}")) && parts.len() <= 3;
        if own { format!("written:{key}") } else { key }
    };
    walk(decls, &mut |t, path| {
        if !matches!(t, Type::Union(_)) || bodies.contains(&(t as *const Type)) {
            return true;
        }
        let Type::Union(parts) = &*t else { unreachable!() };
        let bucket = bucket_of(t, parts, path);
        if !by_key.contains_key(&bucket) {
            if !sample.contains_key(&bucket) {
                order.push(bucket.clone());
                sample.insert(bucket.clone(), t.clone());
            }
            places.entry(bucket).or_default().push(path.to_vec());
        }
        false
    });

    // Count plain concept names across single-concept buckets and multi-concept aliases
    // so only unambiguous words claim unqualified top-level names.
    let mut synth_names: HashSet<String> = HashSet::new();
    for d in decls.iter() {
        match d {
            Decl::Interface(i) if i.synthetic => {
                synth_names.insert(i.name.clone());
            }
            Decl::Alias(a) if a.synthetic => {
                synth_names.insert(a.name.clone());
            }
            _ => {}
        }
    }
    struct Plan {
        single_plain: Option<String>,
        best_path: Vec<String>,
        alias_words: Vec<String>,
    }
    let mut plans: HashMap<String, Plan> = HashMap::new();
    let mut plain_counts: HashMap<String, usize> = HashMap::new();
    for bucket in &order {
        let ps = &places[bucket];
        let best_path = super::hoist::pick_best_path(ps, &synth_names, sep);
        let extracted: Vec<(String, bool)> = ps.iter().filter_map(|p| extract_word(p)).collect();
        let words: HashSet<String> = extracted.iter().map(|(w, _)| w.clone()).collect();
        let single = (extracted.len() == ps.len())
            .then(|| words.iter().find(|w| words.iter().all(|o| camel(w).starts_with(&camel(o)))).cloned())
            .flatten();
        if let Some(w) = single {
            let stepped = extracted.iter().any(|(_, s)| *s);
            let p = pascal_ident(&w, sep);
            let plain = if stepped && !w.ends_with("Result") && !w.ends_with("Arg") { format!("{p}Arg") } else { p };
            if plain.len() >= 3 && !is_builtin_or_prelude(&plain) && !plain.contains("_or_") {
                *plain_counts.entry(plain.clone()).or_default() += 1;
                plans.insert(bucket.clone(), Plan { single_plain: Some(plain), best_path, alias_words: Vec::new() });
            } else {
                plans.insert(bucket.clone(), Plan { single_plain: None, best_path, alias_words: Vec::new() });
            }
        } else {
            let mut alias_words = Vec::new();
            let mut seen_a = HashSet::new();
            for (w, stepped) in extracted {
                if stepped
                    || w.len() < 3
                    || GENERIC_WORDS.contains(&w.as_str())
                    || w.ends_with("Result")
                    || ((w.starts_with("get") || w.starts_with("set")) && w.as_bytes().get(3).is_some_and(u8::is_ascii_uppercase))
                {
                    continue;
                }
                let p = pascal_ident(&w, sep);
                if p.len() >= 3 && !is_builtin_or_prelude(&p) && !p.contains("_or_") && seen_a.insert(p.clone()) {
                    *plain_counts.entry(p.clone()).or_default() += 1;
                    alias_words.push(p);
                }
            }
            plans.insert(bucket.clone(), Plan { single_plain: None, best_path, alias_words });
        }
    }

    let mut added = Vec::new();
    let mut deferred_aliases: Vec<(String, String)> = Vec::new();
    for bucket in &order {
        let t = &sample[bucket];
        let Type::Union(parts) = t else { unreachable!() };
        let plan = &plans[bucket];
        let at = compact_hoist_path(&plan.best_path, sep);
        let base = if let Some(plain) = &plan.single_plain {
            if plain_counts.get(plain).copied() == Some(1) && !names.contains(plain) {
                plain.clone()
            } else if let Some(s) = structural_union_name(parts, names, sep).filter(|s| !names.contains(s)) {
                s
            } else if !plan.best_path.is_empty() && !names.contains(&at) && !at.contains("_or_") {
                at
            } else if let Some(s) = structural_union_name(parts, names, sep) {
                s
            } else if !plan.best_path.is_empty() && !at.contains("_or_") {
                format!("{at}{sep}value")
            } else {
                let mut m: String = parts.iter().take(3).map(|p| pascal_ident(&mnemonic(p), sep)).collect();
                if m.is_empty() { m = "Value".into(); }
                m
            }
        } else if let Some(s) = structural_union_name(parts, names, sep).filter(|s| !names.contains(s)) {
            s
        } else if !plan.best_path.is_empty() && !names.contains(&at) && !at.contains("_or_") {
            at
        } else if !plan.best_path.is_empty() && !at.contains("_or_") {
            format!("{at}{sep}value")
        } else {
            let mut m: String = parts.iter().take(3).map(|p| pascal_ident(&mnemonic(p), sep)).collect();
            if m.is_empty() { m = "Value".into(); }
            m
        };
        let backing_name = unique_with_limit(base, SYNTH_NAME_MAX, names);
        let free = free_type_params(t);
        let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None, self_bounded: false }).collect();
        free_of.insert(backing_name.clone(), free.clone());
        by_key.insert(bucket.clone(), backing_name.clone());
        added.push(Decl::Alias(Alias { name: backing_name.clone(), sym: None, type_params, extends: Vec::new(), ty: t.clone(), computed: false, synthetic: true, doc: None }));
        if free.is_empty() && plan.single_plain.is_none() {
            for alias in &plan.alias_words {
                if plain_counts.get(alias).copied() == Some(1) && alias.len() <= SYNTH_NAME_MAX && !names.contains(alias) {
                    deferred_aliases.push((alias.clone(), backing_name.clone()));
                }
            }
        }
    }
    for (alias, backing_name) in deferred_aliases {
        if names.insert(alias.clone()) {
            added.push(Decl::Alias(Alias {
                name: alias,
                sym: None,
                type_params: Vec::new(),
                extends: Vec::new(),
                ty: Type::synthetic_ref(backing_name, "typeAlias", Vec::new()),
                computed: false,
                synthetic: true,
                doc: None,
            }));
        }
    }
    let mut replaced = false;
    walk(decls, &mut |t, path| {
        if !matches!(t, Type::Union(_)) || bodies.contains(&(t as *const Type)) {
            return true;
        }
        let Type::Union(parts) = &*t else { unreachable!() };
        let bucket = bucket_of(t, parts, path);
        let Some(name) = by_key.get(&bucket).cloned() else { return false };
        let args = free_of.get(&name).cloned().unwrap_or_default();
        *t = Type::synthetic_ref(name, "typeAlias", args.into_iter().map(Type::Ref).collect());
        replaced = true;
        false
    });
    decls.extend(added);
    replaced
}

// Says how each member of each union is recognised at runtime, so a
// generator can tell which one a value is: `typeof:string`, `array`,
// `instanceof:HTMLElement`, `has:once`, or `tag:circle` when one property
// (`discriminant`) tells them all apart.
pub(super) type PropShapes = Vec<(String, bool, Option<String>)>;

pub(super) fn annotate_unions(front: &mut Front, decls: &mut [Decl]) {
    // An enum's values are all strings, or all numbers: that tells it apart.
    fn enums(decls: &[Decl], out: &mut HashMap<String, &'static str>) {
        for d in decls {
            match d {
                Decl::Enum(e) if !e.members.is_empty() && e.members.iter().all(|m| m.kind == "string") => {
                    out.insert(e.name.clone(), "string");
                }
                Decl::Enum(e) if !e.members.is_empty() && e.members.iter().all(|m| m.kind == "number") => {
                    out.insert(e.name.clone(), "number");
                }
                Decl::Namespace(n) => enums(&n.decls, out),
                _ => {}
            }
        }
    }
    fn collect_iface_shapes(decls: &[Decl], prefix: &[String], out: &mut HashMap<String, PropShapes>) {
        for d in decls {
            match d {
                Decl::Interface(i) => {
                    let shape: PropShapes = i
                        .members
                        .iter()
                        .filter(|m| matches!(m.kind, MemberKind::Property | MemberKind::Method))
                        .map(|m| {
                            let literal = match (&m.ty, &m.constant) {
                                (_, Some(c)) => Some(c.clone()),
                                (Some(Type::Literal { value, .. }), _) => Some(value.clone()),
                                _ => None,
                            };
                            (m.name.clone(), m.optional, literal)
                        })
                        .collect();
                    let qual = if prefix.is_empty() { i.name.clone() } else { format!("{}.{}", prefix.join("."), i.name) };
                    out.insert(qual, shape.clone());
                    out.entry(i.name.clone()).or_insert(shape);
                }
                Decl::Namespace(n) => {
                    let mut next = prefix.to_vec();
                    next.push(n.name.clone());
                    collect_iface_shapes(&n.decls, &next, out);
                }
                _ => {}
            }
        }
    }
    fn collect_aliases(decls: &[Decl], prefix: &str, out: &mut HashMap<String, Type>) {
        for d in decls {
            match d {
                Decl::Alias(a) => {
                    let qual = if prefix.is_empty() { a.name.clone() } else { format!("{prefix}.{}", a.name) };
                    out.insert(qual, a.ty.clone());
                    out.entry(a.name.clone()).or_insert_with(|| a.ty.clone());
                }
                Decl::Namespace(n) => {
                    let mut next = prefix.to_string();
                    if !next.is_empty() {
                        next.push('.');
                    }
                    next.push_str(&n.name);
                    collect_aliases(&n.decls, &next, out);
                }
                _ => {}
            }
        }
    }
    fn fixed_tuple_len(t: &Type, aliases: &HashMap<String, Type>, depth: usize) -> Option<usize> {
        if depth > 8 {
            return None;
        }
        match t {
            Type::Tuple(elems) => elems.iter().all(|e| !e.rest && !e.optional).then_some(elems.len()),
            Type::Readonly(inner) => fixed_tuple_len(inner, aliases, depth + 1),
            Type::Ref(r) if r.kind == "typeAlias" => {
                let body = r.qualified.as_ref().and_then(|q| aliases.get(q)).or_else(|| aliases.get(&r.name))?;
                fixed_tuple_len(body, aliases, depth + 1)
            }
            _ => None,
        }
    }
    const CONCRETE_VIEWS: &[&str] = &[
        "DataView", "Int8Array", "Uint8Array", "Uint8ClampedArray", "Int16Array", "Uint16Array",
        "Int32Array", "Uint32Array", "Float32Array", "Float64Array", "BigInt64Array", "BigUint64Array",
    ];
    let mut enum_kinds = HashMap::new();
    enums(decls, &mut enum_kinds);
    let mut iface_shapes = HashMap::new();
    collect_iface_shapes(decls, &[], &mut iface_shapes);
    let mut alias_types = HashMap::new();
    collect_aliases(decls, "", &mut alias_types);
    let enum_test = |p: &Type| match p {
        Type::Ref(r) if r.kind == "enum" => enum_kinds.get(&r.name).map(|k| Test::TypeOf(k)),
        _ => None,
    };
    walk(decls, &mut |t, _| {
        let Type::Union(parts) = t else { return true };
        let shapes: Vec<Option<PropShapes>> = parts
            .iter()
            .map(|p| {
                front.object_shape(p).or_else(|| match p {
                    Type::Ref(r) if matches!(r.kind, "interface" | "class") => {
                        r.qualified.as_ref().and_then(|q| iface_shapes.get(q)).or_else(|| iface_shapes.get(&r.name)).cloned()
                    }
                    _ => None,
                })
            })
            .collect();
        let (discriminant, tests) = match discriminant_tests(&shapes) {
            Some((d, tests)) => (Some(d), tests),
            None => {
                let mut tests: Vec<Option<Test>> = parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| enum_test(p).or_else(|| front.runtime_test(p)).or_else(|| unique_property(&shapes, i).map(Test::Has)))
                    .collect();
                let array_indices: Vec<usize> = tests.iter().enumerate().filter(|(_, t)| matches!(t, Some(Test::Array))).map(|(i, _)| i).collect();
                if array_indices.len() >= 2
                    && let Some(lens) = array_indices.iter().map(|&i| fixed_tuple_len(&parts[i], &alias_types, 0)).collect::<Option<Vec<usize>>>()
                {
                    let unique: HashSet<usize> = lens.iter().copied().collect();
                    if unique.len() == lens.len() {
                        for (i, len) in array_indices.into_iter().zip(lens) {
                            tests[i] = Some(Test::TupleLen(len));
                        }
                    }
                }
                let has_concrete_view = tests.iter().any(|t| matches!(t, Some(Test::InstanceOf(c)) if CONCRETE_VIEWS.contains(&c.as_str())));
                let has_concrete_buffer = tests.iter().any(|t| matches!(t, Some(Test::InstanceOf(c)) if c == "ArrayBuffer" || c == "SharedArrayBuffer"));
                let has_promise_or_then = tests.iter().enumerate().any(|(j, t)| {
                    !matches!(t, Some(Test::Thenable))
                        && (matches!(t, Some(Test::InstanceOf(c)) if c == "Promise")
                            || shapes[j].as_ref().is_some_and(|s| s.iter().any(|p| p.0 == "then")))
                });
                for t in &mut tests {
                    match t {
                        Some(Test::ArrayBufferView) if has_concrete_view => *t = None,
                        Some(Test::ArrayBufferLike) if has_concrete_buffer => *t = None,
                        Some(Test::Thenable) if has_promise_or_then => *t = None,
                        _ => {}
                    }
                }
                (None, tests)
            }
        };
        for (p, test) in parts.iter_mut().zip(tests) {
            if let Some(test) = test {
                *p = Type::Tested(Box::new(std::mem::replace(p, Type::This)), test);
            }
        }
        if let Some(d) = discriminant {
            *t = Type::Tested(Box::new(std::mem::replace(t, Type::This)), Test::Discriminant(d));
        }
        // Members of a union are not themselves annotated again.
        false
    });
}

// A property every member has, required, whose literal values partition the members
// (at most one catch-all member without a literal, and any duplicate-literal group
// disambiguated by a required property unique within that group).
fn discriminant_tests(shapes: &[Option<PropShapes>]) -> Option<(String, Vec<Option<Test>>)> {
    if shapes.len() < 2 {
        return None;
    }
    let shapes: Vec<&PropShapes> = shapes.iter().map(|s| s.as_ref()).collect::<Option<_>>()?;
    let first = shapes.first()?;
    for name in first.iter().filter(|f| !f.1).map(|f| &f.0) {
        let mut groups: HashMap<&str, Vec<usize>> = HashMap::new();
        let mut catchall = 0usize;
        let mut ok = true;
        for (i, s) in shapes.iter().enumerate() {
            let Some(prop) = s.iter().find(|f| f.0 == *name && !f.1) else {
                ok = false;
                break;
            };
            match prop.2.as_deref() {
                Some(v) => groups.entry(v).or_default().push(i),
                None => {
                    catchall += 1;
                    if catchall > 1 {
                        ok = false;
                        break;
                    }
                }
            }
        }
        if !ok || groups.is_empty() {
            continue;
        }
        let mut tests: Vec<Option<Test>> = vec![None; shapes.len()];
        for (val, members) in groups {
            if members.len() == 1 {
                tests[members[0]] = Some(Test::Tag(val.to_string()));
            } else {
                for &idx in &members {
                    let Some(prop) = shapes[idx]
                        .iter()
                        .filter(|f| !f.1 && !f.0.contains(':'))
                        .find(|f| members.iter().all(|&j| j == idx || !shapes[j].iter().any(|g| g.0 == f.0)))
                        .map(|f| f.0.clone())
                    else {
                        ok = false;
                        break;
                    };
                    tests[idx] = Some(Test::TagHas(val.to_string(), prop));
                }
                if !ok {
                    break;
                }
            }
        }
        if ok {
            return Some((name.clone(), tests));
        }
    }
    None
}

// A required property of member `i` that no other member has.
// Every object has these (`"toString" in v` always holds): they tell nothing apart.
const OBJECT_PROTOTYPE: [&str; 7] = ["constructor", "hasOwnProperty", "isPrototypeOf", "propertyIsEnumerable", "toLocaleString", "toString", "valueOf"];

pub(super) fn unique_property(shapes: &[Option<PropShapes>], i: usize) -> Option<String> {
    let own = shapes[i].as_ref()?;
    own.iter()
        .filter(|f| !f.1 && !OBJECT_PROTOTYPE.contains(&f.0.as_str()))
        .find(|f| shapes.iter().enumerate().all(|(j, s)| j == i || s.as_ref().is_none_or(|s| !s.iter().any(|g| g.0 == f.0))))
        .map(|f| f.0.clone())
}

pub(super) fn flatten_unions(decls: &mut [Decl]) {
    walk(decls, &mut |t, _| {
        while let Type::Union(parts) = t
            && parts.iter().any(|p| matches!(p, Type::Union(_)))
        {
            let mut flat = Vec::new();
            let mut seen = HashSet::new();
            for p in parts.drain(..) {
                match p {
                    Type::Union(inner) => flat.extend(inner),
                    p => flat.push(p),
                }
            }
            flat.retain(|p| !matches!(p, Type::Union(_)) && seen.insert(type_key(p)) || matches!(p, Type::Union(_)));
            *t = Type::Union(flat);
        }
        true
    });
}
