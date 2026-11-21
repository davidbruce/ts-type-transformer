// ---- overloads ----

use std::collections::HashSet;

use crate::ir::*;
use crate::xml::{member_key, signature_key, type_key};

use super::generics::{NAME_MAX, keyed_map, unique};
use super::Options;
use super::walk::each_member_list;

// One signature per way of calling: optional parameters and union-typed
// parameters are spelled out.
fn expand_signature(sig: &Signature, o: Options) -> Vec<Signature> {
    let mut out = vec![sig.clone()];
    if o.optionals {
        // Every prefix that ends after the last required parameter is callable.
        let required = sig.params.iter().rposition(|p| !p.optional && !p.rest).map_or(0, |i| i + 1);
        // A rest parameter takes an empty array: the overload without it says nothing more.
        let rest_at = sig.params.last().filter(|p| p.rest).map(|_| sig.params.len() - 1);
        out = (required..=sig.params.len())
            .filter(|n| Some(*n) != rest_at)
            .map(|n| {
                let mut s = sig.clone();
                s.params.truncate(n);
                s.params.iter_mut().for_each(|p| p.optional = false);
                s
            })
            .collect();
    }
    if o.param_unions {
        let mut expanded = Vec::new();
        for s in out {
            let mut variants = vec![s.clone()];
            for (i, p) in s.params.iter().enumerate() {
                let (parts, wrap): (&[Type], Option<Absent>) = match &p.ty {
                    Type::Union(parts) => (parts, None),
                    Type::Nullable(inner, absent) => match &**inner {
                        Type::Union(parts) => (parts, Some(*absent)),
                        _ => continue,
                    },
                    _ => continue,
                };
                // Too many combinations: leave the union for a later pass.
                if variants.len() * parts.len() > o.union_param_limit {
                    continue;
                }
                variants = variants
                    .iter()
                    .flat_map(|v| {
                        parts.iter().map(move |part| {
                            let mut v = v.clone();
                            v.params[i].ty = match wrap {
                                Some(absent) => Type::Nullable(Box::new(part.clone()), absent),
                                None => part.clone(),
                            };
                            v
                        })
                    })
                    .collect();
            }
            expanded.extend(variants);
        }
        out = expanded;
    }
    out
}

pub(super) fn expand_members(members: &mut Vec<Member>, o: Options) {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for m in members.drain(..) {
        match &m.sig {
            Some(sig) if m.kind != MemberKind::Index => {
                for s in expand_signature(sig, o) {
                    let mut v = m.clone();
                    v.sig = Some(s);
                    if seen.insert(member_key(&v)) {
                        out.push(v);
                    }
                }
            }
            _ => out.push(m),
        }
    }
    *members = out;
}

pub(super) fn expand_functions(decls: &mut Vec<Decl>, o: Options) {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for d in decls.drain(..) {
        match d {
            Decl::Function(f) => {
                for sig in expand_signature(&f.sig, o) {
                    if seen.insert(format!("{} {}", f.name, signature_key(&sig))) {
                        out.push(Decl::Function(Function { name: f.name.clone(), js_name: f.js_name.clone(), sig }));
                    }
                }
            }
            Decl::Namespace(mut n) => {
                expand_functions(&mut n.decls, o);
                out.push(Decl::Namespace(n));
            }
            d => out.push(d),
        }
    }
    *decls = out;
}

// Names for overloads. An overload with fixed arguments is named by their
// values: `createEvent_MouseEvent`. The first one without is the plain name;
// each other one by the least that tells it apart: the parameters it adds
// (`f_with_options`), else the one type that differs (`f_with_Blob`), else a number.
fn overload_names(base: &str, sigs: &[&Signature], taken: &mut HashSet<String>, o: Options) -> Vec<String> {
    let sep = o.separator;
    let constants = |s: &Signature| -> Vec<String> { s.params.iter().filter_map(|p| p.constant.as_deref()).map(name_part).collect() };
    // Overloads with the same fixed values are named as a family of their own,
    // after the values: `addEventListener_click`, `addEventListener_click_with_options`.
    let mut out = vec![String::new(); sigs.len()];
    let mut families: Vec<(Vec<String>, Vec<usize>)> = Vec::new();
    for (i, s) in sigs.iter().enumerate() {
        let key = constants(s);
        match families.iter_mut().find(|f| f.0 == key) {
            Some(f) => f.1.push(i),
            None => families.push((key, vec![i])),
        }
    }
    for (fixed, members) in families {
        let family_base = if fixed.is_empty() || o.numbered_overloads { base.to_string() } else { format!("{base}{sep}{}", fixed.join(sep)) };
        let family: Vec<&Signature> = members.iter().map(|&i| sigs[i]).collect();
        let names = family_names(&family_base, &family, fixed.is_empty(), taken, o);
        for (i, name) in members.into_iter().zip(names) {
            out[i] = name;
        }
    }
    out
}

// ---- overload marks ----

// Records on each overload what tells it apart from the others where a
// target loses it: its type parameters' constraints (`merge<T extends
// Dictionary>` beside `merge<T>`), the literals of its options (`{async:
// true}` is `async`, a string its value; `false` is the default, nothing).
// Words to name it by; kept only where the overloads' differ.
pub(super) fn mark_overloads(decls: &mut [Decl]) {
    fn mark(sigs: Vec<&mut Signature>) {
        let marks: Vec<Vec<String>> = sigs.iter().map(|s| signature_marks(s)).collect();
        // Marked before: kept, as erasing may have lost them since.
        if sigs.iter().any(|s| !s.marks.is_empty()) || marks.iter().all(|m| *m == marks[0]) {
            return;
        }
        for (s, m) in sigs.into_iter().zip(marks) {
            s.marks = m;
        }
    }
    each_member_list(decls, &mut |members| {
        let mut groups: Vec<((u8, String), Vec<usize>)> = Vec::new();
        for (i, m) in members.iter().enumerate() {
            if !matches!(m.kind, MemberKind::Method | MemberKind::Call | MemberKind::Construct) || m.sig.is_none() {
                continue;
            }
            let key = (m.kind as u8, m.name.clone());
            match groups.iter_mut().find(|g| g.0 == key) {
                Some(g) => g.1.push(i),
                None => groups.push((key, vec![i])),
            }
        }
        for (_, idx) in groups.into_iter().filter(|g| g.1.len() > 1) {
            let sigs = members.iter_mut().enumerate().filter(|(i, _)| idx.contains(i)).filter_map(|(_, m)| m.sig.as_mut()).collect();
            mark(sigs);
        }
    });
    fn functions(decls: &mut [Decl]) {
        let mut names: Vec<String> = Vec::new();
        for d in decls.iter_mut() {
            match d {
                Decl::Function(f) if !names.contains(&f.name) => names.push(f.name.clone()),
                Decl::Namespace(n) => functions(&mut n.decls),
                _ => {}
            }
        }
        for name in names {
            let sigs: Vec<&mut Signature> = decls.iter_mut().filter_map(|d| if let Decl::Function(f) = d { (f.name == name).then_some(&mut f.sig) } else { None }).collect();
            if sigs.len() > 1 {
                mark(sigs);
            }
        }
    }
    functions(decls);
}

fn signature_marks(s: &Signature) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for p in &s.type_params {
        if let Some(Type::Ref(r)) = &p.constraint
            && r.kind != "typeParameter"
        {
            out.push(r.name.rsplit('.').next().unwrap_or_default().to_string());
        }
    }
    // The literal options: properties of an object written in a parameter.
    fn literals(t: &Type, out: &mut Vec<String>) {
        match t {
            Type::Object(members) => {
                for m in members.iter().filter(|m| m.kind == MemberKind::Property) {
                    let value = match &m.ty {
                        Some(Type::Literal { kind, value }) => Some((*kind, value.as_str())),
                        _ => m.constant.as_deref().map(|v| (if v == "true" || v == "false" { "boolean" } else { "string" }, v)),
                    };
                    match value {
                        Some(("boolean", "true")) => out.push(m.name.clone()),
                        Some(("string", v)) => out.push(v.to_string()),
                        _ => {}
                    }
                }
            }
            Type::Intersection(parts, _) => parts.iter().for_each(|p| literals(p, out)),
            Type::Nullable(inner, _) | Type::Readonly(inner) => literals(inner, out),
            _ => {}
        }
    }
    for p in &s.params {
        literals(&p.ty, &mut out);
    }
    let mut seen = HashSet::new();
    out.into_iter().map(|w| name_part(&w)).filter(|w| seen.insert(w.clone())).collect()
}

// Names for overloads that share their fixed values. The one with no marks
// of its own (the first, without marks) takes `base`, the others are named
// for theirs, `merge_Dictionary`, else as `family_names_from` names them.
fn family_names(base: &str, sigs: &[&Signature], base_taken: bool, taken: &mut HashSet<String>, o: Options) -> Vec<String> {
    let own = crate::generators::own_marks(&sigs.iter().map(|s| s.marks.clone()).collect::<Vec<_>>());
    let plain = own.iter().position(Vec::is_empty).unwrap_or(0);
    let order: Vec<usize> = std::iter::once(plain).chain((0..sigs.len()).filter(|&i| i != plain)).collect();
    let ordered: Vec<&Signature> = order.iter().map(|&i| sigs[i]).collect();
    let ordered_own: Vec<Vec<String>> = order.iter().map(|&i| own[i].clone()).collect();
    let names = family_names_from(base, &ordered, &ordered_own, base_taken, taken, o);
    let mut out = vec![String::new(); sigs.len()];
    for (k, i) in order.into_iter().enumerate() {
        out[i] = names[k].clone();
    }
    out
}

// Names for overloads that share their fixed values. The first takes `base`;
// `base_taken` says whether it is already the member's own name.
fn family_names_from(base: &str, sigs: &[&Signature], own: &[Vec<String>], base_taken: bool, taken: &mut HashSet<String>, o: Options) -> Vec<String> {
    let sep = o.separator;
    let first = sigs[0];
    let mut out = Vec::new();
    for (n, s) in sigs.iter().enumerate() {
        if n == 0 {
            out.push(if base_taken { base.to_string() } else { unique(base.to_string(), taken) });
            continue;
        }
        if o.numbered_overloads {
            out.push(unique(format!("{base}{sep}{}", n + 1), taken));
            continue;
        }
        if !own[n].is_empty() {
            out.push(unique(format!("{base}{sep}{}", own[n].join(sep)), taken));
            continue;
        }
        let with = |parts: Vec<String>| format!("{base}{sep}with{sep}{}", parts.join(&format!("{sep}and{sep}")));
        let names = |s: &Signature| s.params.iter().map(|p| p.name.clone()).collect::<Vec<_>>();
        // Overloads with the same parameter names: the first is named by what
        // it adds, the others by the types that vary among them.
        let group: Vec<&Signature> = sigs.iter().copied().filter(|t| names(t) == names(s)).collect();
        let varying_at: Vec<usize> = (0..s.params.len()).filter(|&i| group.iter().any(|t| type_key(&t.params[i].ty) != type_key(&s.params[i].ty))).collect();
        let varying: Vec<String> = varying_at.iter().map(|&i| param_mnemonic(s, &s.params[i].ty)).collect();
        let leads = std::ptr::eq(group[0], *s);
        // Parameters the plain overload does not have. When they only extend
        // it (`f(a)`, `f(a, b, c)`), the last one says it: `f_with_c`.
        let mut added: Vec<String> = s.params.iter().filter(|p| !first.params.iter().any(|q| q.name == p.name)).map(|p| p.name.clone()).collect();
        let extends = s.params.len() > first.params.len() && first.params.iter().zip(&s.params).all(|(a, b)| a.name == b.name && type_key(&a.ty) == type_key(&b.ty));
        if extends && added.len() > 1 {
            added = vec![added.pop().unwrap()];
        }
        // Parameters whose type differs from the plain overload's.
        // One only made optional, or no longer, by its name: `marked_with_options`.
        let unwrapped = |t: &Type| match t {
            Type::Nullable(inner, _) => type_key(inner),
            t => type_key(t),
        };
        let retyped: Vec<String> = s
            .params
            .iter()
            .enumerate()
            .filter(|(i, p)| first.params.get(*i).is_none_or(|q| type_key(&q.ty) != type_key(&p.ty)))
            .map(|(i, p)| match first.params.get(i) {
                Some(q) if unwrapped(&q.ty) == unwrapped(&p.ty) => p.name.clone(),
                _ => param_mnemonic(s, &p.ty),
            })
            .collect();
        // Named by the first type that sets it apart, plus what it adds:
        // `addEventListener_with_string_and_options`.
        let first_name = varying_at.first().map(|&i| s.params[i].name.clone());
        let first_varying: Vec<String> = varying.first().cloned().into_iter().chain(added.iter().filter(|a| Some(*a) != first_name.as_ref()).cloned()).collect();
        let candidates = [
            (leads && !added.is_empty()).then(|| with(added.clone())),
            (!leads && !varying.is_empty()).then(|| with(first_varying.clone())),
            (!leads && !varying.is_empty()).then(|| with(varying.clone())),
            (!added.is_empty()).then(|| with(added.clone())),
            retyped.last().map(|t| with(vec![t.clone()])),
            (!retyped.is_empty()).then(|| with(retyped.clone())),
            // Too long to say all it adds (`compressedTexImage2D(…, srcData,
            // srcOffset, srcLengthOverride)`): the first.
            added.first().map(|a| with(vec![a.clone()])),
            varying.first().map(|v| with(vec![v.clone()])),
        ];
        let free: Vec<String> = candidates.into_iter().flatten().filter(|c| !taken.contains(c)).collect();
        let name = free.iter().find(|c| c.len() <= NAME_MAX).or(free.first()).cloned().unwrap_or_else(|| format!("{base}{sep}{}", n + 1));
        out.push(unique(name, taken));
    }
    out
}

// A value as part of a name: `2d`, `image_png`.
pub(super) fn name_part(value: &str) -> String {
    // A URL is known by its last part: `http://www.w3.org/2000/svg` -> `svg`.
    let value = if value.contains("://") { value.trim_end_matches('/').rsplit('/').next().unwrap_or(value) } else { value };
    let s: String = value.chars().map(|c| if c.is_alphanumeric() { c } else { '_' }).collect();
    if s.is_empty() { "empty".into() } else { s }
}

// A parameter's type in a name; a key of a map is named by its map.
fn param_mnemonic(s: &Signature, t: &Type) -> String {
    if let Type::Ref(r) = t
        && r.kind == "typeParameter"
        && let Some(map) = s.type_params.iter().find(|p| p.sym.is_some() && p.sym == r.sym).and_then(keyed_map)
    {
        return map.name.rsplit('.').next().unwrap_or_default().to_string();
    }
    mnemonic(t)
}

pub(super) fn mnemonic(t: &Type) -> String {
    describe(t).unwrap_or_else(|| "x".into())
}

// A type in a few words, if it has them: not an inline object, nor a
// type-level expression.
pub(super) fn describe(t: &Type) -> Option<String> {
    Some(match t {
        Type::Primitive(n) => n.clone(),
        // Written qualified: `Tokens.HTML` is `Tokens_HTML`.
        Type::Ref(r) => r.name.replace('.', "_"),
        Type::Array(inner) => format!("{}_array", describe(inner)?),
        Type::Nullable(inner, _) => format!("opt_{}", describe(inner)?),
        Type::Readonly(inner) | Type::Tested(inner, _) => describe(inner)?,
        Type::Union(parts) => parts.iter().map(describe).collect::<Option<Vec<_>>>()?.join("_or_"),
        Type::Literal { value, .. } => name_part(value),
        Type::Function(_) => "fn".into(),
        Type::Constructor(_) => "ctor".into(),
        Type::Tuple(_) => "tuple".into(),
        Type::Erased(_) => "any".into(),
        Type::This => "this".into(),
        Type::Object(_) | Type::Intersection(..) | Type::Op(..) => return None,
    })
}

pub(super) fn rename_members(members: &mut [Member], o: Options) {
    let mut taken: HashSet<String> = members.iter().map(|m| m.name.clone()).collect();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, m) in members.iter().enumerate() {
        // A call or construct signature has no name to tell overloads apart by.
        if !matches!(m.kind, MemberKind::Method | MemberKind::Construct | MemberKind::Call) || m.name.is_empty() {
            continue;
        }
        let key = format!("{:?}{}", m.kind as u8, m.name);
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(i),
            None => groups.push((key, vec![i])),
        }
    }
    for (_, idx) in groups.into_iter().filter(|g| g.1.len() > 1) {
        let base = members[idx[0]].name.clone();
        let sigs: Vec<&Signature> = idx.iter().map(|i| members[*i].sig.as_ref().unwrap()).collect();
        let names = overload_names(&base, &sigs, &mut taken, o);
        for (i, name) in idx.into_iter().zip(names) {
            if name != base {
                members[i].js_name = Some(base.clone());
                members[i].name = name;
            }
        }
    }
}

// Gives each renamed overload's (old, new) name.
pub(super) fn rename_functions(decls: &mut [Decl], o: Options) -> Vec<(String, String)> {
    let mut renamed = Vec::new();
    let mut taken: HashSet<String> = HashSet::new();
    let mut groups: Vec<(String, Vec<usize>)> = Vec::new();
    for (i, d) in decls.iter_mut().enumerate() {
        match d {
            Decl::Function(f) => {
                taken.insert(f.name.clone());
                match groups.iter_mut().find(|g| g.0 == f.name) {
                    Some(g) => g.1.push(i),
                    None => groups.push((f.name.clone(), vec![i])),
                }
            }
            Decl::Namespace(n) => renamed.extend(rename_functions(&mut n.decls, o)),
            _ => {}
        }
    }
    for (base, idx) in groups.into_iter().filter(|g| g.1.len() > 1) {
        let sigs: Vec<&Signature> = idx.iter().map(|i| if let Decl::Function(f) = &decls[*i] { &f.sig } else { unreachable!() }).collect();
        let names = overload_names(&base, &sigs, &mut taken, o);
        for (i, name) in idx.into_iter().zip(names) {
            if let Decl::Function(f) = &mut decls[i]
                && name != base
            {
                // What it is exported as (`default`), else the name it shares.
                f.js_name = Some(f.js_name.take().unwrap_or_else(|| base.clone()));
                renamed.push((base.clone(), name.clone()));
                f.name = name;
            }
        }
    }
    renamed
}
