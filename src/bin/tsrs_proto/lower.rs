// Lowering passes. Each removes one TypeScript construct from the IR, for
// targets that cannot express it. Passes that need semantics ask the checker
// through `Front`; the rest are plain IR rewrites.

use std::collections::{HashMap, HashSet};

use tsrs_ast::Symbol;
use tsrs_core::P;

use crate::front::Front;
use crate::ir::*;
use crate::xml::{member_key, signature_key, type_key};

// What to lower. Built from a config file; see config.rs.
#[derive(Clone, Copy)]
pub struct Options {
    pub classes: bool,
    pub typeops: bool,
    pub intersections: bool,
    pub inheritance: bool,
    pub generics: bool,
    pub aliases: bool,
    pub nullable: bool,
    pub literals: bool,
    pub anonymous: bool,
    pub optionals: bool,
    pub param_unions: bool,
    pub unions: bool,
    pub overloads: bool,
    pub keyed: bool,
    pub redeclared: bool,
    // Keep keyed signatures generic, with their map (`keyed_params = "map"`).
    pub map_keyed: bool,
    // Unions become named declarations (`unions = "name"`).
    pub name_unions: bool,
    // Most keys a keyed signature is expanded over.
    pub keyed_limit: usize,
    // Most overloads one signature may expand into through union parameters.
    pub union_param_limit: usize,
    // Largest merged intersection that is written out inline.
    pub intersection_limit: usize,
    // Joins the parts of generated names.
    pub separator: &'static str,
    // Overload names: `f_2`, `f_3` instead of `f_with_x`.
    pub numbered_overloads: bool,
}

impl Default for Options {
    fn default() -> Self {
        Options {
            classes: false, typeops: false, intersections: false, inheritance: false, generics: false, aliases: false, nullable: false,
            literals: false, anonymous: false, optionals: false, param_unions: false, unions: false, overloads: false, keyed: false, redeclared: false, map_keyed: false, name_unions: false, keyed_limit: 256,
            union_param_limit: 16, intersection_limit: 64, separator: "_", numbered_overloads: false,
        }
    }
}

impl Options {
    // Whether the construct with this config key is lowered.
    pub fn slot(&mut self, key: &str) -> Option<&mut bool> {
        Some(match key {
            "classes" => &mut self.classes,
            "type_ops" => &mut self.typeops,
            "intersections" => &mut self.intersections,
            "inheritance" => &mut self.inheritance,
            "generics" => &mut self.generics,
            "aliases" => &mut self.aliases,
            "nullable" => &mut self.nullable,
            "literals" => &mut self.literals,
            "anonymous" => &mut self.anonymous,
            "optional_params" => &mut self.optionals,
            "union_params" => &mut self.param_unions,
            "unions" => &mut self.unions,
            "overloads" => &mut self.overloads,
            "keyed_params" => &mut self.keyed,
            "redeclarations" => &mut self.redeclared,
            _ => return None,
        })
    }
}

pub fn lower(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    front.merge_intersections = o.intersections;
    front.intersection_limit = o.intersection_limit;
    front.expand_keyed = o.keyed;
    front.map_keyed = o.map_keyed;
    front.keyed_limit = o.keyed_limit;
    // An inherited member serves every subtype, where a target has subtypes.
    if o.redeclared && !o.inheritance {
        let dropped = drop_redeclared(decls);
        *front.stats.entry("dropped: member redeclared as inherited".into()).or_default() += dropped;
    }
    if o.keyed {
        expand_keyed(front, decls, o);
    }
    if o.classes {
        pair_classes(decls);
    }
    if o.inheritance {
        flatten(front, decls);
    }
    if o.keyed {
        drop_unreachable_overloads(decls);
    }
    if o.typeops || o.intersections {
        evaluate(front, decls, o);
        flatten_unions(decls);
        inline_object_bases(front, decls);
    }
    if o.aliases {
        expand_aliases(front, decls);
    }
    if o.generics {
        erase_generics(decls, o.map_keyed);
    }
    if o.nullable {
        walk(decls, &mut |t, _| {
            nullable(t);
            true
        });
    }
    if o.literals {
        let widened = literals(decls, o.separator);
        *front.stats.entry("widened: literal -> primitive".into()).or_default() += widened;
    }
    if o.anonymous {
        hoist(decls, o.separator);
    }
    if o.optionals || o.param_unions {
        each_member_list(decls, &mut |members| expand_members(members, o));
        expand_functions(decls, o);
    }
    if o.unions {
        walk(decls, &mut |t, _| {
            match t {
                Type::Union(parts) => match front.common_supertype(parts) {
                    Some(sup) => {
                        front.count("widened: union -> common supertype");
                        *t = sup;
                    }
                    None => {
                        front.count("erased: union");
                        *t = Type::Erased("union");
                    }
                },
                Type::Intersection(..) => {
                    front.count("erased: intersection");
                    *t = Type::Erased("intersection");
                }
                _ => {}
            }
            true
        });
    }
    if o.name_unions {
        // Merging arrays can bring a union alias next to its own members: a few rounds settle it.
        for _ in 0..3 {
            splice_union_aliases(decls);
            flatten_unions(decls);
            merge_array_members(decls);
        }
        absorb_enums(decls);
        name_unions(decls, o.separator);
    }
    if o.overloads {
        each_member_list(decls, &mut |members| rename_members(members, o));
        rename_functions(decls, o);
    }
    if o.classes {
        mark_class_refs(decls);
    }
    annotate_unions(front, decls);
}

// A method a type redeclares as its ancestor declares it, but for the types
// of `this`: `HTMLDivElement.addEventListener` is `HTMLElement.addEventListener`.
fn drop_redeclared(decls: &mut [Decl]) -> usize {
    let key = |m: &Member| -> Option<String> {
        let mut sig = m.sig.clone()?;
        sig.this = None;
        let mut path = Vec::new();
        walk_signature(&mut sig, &mut path, &mut |t, _| {
            if let Type::Function(s) | Type::Constructor(s) = t {
                s.this = None;
            }
            true
        });
        let params: Vec<String> = sig.params.iter().map(|p| format!("{}{}{}", p.optional, p.rest, crate::xml::type_key(&p.ty))).collect();
        let tps: Vec<String> = sig.type_params.iter().map(|t| format!("{}:{}", t.name, t.constraint.as_ref().map(crate::xml::type_key).unwrap_or_default())).collect();
        Some(format!("{}({})<{}>{}", m.name, params.join(","), tps.join(","), sig.ret.as_ref().map(crate::xml::type_key).unwrap_or_default()))
    };
    let mut own: HashMap<String, HashSet<String>> = HashMap::new();
    let mut bases: HashMap<String, Vec<String>> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Interface(i) = d {
            own.entry(i.name.clone()).or_default().extend(i.members.iter().filter(|m| m.kind == MemberKind::Method).filter_map(key));
            bases.entry(i.name.clone()).or_default().extend(i.extends.iter().filter_map(|t| if let Type::Ref(r) = t { Some(r.name.clone()) } else { None }));
        }
    }
    let inherited = |name: &str| -> HashSet<String> {
        let mut out = HashSet::new();
        let mut todo: Vec<String> = bases.get(name).cloned().unwrap_or_default();
        let mut seen = HashSet::new();
        while let Some(b) = todo.pop() {
            if seen.insert(b.clone()) {
                out.extend(own.get(&b).cloned().unwrap_or_default());
                todo.extend(bases.get(&b).cloned().unwrap_or_default());
            }
        }
        out
    };
    let mut dropped = 0;
    for d in decls.iter_mut() {
        if let Decl::Interface(i) = d {
            let from_bases = inherited(&i.name);
            if from_bases.is_empty() {
                continue;
            }
            // Only whole overload sets: a set split between a type and its
            // ancestor is resolved differently, or hidden, in some targets.
            let redeclared = |m: &Member| m.kind == MemberKind::Method && key(m).is_some_and(|k| from_bases.contains(&k));
            let names: HashSet<String> = i.members.iter().filter(|m| m.kind == MemberKind::Method).map(|m| m.name.clone()).collect();
            let whole: HashSet<String> = names.into_iter().filter(|n| i.members.iter().filter(|m| m.kind == MemberKind::Method && m.name == *n).all(redeclared)).collect();
            let before = i.members.len();
            i.members.retain(|m| !(m.kind == MemberKind::Method && whole.contains(&m.name)));
            dropped += before - i.members.len();
        }
    }
    dropped
}

// ---- unions ----

// Each union that is not already a declaration's body becomes one: named by
// its members when there are few (`boolean_or_AddEventListenerOptions`),
// else by where it appears.
// `A[] | B[]` is `(A | B)[]`: a value of either is an array, and nothing at
// runtime says of which.
fn merge_array_members(decls: &mut [Decl]) {
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
fn absorb_enums(decls: &mut [Decl]) {
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

fn collect_declared_names(decls: &[Decl], prefix: &str, sep: &str, out: &mut HashSet<String>) {
    for d in decls {
        if let Some(name) = decl_name(d) {
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
        // A method's function (`Transformer_isTransforming`) is not a type's
        // name, nor is an interface's constructor (`Point_create`).
        if let Decl::Interface(i) = d {
            let statics = i.class.iter().flat_map(|c| &c.statics);
            let methods = i.members.iter().chain(statics).filter(|m| matches!(m.kind, MemberKind::Method)).map(|m| m.name.as_str());
            for m in methods.chain(["create"]) {
                out.insert(format!("{}{sep}{m}", i.name));
                out.insert(format!("{}_{m}", i.name));
            }
        }
    }
}

fn name_unions(decls: &mut Vec<Decl>, sep: &str) {
    let mut names: HashSet<String> = HashSet::new();
    collect_declared_names(decls, "", sep, &mut names);
    // A union written inline that a declared alias already names uses that name.
    let mut by_key: HashMap<String, String> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Alias(a) = d
            && a.type_params.is_empty()
            && matches!(a.ty, Type::Union(_))
        {
            by_key.entry(type_key(&a.ty)).or_insert_with(|| a.name.clone());
        }
    }
    // A named union may itself hold unions, inside its arrays: until none is left.
    let mut free_of: HashMap<String, Vec<Ref>> = HashMap::new();
    loop {
        let before = decls.len();
        name_unions_once(decls, sep, &mut names, &mut by_key, &mut free_of);
        if decls.len() == before {
            break;
        }
    }
}

fn name_unions_once(decls: &mut Vec<Decl>, sep: &str, names: &mut HashSet<String>, by_key: &mut HashMap<String, String>, free_of: &mut HashMap<String, Vec<Ref>>) {
    let mut bodies: HashSet<*const Type> = HashSet::new();
    collect_bodies(decls, &mut bodies);
    let mut added = Vec::new();
    walk(decls, &mut |t, path| {
        if !matches!(t, Type::Union(_)) || bodies.contains(&(t as *const Type)) {
            return true;
        }
        let Type::Union(parts) = &*t else { unreachable!() };
        let name = by_key
            .entry(type_key(t))
            .or_insert_with(|| {
                let base = if parts.len() <= 3 { parts.iter().map(mnemonic).collect::<Vec<_>>().join(&format!("{sep}or{sep}")) } else { path.join(sep) };
                let name = unique(base, names);
                // Type parameters the union mentions become its own.
                let free = free_type_params(t);
                let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None }).collect();
                free_of.insert(name.clone(), free);
                added.push(Decl::Alias(Alias { name: name.clone(), sym: None, type_params, ty: t.clone(), computed: false, synthetic: true }));
                name
            })
            .clone();
        let args = free_of.get(&name).cloned().unwrap_or_default();
        *t = Type::synthetic_ref(name, "typeAlias", args.into_iter().map(Type::Ref).collect());
        false
    });
    decls.extend(added);
}

// Says how each member of each union is recognised at runtime, so a
// generator can tell which one a value is: `typeof:string`, `array`,
// `instanceof:HTMLElement`, `has:once`, or `tag:circle` when one property
// (`discriminant`) tells them all apart.
fn annotate_unions(front: &mut Front, decls: &mut [Decl]) {
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
    let mut enum_kinds = HashMap::new();
    enums(decls, &mut enum_kinds);
    let enum_test = |p: &Type| match p {
        Type::Ref(r) if r.kind == "enum" => enum_kinds.get(&r.name).map(|k| Test::TypeOf(k)),
        _ => None,
    };
    walk(decls, &mut |t, _| {
        let Type::Union(parts) = t else { return true };
        let shapes: Vec<Option<Vec<(String, bool, Option<String>)>>> = parts.iter().map(|p| front.object_shape(p)).collect();
        let discriminant = discriminant_of(&shapes);
        let tests: Vec<Option<Test>> = parts
            .iter()
            .enumerate()
            .map(|(i, p)| match &discriminant {
                Some(d) => shapes[i].as_ref().and_then(|s| s.iter().find(|f| f.0 == *d)).and_then(|f| f.2.clone()).map(Test::Tag),
                None => enum_test(p).or_else(|| front.runtime_test(p)).or_else(|| unique_property(&shapes, i).map(Test::Has)),
            })
            .collect();
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

// A property every member has, required, with a different literal value in each.
fn discriminant_of(shapes: &[Option<Vec<(String, bool, Option<String>)>>]) -> Option<String> {
    let shapes: Vec<&Vec<(String, bool, Option<String>)>> = shapes.iter().map(|s| s.as_ref()).collect::<Option<_>>()?;
    let first = shapes.first()?;
    first.iter().map(|f| &f.0).find(|name| {
        let mut values = HashSet::new();
        shapes.iter().all(|s| s.iter().any(|f| f.0 == **name && !f.1 && f.2.as_ref().is_some_and(|v| values.insert(v.clone()))))
    }).cloned()
}

// A required property of member `i` that no other member has.
fn unique_property(shapes: &[Option<Vec<(String, bool, Option<String>)>>], i: usize) -> Option<String> {
    let own = shapes[i].as_ref()?;
    own.iter()
        .filter(|f| !f.1)
        .find(|f| shapes.iter().enumerate().all(|(j, s)| j == i || s.as_ref().is_none_or(|s| !s.iter().any(|g| g.0 == f.0))))
        .map(|f| f.0.clone())
}

// ---- traversal ----

type TypeFn<'a> = dyn FnMut(&mut Type, &[String]) -> bool + 'a;

// Visits every type position, pre-order, with the naming path to it.
// The callback may replace the type; returning false skips its children.
fn walk(decls: &mut [Decl], f: &mut TypeFn) {
    let mut path = Vec::new();
    walk_decls(decls, &mut path, f);
}

fn walk_decls(decls: &mut [Decl], path: &mut Vec<String>, f: &mut TypeFn) {
    for d in decls {
        match d {
            Decl::Interface(i) => {
                path.push(i.name.clone());
                walk_type_params(&mut i.type_params, path, f);
                for e in &mut i.extends {
                    walk_type(e, path, f);
                }
                if let Some(c) = &mut i.class {
                    walk_members(&mut c.constructors, path, f);
                    walk_members(&mut c.statics, path, f);
                }
                walk_members(&mut i.members, path, f);
                path.pop();
            }
            Decl::Alias(a) => {
                path.push(a.name.clone());
                walk_type_params(&mut a.type_params, path, f);
                walk_type(&mut a.ty, path, f);
                path.pop();
            }
            Decl::Function(func) => {
                path.push(func.name.clone());
                walk_signature(&mut func.sig, path, f);
                path.pop();
            }
            Decl::Var(v) => {
                path.push(v.name.clone());
                walk_type(&mut v.ty, path, f);
                path.pop();
            }
            Decl::Enum(_) => {}
            Decl::Namespace(n) => {
                path.push(n.name.clone());
                walk_decls(&mut n.decls, path, f);
                path.pop();
            }
        }
    }
}

fn walk_members(members: &mut [Member], path: &mut Vec<String>, f: &mut TypeFn) {
    for m in members {
        let seg = match m.kind {
            MemberKind::Call => "call".to_string(),
            MemberKind::Construct => "new".to_string(),
            MemberKind::Index => "index".to_string(),
            // A fixed argument is part of what the member is: `addEventListener_click`.
            _ => {
                let mut seg = m.js_name.clone().unwrap_or_else(|| m.name.clone());
                for c in m.sig.iter().flat_map(|s| s.params.iter().filter_map(|p| p.constant.as_deref())) {
                    seg = format!("{seg}_{}", name_part(c));
                }
                seg
            }
        };
        path.push(seg);
        if let Some(t) = &mut m.ty {
            walk_type(t, path, f);
        }
        if let Some(s) = &mut m.sig {
            walk_signature(s, path, f);
        }
        path.pop();
    }
}

fn walk_type_params(params: &mut [TypeParam], path: &mut Vec<String>, f: &mut TypeFn) {
    for p in params {
        path.push(p.name.clone());
        for t in p.constraint.iter_mut().chain(p.default.iter_mut()) {
            walk_type(t, path, f);
        }
        path.pop();
    }
}

fn walk_signature(s: &mut Signature, path: &mut Vec<String>, f: &mut TypeFn) {
    walk_type_params(&mut s.type_params, path, f);
    if let Some(t) = &mut s.this {
        walk_type(t, path, f);
    }
    for p in &mut s.params {
        path.push(p.name.clone());
        walk_type(&mut p.ty, path, f);
        path.pop();
    }
    if let Some(r) = &mut s.ret {
        path.push("result".into());
        walk_type(r, path, f);
        path.pop();
    }
}

fn walk_type(t: &mut Type, path: &mut Vec<String>, f: &mut TypeFn) {
    if !f(t, path) {
        return;
    }
    match t {
        Type::Primitive(_) | Type::Erased(_) | Type::Literal { .. } | Type::This => {}
        Type::Ref(r) => r.args.iter_mut().for_each(|a| walk_type(a, path, f)),
        Type::Union(ts) | Type::Intersection(ts, _) => ts.iter_mut().for_each(|a| walk_type(a, path, f)),
        Type::Array(inner) | Type::Readonly(inner) | Type::Nullable(inner, _) => walk_type(inner, path, f),
        Type::Tuple(els) => els.iter_mut().for_each(|e| walk_type(&mut e.ty, path, f)),
        Type::Function(sig) | Type::Constructor(sig) => walk_signature(sig, path, f),
        Type::Object(members) => walk_members(members, path, f),
        Type::Tested(inner, _) => walk_type(inner, path, f),
        Type::Op(op, _) => match &mut **op {
            Op::KeyOf(a) | Op::Unique(a) => walk_type(a, path, f),
            Op::IndexedAccess(a, b) => {
                walk_type(a, path, f);
                walk_type(b, path, f);
            }
            Op::Conditional { check, extends, yes, no } => {
                for a in [check, extends, yes, no] {
                    walk_type(a, path, f);
                }
            }
            Op::Mapped { param, as_, value, .. } => {
                walk_type_params(std::slice::from_mut(param), path, f);
                for a in as_.iter_mut().chain(value.iter_mut()) {
                    walk_type(a, path, f);
                }
            }
            Op::Template(parts) => {
                for p in parts {
                    if let TemplatePart::Type(a) = p {
                        walk_type(a, path, f);
                    }
                }
            }
            Op::Infer(p) => walk_type_params(std::slice::from_mut(p), path, f),
            Op::Predicate { ty, .. } => {
                if let Some(a) = ty {
                    walk_type(a, path, f);
                }
            }
            Op::TypeOf(_) | Op::Raw { .. } => {}
        },
    }
}

// Visits every list of members: interfaces, class statics and constructors, object types.
fn each_member_list(decls: &mut [Decl], f: &mut dyn FnMut(&mut Vec<Member>)) {
    for d in decls.iter_mut() {
        match d {
            Decl::Interface(i) => {
                f(&mut i.members);
                if let Some(c) = &mut i.class {
                    f(&mut c.statics);
                    f(&mut c.constructors);
                }
            }
            Decl::Namespace(n) => each_member_list(&mut n.decls, f),
            _ => {}
        }
    }
    walk(decls, &mut |t, _| {
        if let Type::Object(members) = t {
            f(members);
        }
        true
    });
}

// ---- names ----

// Gives generated declarations the names a config chose: `[rename]`. Only
// generated ones: a declared name is the library's, and JavaScript's. A name
// this target did not generate is skipped and returned: a library's table
// serves every target, and they generate different names.
pub fn rename(decls: &mut [Decl], renames: &[(String, String)]) -> Result<Vec<String>, String> {
    fn names(decls: &[Decl], out: &mut Vec<(String, bool)>) {
        for d in decls {
            match d {
                Decl::Interface(i) => out.push((i.name.clone(), i.synthetic)),
                Decl::Alias(a) => out.push((a.name.clone(), a.synthetic)),
                Decl::Enum(e) => out.push((e.name.clone(), e.synthetic)),
                Decl::Function(f) => out.push((f.name.clone(), false)),
                Decl::Var(v) => out.push((v.name.clone(), false)),
                Decl::Namespace(n) => {
                    out.push((n.name.clone(), false));
                    names(&n.decls, out);
                }
            }
        }
    }
    if renames.is_empty() {
        return Ok(Vec::new());
    }
    let mut declared = Vec::new();
    names(decls, &mut declared);
    let mut unused = Vec::new();
    for (i, (old, new)) in renames.iter().enumerate() {
        match declared.iter().find(|d| d.0 == *old) {
            None => unused.push(old.clone()),
            Some((_, false)) => return Err(format!("rename.{old}: only generated names can be renamed, and the library declares this one")),
            Some(_) => {}
        }
        if new.is_empty() || !new.chars().all(|c| c.is_alphanumeric() || c == '_') || new.chars().next().is_some_and(|c| c.is_numeric()) {
            return Err(format!("rename.{old} = \"{new}\": not a name"));
        }
        if declared.iter().any(|d| d.0 == *new) || renames[..i].iter().any(|r| r.1 == *new) {
            return Err(format!("rename.{old} = \"{new}\": that name is taken"));
        }
    }
    rename_everywhere(decls, renames);
    Ok(unused)
}

// Renames declarations, and what refers to them: references, inherited
// members, and the classes union members are tested to be instances of.
pub fn rename_everywhere(decls: &mut [Decl], renames: &[(String, String)]) {
    fn apply(decls: &mut [Decl], to: &dyn Fn(&str) -> Option<String>) {
        let set = |name: &mut String| {
            if let Some(new) = to(name) {
                *name = new;
            }
        };
        for d in decls {
            match d {
                Decl::Interface(i) => {
                    set(&mut i.name);
                    i.ancestors.iter_mut().for_each(set);
                }
                Decl::Alias(a) => set(&mut a.name),
                Decl::Enum(e) => set(&mut e.name),
                Decl::Namespace(n) => apply(&mut n.decls, to),
                Decl::Function(f) => set(&mut f.name),
                Decl::Var(v) => set(&mut v.name),
            }
        }
    }
    if renames.is_empty() {
        return;
    }
    let to = |name: &str| renames.iter().find(|r| r.0 == name).map(|r| r.1.clone());
    apply(decls, &to);
    each_member_list(decls, &mut |members| {
        for m in members {
            if let Some(new) = m.from.as_deref().and_then(to) {
                m.from = Some(new);
            }
        }
    });
    walk(decls, &mut |t, _| {
        if let Type::Ref(r) = t
            && r.kind != "typeParameter"
            && let Some(new) = to(&r.name)
        {
            r.qualified = None;
            r.name = new;
        }
        if let Type::Tested(_, Test::InstanceOf(c)) = t
            && let Some(new) = to(c)
        {
            *c = new;
        }
        true
    });
}

// ---- classes ----

// `interface X` + `var X: { prototype: X; new(): X; statics }` is a class.
fn pair_classes(decls: &mut Vec<Decl>) {
    // The class, and what its constructor is exported as.
    let mut classes: HashMap<String, (Class, Option<String>)> = HashMap::new();
    let interfaces: HashSet<String> = decls.iter().filter_map(|d| if let Decl::Interface(i) = d { Some(i.name.clone()) } else { None }).collect();
    decls.retain_mut(|d| {
        let Decl::Var(v) = d else { return true };
        let Type::Object(members) = &mut v.ty else { return true };
        let is_proto = |m: &Member| m.name == "prototype" && matches!(&m.ty, Some(Type::Ref(r)) if r.name == v.name);
        if !interfaces.contains(&v.name) || !members.iter().any(is_proto) {
            return true;
        }
        let mut class = Class { constructors: Vec::new(), statics: Vec::new() };
        for mut m in members.drain(..).filter(|m| !is_proto(m)) {
            if m.kind == MemberKind::Construct {
                // The constructor's result and type parameters are the class's own.
                if let Some(s) = &mut m.sig {
                    s.ret = None;
                    s.type_params.clear();
                }
                m.name = "new".into();
                class.constructors.push(m);
            } else {
                class.statics.push(m);
            }
        }
        classes.insert(v.name.clone(), (class, v.js_name.clone()));
        false
    });
    for d in decls.iter_mut() {
        match d {
            // A declared class keeps its own constructors and statics.
            Decl::Interface(i) => {
                if let Some((class, js_name)) = classes.remove(&i.name) {
                    i.class = Some(class);
                    i.js_name = js_name.or(i.js_name.take());
                }
            }
            Decl::Namespace(n) => pair_classes(&mut n.decls),
            _ => {}
        }
    }
}

// References to a paired interface are references to a class. Runs last:
// later passes bring in references the checker made.
fn mark_class_refs(decls: &mut [Decl]) {
    fn collect(decls: &[Decl], out: &mut HashSet<P<Symbol>>) {
        for d in decls {
            match d {
                Decl::Interface(i) if i.class.is_some() => out.extend(i.sym),
                Decl::Namespace(n) => collect(&n.decls, out),
                _ => {}
            }
        }
    }
    let mut classes = HashSet::new();
    collect(decls, &mut classes);
    walk(decls, &mut |t, _| {
        if let Type::Ref(r) = t
            && r.kind == "interface"
            && r.sym.is_some_and(|s| classes.contains(&s))
        {
            r.kind = "class";
        }
        true
    });
}

// ---- inheritance ----

fn flatten(front: &mut Front, decls: &mut [Decl]) {
    for d in decls {
        match d {
            Decl::Interface(i) => {
                if let Some(sym) = i.sym {
                    let (members, ancestors) = front.flattened(sym);
                    i.members = members;
                    i.ancestors = ancestors;
                    i.extends.clear();
                }
            }
            Decl::Namespace(n) => flatten(front, &mut n.decls),
            _ => {}
        }
    }
}

// ---- type-level expressions ----

fn evaluate(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    if o.typeops {
        drop_type_functions(decls);
    }
    // An alias body is evaluated without folding back into the alias itself.
    for_each_alias(decls, &mut |a| {
        let node = match &a.ty {
            Type::Op(_, node) if o.typeops => *node,
            // Not a reference by name to an alias the output declares, which is
            // evaluated where it is declared: `type ShapeConfig = Shape.ShapeConfig`.
            Type::Ref(r) if o.typeops && r.computed && !(r.args.is_empty() && r.file.is_none()) => r.node,
            Type::Intersection(_, node) if o.intersections => *node,
            _ => None,
        };
        if let (Some(node), Some(sym)) = (node, a.sym) {
            a.ty = front.eval_alias_body(node, sym);
        }
    });
    // `keyed_params = "map"`: `K extends keyof M` and `M[K]` stay as written.
    let keyed = if o.map_keyed { keyed_syms(decls) } else { HashSet::new() };
    let mut kept: HashSet<*const Type> = HashSet::new();
    each_signature(decls, &mut |s| {
        for p in s.type_params.iter().filter(|p| keyed_map(p).is_some()) {
            kept.insert(p.constraint.as_ref().unwrap() as *const Type);
        }
    });
    walk(decls, &mut |t, _| {
        if kept.contains(&(t as *const Type)) {
            return false;
        }
        if let Type::Op(op, _) = t
            && let Op::IndexedAccess(Type::Ref(m), Type::Ref(k)) = &**op
            && m.kind != "typeParameter"
            && k.sym.is_some_and(|s| keyed.contains(&s))
        {
            return false;
        }
        let replacement = match t {
            Type::Op(op, node) if o.typeops => Some(match (&**op, node) {
                (Op::Predicate { asserts, .. }, _) => Type::prim(if *asserts { "void" } else { "boolean" }),
                (Op::Unique(_), _) => Type::prim("symbol"),
                (Op::Raw { .. }, _) => {
                    front.count("erased: unsupported syntax");
                    Type::Erased("unsupported syntax")
                }
                (_, Some(node)) => front.eval(*node),
                (_, None) => Type::Erased("type-level expression"),
            }),
            Type::Ref(r) if o.typeops && r.computed => Some(match r.node {
                Some(node) => front.eval(node),
                None => Type::Erased("type function"),
            }),
            Type::Intersection(_, Some(node)) if o.intersections => Some(front.eval(*node)),
            Type::Readonly(inner) if o.typeops => Some((**inner).clone()),
            _ => None,
        };
        match replacement {
            Some(r) => {
                *t = r;
                // Checker results are already lowered, except for a stripped `readonly`.
                matches!(t, Type::Array(_) | Type::Tuple(_))
            }
            None => true,
        }
    });
}

fn for_each_alias(decls: &mut [Decl], f: &mut dyn FnMut(&mut Alias)) {
    for d in decls {
        match d {
            Decl::Alias(a) => f(a),
            Decl::Namespace(n) => for_each_alias(&mut n.decls, f),
            _ => {}
        }
    }
}

// Generic computed aliases (`Partial<T>`) are functions over types, not types.
fn drop_type_functions(decls: &mut Vec<Decl>) {
    decls.retain_mut(|d| match d {
        Decl::Alias(a) => !a.computed,
        Decl::Namespace(n) => {
            drop_type_functions(&mut n.decls);
            true
        }
        _ => true,
    });
}

// ---- keyed signatures ----

// `on<K extends keyof Map>(type: K, f: (ev: Map[K]) => void)`, written in the
// source: one signature per key, with the key a fixed argument. Members the
// checker builds are expanded as they are built (`Front::keyed_variants`).
fn expand_keyed(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    let mut expanded = 0;
    each_member_list(decls, &mut |members| {
        let mut out = Vec::new();
        for m in members.drain(..) {
            match m.sig.as_ref().and_then(|s| keyed_variants(front, s, o.keyed_limit)) {
                Some(variants) => {
                    expanded += 1;
                    out.extend(variants.into_iter().map(|s| Member { sig: Some(s), ..m.clone() }));
                }
                None => out.push(m),
            }
        }
        *members = out;
    });
    expand_keyed_functions(front, decls, o.keyed_limit, &mut expanded);
    *front.stats.entry("expanded: keyed signature".into()).or_default() += expanded;
}

fn expand_keyed_functions(front: &mut Front, decls: &mut Vec<Decl>, limit: usize, expanded: &mut usize) {
    let mut out = Vec::new();
    for d in decls.drain(..) {
        match d {
            Decl::Function(f) => match keyed_variants(front, &f.sig, limit) {
                Some(variants) => {
                    *expanded += 1;
                    out.extend(variants.into_iter().map(|sig| Decl::Function(Function { name: f.name.clone(), js_name: f.js_name.clone(), sig })));
                }
                None => out.push(Decl::Function(f)),
            },
            Decl::Namespace(mut n) => {
                expand_keyed_functions(front, &mut n.decls, limit, expanded);
                out.push(Decl::Namespace(n));
            }
            d => out.push(d),
        }
    }
    *decls = out;
}

// TypeScript picks the first overload that matches a call, so an overload
// whose parameters repeat an earlier one's can never be chosen. Expansion
// makes these: `"a"` is a key of both the HTML and the SVG tag maps.
fn drop_unreachable_overloads(decls: &mut Vec<Decl>) {
    let call_key = |name: &str, s: &Signature| {
        let params: Vec<String> = s.params.iter().map(|p| format!("{:?}{}{}{}", p.constant, p.optional, p.rest, type_key(&p.ty))).collect();
        format!("{name}({})", params.join(","))
    };
    each_member_list(decls, &mut |members| {
        let mut seen = HashSet::new();
        members.retain(|m| m.kind != MemberKind::Method || m.sig.as_ref().is_none_or(|s| seen.insert(call_key(&m.name, s))));
    });
    fn functions(decls: &mut Vec<Decl>, key: &dyn Fn(&str, &Signature) -> String) {
        let mut seen = HashSet::new();
        decls.retain_mut(|d| match d {
            Decl::Function(f) => seen.insert(key(&f.name, &f.sig)),
            Decl::Namespace(n) => {
                functions(&mut n.decls, key);
                true
            }
            _ => true,
        });
    }
    functions(decls, &call_key);
}

fn keyed_variants(front: &mut Front, sig: &Signature, limit: usize) -> Option<Vec<Signature>> {
    for (i, tp) in sig.type_params.iter().enumerate() {
        let Some(sym) = tp.sym else { continue };
        let is_tp = |t: &Type| matches!(t, Type::Ref(r) if r.kind == "typeParameter" && r.sym == Some(sym));
        if !sig.params.iter().any(|p| is_tp(&p.ty)) {
            continue;
        }
        let keys: Vec<String> = match &tp.constraint {
            // `keyof Map`, for a map that is not generic.
            Some(Type::Op(op, _)) => match &**op {
                Op::KeyOf(Type::Ref(m)) if m.args.is_empty() && m.kind != "typeParameter" => match m.sym {
                    Some(s) => front.keys_of(s),
                    None => continue,
                },
                _ => continue,
            },
            Some(t) => match t {
                Type::Literal { kind: "string", value } => vec![value.clone()],
                Type::Union(parts) => match parts.iter().map(|p| if let Type::Literal { kind: "string", value } = p { Some(value.clone()) } else { None }).collect() {
                    Some(v) => v,
                    None => continue,
                },
                _ => continue,
            },
            None => continue,
        };
        if keys.is_empty() || keys.len() > limit {
            continue;
        }
        let variants = keys
            .iter()
            .map(|key| {
                let mut s = sig.clone();
                s.type_params.remove(i);
                for p in &mut s.params {
                    if is_tp(&p.ty) {
                        p.constant = Some(key.clone());
                        p.ty = Type::prim("string");
                    }
                }
                let mut probe = std::mem::take(&mut s);
                walk_signature(&mut probe, &mut Vec::new(), &mut |t, _| {
                    if is_tp(t) {
                        *t = Type::Literal { kind: "string", value: key.clone() };
                        return false;
                    }
                    // `Map[K]` is the type of the key's property.
                    if let Type::Op(op, _) = t
                        && let Op::IndexedAccess(Type::Ref(m), index) = &**op
                        && is_tp(index)
                        && m.kind != "typeParameter"
                        && let Some(ms) = m.sym
                    {
                        *t = front.property_type(ms, key);
                        return false;
                    }
                    true
                });
                probe
            })
            .collect();
        return Some(variants);
    }
    None
}

// Evaluation can put a union inside a union: `A | keyof B`.
// A union member that names a union alias becomes that alias's members:
// `string | BufferSource` is `string | ArrayBufferView | ArrayBuffer`, so each
// member can be told apart from all the others.
fn splice_union_aliases(decls: &mut [Decl]) {
    // By name, and for an alias in a namespace, by its qualified name.
    fn collect(decls: &[Decl], prefix: &str, out: &mut HashMap<String, Type>) {
        for d in decls {
            match d {
                Decl::Alias(a) if a.type_params.is_empty() && matches!(a.ty, Type::Union(_)) => {
                    out.insert(format!("{prefix}{}", a.name), a.ty.clone());
                }
                Decl::Namespace(n) => collect(&n.decls, &format!("{prefix}{}.", n.name), out),
                _ => {}
            }
        }
    }
    let mut bodies: HashMap<String, Type> = HashMap::new();
    collect(decls, "", &mut bodies);
    let splice = |t: &mut Type| {
        let Type::Union(parts) = t else { return };
        for p in parts.iter_mut() {
            if let Type::Ref(r) = p
                && r.kind == "typeAlias"
                && r.args.is_empty()
                && let Some(body) = bodies.get(r.qualified.as_deref().unwrap_or(&r.name))
            {
                *p = body.clone();
            }
        }
    };
    // Aliases of aliases: a few rounds reach the bottom.
    for _ in 0..4 {
        walk(decls, &mut |t, _| {
            splice(t);
            true
        });
    }
}

// Bases that are not named types: an object a type-level expression
// evaluated to (`extends Partial<X>`) gives its members to the interface,
// and `any`, which says nothing, is dropped.
fn inline_object_bases(front: &mut Front, decls: &mut [Decl]) {
    // A base that is an alias, by what the alias lowered to: an object, a
    // class or interface (`type _Worker = Worker`), or nothing.
    let mut aliases: HashMap<String, (Vec<TypeParam>, Type)> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Alias(a) = d
            && (a.type_params.is_empty() && matches!(a.ty, Type::Object(_) | Type::Erased(_) | Type::Primitive(_))
                || matches!(&a.ty, Type::Ref(r) if matches!(r.kind, "class" | "interface")))
        {
            aliases.insert(a.name.clone(), (a.type_params.clone(), a.ty.clone()));
        }
    }
    inline_object_bases_with(front, decls, &aliases);
}

// `aliases = "expand"`: `List<number>`, where `type List<T> = T[]`, is
// `number[]`. Not an alias of an object, a function or a union, which are
// declarations generators keep generic; nor one that refers to itself. The
// aliases expanded are gone.
fn expand_aliases(front: &mut Front, decls: &mut Vec<Decl>) {
    fn collect(decls: &[Decl], prefix: &str, out: &mut HashMap<String, (Vec<TypeParam>, Type)>) {
        for d in decls {
            match d {
                Decl::Alias(a) if !a.type_params.is_empty() && !matches!(a.ty, Type::Object(_) | Type::Function(_) | Type::Constructor(_) | Type::Union(_)) => {
                    let mut own = false;
                    let mut ty = a.ty.clone();
                    walk_type(&mut ty, &mut Vec::new(), &mut |t, _| {
                        own |= matches!(t, Type::Ref(r) if r.name == a.name);
                        true
                    });
                    if !own {
                        out.insert(format!("{prefix}{}", a.name), (a.type_params.clone(), a.ty.clone()));
                    }
                }
                Decl::Namespace(n) => collect(&n.decls, &format!("{prefix}{}.", n.name), out),
                _ => {}
            }
        }
    }
    fn expand(t: &mut Type, aliases: &HashMap<String, (Vec<TypeParam>, Type)>, depth: usize, count: &mut usize) {
        walk_type(t, &mut Vec::new(), &mut |t, _| {
            let Type::Ref(r) = t else { return true };
            let key = r.qualified.as_ref().unwrap_or(&r.name);
            let Some((params, body)) = aliases.get(key).filter(|_| r.kind == "typeAlias" && depth < 16) else { return true };
            let mut body = instantiate(params, body, &r.args);
            expand(&mut body, aliases, depth + 1, count);
            *t = body;
            *count += 1;
            false
        });
    }
    let mut aliases = HashMap::new();
    collect(decls, "", &mut aliases);
    if aliases.is_empty() {
        return;
    }
    let mut count = 0;
    walk(decls, &mut |t, _| {
        expand(t, &aliases, 0, &mut count);
        false
    });
    *front.stats.entry("expanded: generic alias".into()).or_default() += count;
    fn drop(decls: &mut Vec<Decl>, prefix: &str, aliases: &HashMap<String, (Vec<TypeParam>, Type)>) {
        decls.retain_mut(|d| match d {
            Decl::Alias(a) => !aliases.contains_key(&format!("{prefix}{}", a.name)),
            Decl::Namespace(n) => {
                drop(&mut n.decls, &format!("{prefix}{}.", n.name), aliases);
                true
            }
            _ => true,
        });
    }
    drop(decls, "", &aliases);
}

// An alias's body, its type parameters replaced by `args` (missing ones by `any`).
fn instantiate(params: &[TypeParam], body: &Type, args: &[Type]) -> Type {
    let mut t = body.clone();
    walk_type(&mut t, &mut Vec::new(), &mut |t, _| {
        let Type::Ref(r) = t else { return true };
        if r.kind != "typeParameter" {
            return true;
        }
        let Some(i) = params.iter().position(|p| if p.sym.is_some() && r.sym.is_some() { p.sym == r.sym } else { p.name == r.name }) else { return true };
        *t = args.get(i).cloned().unwrap_or_else(|| Type::prim("any"));
        false
    });
    t
}

fn inline_object_bases_with(front: &mut Front, decls: &mut [Decl], aliases: &HashMap<String, (Vec<TypeParam>, Type)>) {
    for d in decls {
        match d {
            Decl::Interface(i) => {
                let own: HashSet<String> = i.members.iter().map(|m| m.name.clone()).collect();
                let mut kept = Vec::new();
                for base in std::mem::take(&mut i.extends) {
                    let base = match &base {
                        Type::Ref(r) if r.kind == "typeAlias" && r.qualified.is_none() => match aliases.get(&r.name) {
                            Some((params, body)) => instantiate(params, body, &r.args),
                            None => base,
                        },
                        _ => base,
                    };
                    match base {
                        Type::Object(members) => {
                            front.count("merged: object base");
                            i.members.extend(members.into_iter().filter(|m| !own.contains(&m.name)));
                        }
                        Type::Erased(_) | Type::Primitive(_) => front.count("dropped: base that is not a type"),
                        t => kept.push(t),
                    }
                }
                i.extends = kept;
            }
            Decl::Namespace(n) => inline_object_bases_with(front, &mut n.decls, aliases),
            _ => {}
        }
    }
}

fn flatten_unions(decls: &mut [Decl]) {
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

// ---- generics ----

// With `keep_keyed`, type parameters over the keys of a map stay: `keyed_params = "map"`.
fn erase_generics(decls: &mut [Decl], keep_keyed: bool) {
    let keep = if keep_keyed { keyed_syms(decls) } else { HashSet::new() };
    erase_type_params(decls, true, &|p| p.sym.is_none_or(|s| !keep.contains(&s)), &keep);
}

// `K extends keyof M`, for a map `M` that is not generic: the map.
pub fn keyed_map(tp: &TypeParam) -> Option<&Ref> {
    match &tp.constraint {
        Some(Type::Op(op, _)) => match &**op {
            Op::KeyOf(Type::Ref(m)) if m.kind != "typeParameter" && m.args.is_empty() => Some(m),
            _ => None,
        },
        _ => None,
    }
}

// The type parameters of signatures that range over the keys of a map.
fn keyed_syms(decls: &mut [Decl]) -> HashSet<P<Symbol>> {
    let mut out = HashSet::new();
    each_signature(decls, &mut |s| out.extend(s.type_params.iter().filter(|p| keyed_map(p).is_some()).filter_map(|p| p.sym)));
    out
}

// Whether a type mentions one of these type parameters.
fn mentions(t: &Type, syms: &HashSet<P<Symbol>>) -> bool {
    let mut found = false;
    let mut probe = t.clone();
    walk_type(&mut probe, &mut Vec::new(), &mut |t, _| {
        found |= matches!(t, Type::Ref(r) if r.kind == "typeParameter" && r.sym.is_some_and(|s| syms.contains(&s)));
        !found
    });
    found
}

// Removes the type parameters `erase` picks, replacing each use by its
// constraint. `declarations` includes those of interfaces and aliases, not
// only those of signatures.
fn erase_type_params(decls: &mut [Decl], declarations: bool, erase: &dyn Fn(&TypeParam) -> bool, keep: &HashSet<P<Symbol>>) {
    // Type parameters have unique identities, so no scoping is needed.
    let mut constraints: Constraints = HashMap::new();
    let mut collect = |params: &mut Vec<TypeParam>| {
        params.retain(|p| {
            if !erase(p) {
                return true;
            }
            if let Some(s) = p.sym {
                constraints.insert((s, p.tid), p.constraint.clone());
            }
            false
        });
    };
    for_each_type_params(decls, declarations, &mut collect);
    walk(decls, &mut |t, _| {
        if matches!(t, Type::Ref(r) if r.kind == "typeParameter") {
            return !substitute_constraint(t, &constraints, declarations, keep, 0);
        }
        true
    });
}

type Constraints = HashMap<(P<Symbol>, Option<u32>), Option<Type>>;

// Replaces a type parameter by its constraint. Constraints may mention other
// type parameters, or themselves (`T extends Comparable<T>`).
// With `all`, a type parameter that is not known is erased to `any`;
// otherwise it is left alone. Returns whether the type itself was replaced.
fn substitute_constraint(t: &mut Type, constraints: &Constraints, all: bool, keep: &HashSet<P<Symbol>>, depth: usize) -> bool {
    let top = t as *const Type;
    let mut replaced = false;
    walk_type(t, &mut Vec::new(), &mut |t, _| {
        let Type::Ref(r) = t else { return true };
        if r.kind != "typeParameter" {
            return true;
        }
        // Checker-made references carry a type identity; written ones do not.
        let found = r.sym.and_then(|s| constraints.get(&(s, r.tid)).or_else(|| constraints.get(&(s, None))));
        if found.is_none() && (!all || r.sym.is_some_and(|s| keep.contains(&s))) {
            return true;
        }
        replaced |= std::ptr::eq(t, top);
        *t = match found {
            Some(Some(k)) if depth < 3 => {
                let mut k = k.clone();
                substitute_constraint(&mut k, constraints, all, keep, depth + 1);
                k
            }
            _ => Type::prim("any"),
        };
        false
    });
    replaced
}

fn for_each_type_params(decls: &mut [Decl], declarations: bool, f: &mut dyn FnMut(&mut Vec<TypeParam>)) {
    if declarations {
        fn of_decls(decls: &mut [Decl], f: &mut dyn FnMut(&mut Vec<TypeParam>)) {
            for d in decls {
                match d {
                    Decl::Interface(i) => f(&mut i.type_params),
                    Decl::Alias(a) => f(&mut a.type_params),
                    Decl::Namespace(n) => of_decls(&mut n.decls, f),
                    _ => {}
                }
            }
        }
        of_decls(decls, f);
    }
    each_signature(decls, &mut |s| f(&mut s.type_params));
}

// Visits every signature: members, constructors, functions and function types.
fn each_signature(decls: &mut [Decl], f: &mut dyn FnMut(&mut Signature)) {
    fn members(ms: &mut [Member], f: &mut dyn FnMut(&mut Signature)) {
        ms.iter_mut().filter_map(|m| m.sig.as_mut()).for_each(f);
    }
    fn of_decls(decls: &mut [Decl], f: &mut dyn FnMut(&mut Signature)) {
        for d in decls {
            match d {
                Decl::Interface(i) => {
                    members(&mut i.members, f);
                    if let Some(c) = &mut i.class {
                        members(&mut c.statics, f);
                        members(&mut c.constructors, f);
                    }
                }
                Decl::Function(func) => f(&mut func.sig),
                Decl::Namespace(n) => of_decls(&mut n.decls, f),
                _ => {}
            }
        }
    }
    of_decls(decls, f);
    walk(decls, &mut |t, _| {
        match t {
            Type::Function(s) | Type::Constructor(s) => f(s),
            Type::Object(ms) => members(ms, f),
            _ => {}
        }
        true
    });
}

// ---- nullable ----

fn nullable(t: &mut Type) {
    let Type::Union(parts) = t else { return };
    let is_null = |p: &Type| p.is_prim("null") || p.is_prim("undefined");
    let absent = match (parts.iter().any(|p| p.is_prim("null")), parts.iter().any(|p| p.is_prim("undefined"))) {
        (true, true) => Absent::Either,
        (true, false) => Absent::Null,
        (false, true) => Absent::Undefined,
        (false, false) => return,
    };
    let mut rest: Vec<Type> = parts.drain(..).filter(|p| !is_null(p)).collect();
    let inner = match rest.len() {
        0 => Type::prim("null"),
        1 => rest.pop().unwrap(),
        _ => Type::Union(rest),
    };
    *t = Type::Nullable(Box::new(inner), absent);
}

// ---- literals ----

fn literals(decls: &mut Vec<Decl>, sep: &str) -> usize {
    let mut widened = 0;
    let mut enums: Vec<Enum> = Vec::new();
    let mut by_values: HashMap<Vec<String>, String> = HashMap::new();
    // Generated names must not collide with declared ones. An alias that is
    // itself a string union gives its name to the enum.
    let mut names: HashSet<String> = HashSet::new();
    for d in decls.iter() {
        if !matches!(d, Decl::Alias(a) if string_union(&a.ty).is_some()) {
            names.extend(decl_name(d).map(str::to_string));
        }
    }
    // `f<K extends "a" | "b">(key: K)`: without literal types K has no meaning
    // of its own, so it becomes the values it stands for.
    let only_literals = |t: &Type| match t {
        Type::Literal { .. } => true,
        Type::Union(parts) => parts.iter().all(|p| matches!(p, Type::Literal { .. })),
        _ => false,
    };
    erase_type_params(decls, false, &|p| p.constraint.as_ref().is_some_and(only_literals), &HashSet::new());
    // `createEvent(type: "MouseEvent")`: the parameter has one value. Keep it.
    each_signature(decls, &mut |s| {
        for p in s.params.iter_mut().filter(|p| !p.rest) {
            if let Type::Literal { kind, value } = &p.ty {
                p.constant = Some(value.clone());
                p.ty = Type::prim(kind);
            }
        }
    });
    let mut bodies: HashSet<*const Type> = HashSet::new();
    collect_bodies(decls, &mut bodies);
    // `readonly DEPTH_TEST: 0x0B71` is a constant: keep its value.
    each_member_list(decls, &mut |members| {
        for m in members.iter_mut().filter(|m| m.kind == MemberKind::Property) {
            if let Some(Type::Literal { kind, value }) = &m.ty {
                m.constant = Some(value.clone());
                m.ty = Some(Type::prim(kind));
            }
        }
    });
    // The same set of strings can appear in several places: name it after
    // the shortest (`Shape.kind` over `Shape.new.kind`).
    let mut best: HashMap<Vec<String>, Vec<String>> = HashMap::new();
    walk(decls, &mut |t, path| {
        if let Some(strings) = string_union(t) {
            // Inside an alias but not all of it: the alias keeps its own name.
            let mut path = path.to_vec();
            if path.len() == 1 && !bodies.contains(&(t as *const Type)) {
                path.push("value".into());
            }
            // The shortest path, then the first in alphabetical order: not the first found.
            let known = best.entry(sorted(strings)).or_insert_with(|| path.clone());
            if (path.len(), &path) < (known.len(), &*known) {
                *known = path;
            }
            return false;
        }
        true
    });
    walk(decls, &mut |t, _| {
        if let Some(strings) = string_union(t) {
            let path = &best[&sorted(strings.clone())];
            let name = by_values.entry(sorted(strings.clone())).or_insert_with(|| {
                let name = unique(path.join(sep), &mut names);
                let members = strings.iter().map(|v| EnumMember { name: v.clone(), kind: "string", value: v.clone() }).collect();
                enums.push(Enum { name: name.clone(), members, synthetic: true });
                name
            });
            *t = Type::synthetic_ref(name.clone(), "enum", vec![]);
            return false;
        }
        match t {
            Type::Union(parts) => {
                // Mixed unions: widen the literals and drop duplicates.
                let mut seen = HashSet::new();
                widened += parts.iter().filter(|p| matches!(p, Type::Literal { .. })).count();
                parts.iter_mut().for_each(widen);
                parts.retain(|p| seen.insert(type_key(p)));
                if parts.len() == 1 {
                    *t = parts.pop().unwrap();
                }
            }
            Type::Literal { .. } => {
                widened += 1;
                widen(t);
            }
            _ => {}
        }
        true
    });
    // `type X = "a" | "b"` becomes `enum X` itself.
    let named: HashSet<&str> = enums.iter().map(|e| e.name.as_str()).collect();
    let mut moved = HashMap::new();
    remove_enum_aliases(decls, &named, &[], sep, &mut moved);
    // What referred to such an alias now refers to the enum.
    walk(decls, &mut |t, _| {
        if let Type::Ref(r) = t
            && r.kind == "typeAlias"
        {
            // `symlink.Type`, as written or as the checker names it.
            if let Some(flat) = r.qualified.as_ref().and_then(|q| moved.get(q)).or_else(|| moved.get(&r.name)) {
                r.name = flat.clone();
                r.qualified = None;
                r.kind = "enum";
            } else if named.contains(r.name.as_str()) && r.qualified.is_none() {
                r.kind = "enum";
            }
        }
        true
    });
    decls.extend(enums.into_iter().map(Decl::Enum));
    widened
}

// In a namespace, the enum has the alias's path (`NodeJS_Platform`): what
// referred to the alias (`NodeJS.Platform`) goes in `moved`.
fn remove_enum_aliases(decls: &mut Vec<Decl>, enums: &HashSet<&str>, path: &[String], sep: &str, moved: &mut HashMap<String, String>) {
    decls.retain_mut(|d| match d {
        Decl::Alias(a) => {
            let flat = path.iter().chain([&a.name]).cloned().collect::<Vec<_>>().join(sep);
            let gone = matches!(&a.ty, Type::Ref(r) if r.kind == "enum" && r.name == flat && enums.contains(flat.as_str()));
            if gone && !path.is_empty() {
                moved.insert(path.iter().chain([&a.name]).cloned().collect::<Vec<_>>().join("."), flat);
            }
            !gone
        }
        Decl::Namespace(n) => {
            let inner: Vec<String> = path.iter().cloned().chain([n.name.clone()]).collect();
            remove_enum_aliases(&mut n.decls, enums, &inner, sep, moved);
            true
        }
        _ => true,
    });
}

// The values of a union made only of string literals: a closed set.
fn string_union(t: &Type) -> Option<Vec<String>> {
    let Type::Union(parts) = t else { return None };
    let strings: Vec<String> =
        parts.iter().filter_map(|p| if let Type::Literal { kind: "string", value } = p { Some(value.clone()) } else { None }).collect();
    (strings.len() == parts.len() && strings.len() > 1).then_some(strings)
}

// The checker orders union members its own way: compare sets, not sequences.
fn sorted(mut strings: Vec<String>) -> Vec<String> {
    strings.sort();
    strings
}

fn widen(t: &mut Type) {
    if let Type::Literal { kind, .. } = t {
        *t = Type::prim(kind);
    }
}

fn unique(base: String, taken: &mut HashSet<String>) -> String {
    // A name made of a long path (`Class_extend_result_extend_result_...`)
    // keeps its start and a hash of the rest: names become file names.
    let base = if base.len() > 64 {
        let hash = base.bytes().fold(0xcbf29ce484222325u64, |h, b| (h ^ b as u64).wrapping_mul(0x100000001b3));
        let mut cut = 48;
        while !base.is_char_boundary(cut) {
            cut -= 1;
        }
        format!("{}_{:08x}", &base[..cut], hash as u32)
    } else {
        base
    };
    let mut name = base.clone();
    let mut n = 2;
    while !taken.insert(name.clone()) {
        name = format!("{base}{n}");
        n += 1;
    }
    name
}

// ---- anonymous types ----

// Inline object and function types become named declarations.
// `this` in an inline type of a class (`options: (opt) => this`) is the
// class: once hoisted out of it, `this` would mean nothing.
fn bind_this(decls: &mut [Decl]) {
    for d in decls {
        match d {
            Decl::Interface(i) => {
                let args = i.type_params.iter().map(|p| Type::Ref(Ref { name: p.name.clone(), kind: "typeParameter", sym: p.sym, args: Vec::new(), tid: p.tid, qualified: None, file: None, computed: false, node: None })).collect();
                let me = Type::Ref(Ref { name: i.name.clone(), kind: if i.class.is_some() { "class" } else { "interface" }, sym: i.sym, args, tid: None, qualified: None, file: None, computed: false, node: None });
                let inline = &mut |t: &mut Type, _: &[String]| {
                    if !matches!(t, Type::Function(_) | Type::Constructor(_) | Type::Object(_)) {
                        return true;
                    }
                    walk_type(t, &mut Vec::new(), &mut |u, _| {
                        if matches!(u, Type::This) {
                            *u = me.clone();
                        }
                        true
                    });
                    false
                };
                walk_members(&mut i.members, &mut Vec::new(), inline);
                if let Some(c) = &mut i.class {
                    walk_members(&mut c.statics, &mut Vec::new(), inline);
                }
            }
            Decl::Namespace(n) => bind_this(&mut n.decls),
            _ => {}
        }
    }
}

fn hoist(decls: &mut Vec<Decl>, sep: &str) {
    bind_this(decls);
    let mut names: HashSet<String> = HashSet::new();
    collect_declared_names(decls, "", sep, &mut names);
    let mut by_key: HashMap<String, (String, Vec<Type>)> = HashMap::new();
    // An inline type that already has a name (`type Listener = (e: Event) => void`) uses it.
    for d in decls.iter() {
        if let Decl::Alias(a) = d
            && a.type_params.is_empty()
            && matches!(a.ty, Type::Function(_) | Type::Constructor(_))
        {
            by_key.entry(type_key(&a.ty)).or_insert_with(|| (a.name.clone(), Vec::new()));
        }
    }
    let mut start = 0;
    // Hoisted declarations may themselves contain inline types.
    loop {
        let mut added: Vec<Decl> = Vec::new();
        // The body of an alias, or of a `var`, already has the declaration's name.
        let mut bodies: HashSet<*const Type> = HashSet::new();
        collect_bodies(&decls[start..], &mut bodies);
        // A callback type used in several places is named by what it takes,
        // not by the first place it was found.
        let keyed = keyed_syms(&mut decls[start..]);
        let mut uses: HashMap<String, usize> = HashMap::new();
        // A type is named after the best place it appears: the shortest path,
        // then the first in alphabetical order. Moving declarations around
        // does not rename it.
        let mut best: HashMap<String, Vec<String>> = HashMap::new();
        walk(&mut decls[start..], &mut |t, path| {
            if matches!(t, Type::Function(_) | Type::Constructor(_)) && !bodies.contains(&(t as *const Type)) {
                *uses.entry(type_key(t)).or_default() += 1;
            }
            if matches!(t, Type::Object(_) | Type::Function(_) | Type::Constructor(_)) && !bodies.contains(&(t as *const Type)) {
                let known = best.entry(type_key(t)).or_insert_with(|| path.to_vec());
                if (path.len(), path) < (known.len(), &known[..]) {
                    *known = path.to_vec();
                }
            }
            true
        });
        walk(&mut decls[start..], &mut |t, path| {
            let is_function = matches!(t, Type::Function(_) | Type::Constructor(_));
            if !(matches!(t, Type::Object(_)) || is_function) || bodies.contains(&(t as *const Type)) {
                return true;
            }
            // A callback of `M[K]` depends on its signature's key: it stays inline.
            // An object holding one is hoisted, the signature with it.
            if is_function && !keyed.is_empty() && mentions(t, &keyed) {
                return false;
            }
            let key = type_key(t);
            let (name, args) = by_key
                .entry(key)
                .or_insert_with(|| {
                    let shared = uses.get(&type_key(t)).is_some_and(|n| *n > 1);
                    let base = match t {
                        Type::Function(s) | Type::Constructor(s) if shared => callback_name(s, sep),
                        _ => best.get(&type_key(t)).map_or_else(|| path.join(sep), |p| p.join(sep)),
                    };
                    let name = unique(base, &mut names);
                    // Enclosing type parameters the type mentions become its own.
                    let free = free_type_params(t);
                    let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None }).collect();
                    let args = free.into_iter().map(Type::Ref).collect();
                    let body = std::mem::replace(t, Type::This);
                    // Its methods' functions are not types' names, nor its constructor.
                    if let Type::Object(members) = &body {
                        for m in members.iter().filter(|m| m.kind == MemberKind::Method).map(|m| m.name.as_str()).chain(["create"]) {
                            names.insert(format!("{name}{sep}{m}"));
                            names.insert(format!("{name}_{m}"));
                        }
                    }
                    added.push(match body {
                        Type::Object(members) => Decl::Interface(Interface {
                            name: name.clone(), sym: None, type_params, extends: vec![], ancestors: vec![], members, class: None, synthetic: true, js_name: None,
                        }),
                        ty => Decl::Alias(Alias { name: name.clone(), sym: None, type_params, ty, computed: false, synthetic: true }),
                    });
                    (name, args)
                })
                .clone();
            let kind = if is_function { "typeAlias" } else { "interface" };
            *t = Type::synthetic_ref(name, kind, args);
            false
        });
        if added.is_empty() {
            break;
        }
        start = decls.len();
        decls.extend(added);
    }
}

// `(this: HTMLElement, ev: MouseEvent) => any` -> `HTMLElement_MouseEvent_fn`.
fn callback_name(s: &Signature, sep: &str) -> String {
    let mut parts: Vec<String> = s.this.iter().chain(s.params.iter().map(|p| &p.ty)).map(mnemonic).collect();
    if parts.is_empty() {
        parts.push("void".into());
    }
    parts.push("fn".into());
    parts.join(sep)
}

fn decl_name(d: &Decl) -> Option<&str> {
    Some(match d {
        Decl::Interface(i) => &i.name,
        Decl::Alias(a) => &a.name,
        Decl::Enum(e) => &e.name,
        Decl::Namespace(n) => &n.name,
        Decl::Function(_) | Decl::Var(_) => return None,
    })
}

fn collect_bodies(decls: &[Decl], out: &mut HashSet<*const Type>) {
    for d in decls {
        match d {
            Decl::Alias(a) => {
                out.insert(&a.ty);
            }
            Decl::Var(v) if matches!(v.ty, Type::Object(_)) => {
                out.insert(&v.ty);
            }
            Decl::Namespace(n) => collect_bodies(&n.decls, out),
            _ => {}
        }
    }
}

// Type parameters a type uses but does not declare.
fn free_type_params(t: &Type) -> Vec<Ref> {
    let mut used: Vec<Ref> = Vec::new();
    let mut bound: HashSet<P<Symbol>> = HashSet::new();
    let mut probe = t.clone();
    walk_type(&mut probe, &mut Vec::new(), &mut |t, _| {
        match t {
            Type::Function(s) | Type::Constructor(s) => bound.extend(s.type_params.iter().filter_map(|p| p.sym)),
            Type::Object(ms) => bound.extend(ms.iter().filter_map(|m| m.sig.as_ref()).flat_map(|s| s.type_params.iter().filter_map(|p| p.sym))),
            Type::Ref(r) if r.kind == "typeParameter" && !used.iter().any(|u| u.sym == r.sym) => used.push(r.clone()),
            _ => {}
        }
        true
    });
    used.retain(|r| r.sym.is_none_or(|s| !bound.contains(&s)));
    used
}

// ---- overloads ----

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

fn expand_members(members: &mut Vec<Member>, o: Options) {
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

fn expand_functions(decls: &mut Vec<Decl>, o: Options) {
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

// Names for overloads that share their fixed values. The first takes `base`;
// `base_taken` says whether it is already the member's own name.
fn family_names(base: &str, sigs: &[&Signature], base_taken: bool, taken: &mut HashSet<String>, o: Options) -> Vec<String> {
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
        let retyped: Vec<String> = s
            .params
            .iter()
            .enumerate()
            .filter(|(i, p)| first.params.get(*i).is_none_or(|q| type_key(&q.ty) != type_key(&p.ty)))
            .map(|(_, p)| param_mnemonic(s, &p.ty))
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
        ];
        let name = candidates.into_iter().flatten().find(|c| !taken.contains(c)).unwrap_or_else(|| format!("{base}{sep}{}", n + 1));
        out.push(unique(name, taken));
    }
    out
}

// A value as part of a name: `2d`, `image_png`.
fn name_part(value: &str) -> String {
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

fn mnemonic(t: &Type) -> String {
    match t {
        Type::Primitive(n) => n.clone(),
        // Written qualified: `Tokens.HTML` is `Tokens_HTML`.
        Type::Ref(r) => r.name.replace('.', "_"),
        Type::Array(inner) => format!("{}_array", mnemonic(inner)),
        Type::Nullable(inner, _) => format!("opt_{}", mnemonic(inner)),
        Type::Function(_) => "fn".into(),
        Type::Tuple(_) => "tuple".into(),
        Type::Erased(_) => "any".into(),
        _ => "x".into(),
    }
}

fn rename_members(members: &mut [Member], o: Options) {
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

fn rename_functions(decls: &mut [Decl], o: Options) {
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
            Decl::Namespace(n) => rename_functions(&mut n.decls, o),
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
                f.name = name;
            }
        }
    }
}
