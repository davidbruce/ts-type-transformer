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
    pub nullable: bool,
    pub literals: bool,
    pub anonymous: bool,
    pub optionals: bool,
    pub param_unions: bool,
    pub unions: bool,
    pub overloads: bool,
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
            classes: false, typeops: false, intersections: false, inheritance: false, generics: false, nullable: false,
            literals: false, anonymous: false, optionals: false, param_unions: false, unions: false, overloads: false,
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
            "nullable" => &mut self.nullable,
            "literals" => &mut self.literals,
            "anonymous" => &mut self.anonymous,
            "optional_params" => &mut self.optionals,
            "union_params" => &mut self.param_unions,
            "unions" => &mut self.unions,
            "overloads" => &mut self.overloads,
            _ => return None,
        })
    }
}

pub fn lower(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    front.merge_intersections = o.intersections;
    front.intersection_limit = o.intersection_limit;
    if o.classes {
        pair_classes(decls);
    }
    if o.inheritance {
        flatten(front, decls);
    }
    if o.typeops || o.intersections {
        evaluate(front, decls, o);
    }
    if o.generics {
        erase_generics(decls);
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
    if o.optionals {
        // Callback types cannot be overloaded: they receive every argument.
        walk(decls, &mut |t, _| {
            if let Type::Function(s) | Type::Constructor(s) = t {
                s.params.iter_mut().for_each(|p| p.optional = false);
            }
            true
        });
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
    if o.overloads {
        each_member_list(decls, &mut |members| rename_members(members, o));
        rename_functions(decls, o);
    }
    if o.classes {
        mark_class_refs(decls);
    }
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
            _ => m.js_name.clone().unwrap_or_else(|| m.name.clone()),
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

// ---- classes ----

// `interface X` + `var X: { prototype: X; new(): X; statics }` is a class.
fn pair_classes(decls: &mut Vec<Decl>) {
    let mut classes: HashMap<String, Class> = HashMap::new();
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
        classes.insert(v.name.clone(), class);
        false
    });
    for d in decls.iter_mut() {
        match d {
            Decl::Interface(i) => i.class = classes.remove(&i.name),
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
        // An alias body is evaluated without folding back into the alias itself.
        for_each_alias(decls, &mut |a| {
            if let (Type::Op(_, Some(node)), Some(sym)) = (&a.ty, a.sym) {
                a.ty = front.eval_alias_body(*node, sym);
            }
        });
    }
    walk(decls, &mut |t, _| {
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

// ---- generics ----

fn erase_generics(decls: &mut [Decl]) {
    // Type parameters have unique identities, so no scoping is needed.
    let mut constraints: Constraints = HashMap::new();
    let mut collect = |params: &mut Vec<TypeParam>| {
        for p in params.drain(..) {
            if let Some(s) = p.sym {
                constraints.insert((s, p.tid), p.constraint);
            }
        }
    };
    for_each_type_params(decls, &mut collect);
    walk(decls, &mut |t, _| {
        if matches!(t, Type::Ref(r) if r.kind == "typeParameter") {
            substitute_constraint(t, &constraints, 0);
            return false;
        }
        true
    });
}

type Constraints = HashMap<(P<Symbol>, Option<u32>), Option<Type>>;

// Replaces a type parameter by its constraint. Constraints may mention other
// type parameters, or themselves (`T extends Comparable<T>`).
fn substitute_constraint(t: &mut Type, constraints: &Constraints, depth: usize) {
    walk_type(t, &mut Vec::new(), &mut |t, _| {
        let Type::Ref(r) = t else { return true };
        if r.kind != "typeParameter" {
            return true;
        }
        // Checker-made references carry a type identity; written ones do not.
        let found = r.sym.and_then(|s| constraints.get(&(s, r.tid)).or_else(|| constraints.get(&(s, None))));
        *t = match found {
            Some(Some(k)) if depth < 3 => {
                let mut k = k.clone();
                substitute_constraint(&mut k, constraints, depth + 1);
                k
            }
            _ => Type::prim("any"),
        };
        false
    });
}

fn for_each_type_params(decls: &mut [Decl], f: &mut dyn FnMut(&mut Vec<TypeParam>)) {
    fn members(ms: &mut [Member], f: &mut dyn FnMut(&mut Vec<TypeParam>)) {
        for m in ms {
            if let Some(s) = &mut m.sig {
                f(&mut s.type_params);
            }
        }
    }
    for d in decls.iter_mut() {
        match d {
            Decl::Interface(i) => {
                f(&mut i.type_params);
                members(&mut i.members, f);
                if let Some(c) = &mut i.class {
                    members(&mut c.statics, f);
                    members(&mut c.constructors, f);
                }
            }
            Decl::Alias(a) => f(&mut a.type_params),
            Decl::Function(func) => f(&mut func.sig.type_params),
            Decl::Namespace(n) => for_each_type_params(&mut n.decls, f),
            _ => {}
        }
    }
    walk(decls, &mut |t, _| {
        match t {
            Type::Function(s) | Type::Constructor(s) => f(&mut s.type_params),
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
    let mut names: HashSet<String> = HashSet::new();
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
            let known = best.entry(strings).or_insert_with(|| path.to_vec());
            if path.len() < known.len() {
                *known = path.to_vec();
            }
            return false;
        }
        true
    });
    walk(decls, &mut |t, _| {
        if let Some(strings) = string_union(t) {
            let path = &best[&strings];
            let name = by_values.entry(strings.clone()).or_insert_with(|| {
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
    remove_enum_aliases(decls, &named);
    decls.extend(enums.into_iter().map(Decl::Enum));
    widened
}

fn remove_enum_aliases(decls: &mut Vec<Decl>, enums: &HashSet<&str>) {
    decls.retain_mut(|d| match d {
        Decl::Alias(a) => !matches!(&a.ty, Type::Ref(r) if r.kind == "enum" && r.name == a.name && enums.contains(a.name.as_str())),
        Decl::Namespace(n) => {
            remove_enum_aliases(&mut n.decls, enums);
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

fn widen(t: &mut Type) {
    if let Type::Literal { kind, .. } = t {
        *t = Type::prim(kind);
    }
}

fn unique(base: String, taken: &mut HashSet<String>) -> String {
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
fn hoist(decls: &mut Vec<Decl>, sep: &str) {
    let mut names: HashSet<String> = HashSet::new();
    let mut by_key: HashMap<String, (String, Vec<Type>)> = HashMap::new();
    let mut start = 0;
    // Hoisted declarations may themselves contain inline types.
    loop {
        let mut added: Vec<Decl> = Vec::new();
        walk(&mut decls[start..], &mut |t, path| {
            // A callback alias or a declaration's own body is already named.
            let is_function = matches!(t, Type::Function(_) | Type::Constructor(_));
            if !(matches!(t, Type::Object(_)) || is_function) || path.len() < 2 {
                return true;
            }
            let key = type_key(t);
            let (name, args) = by_key
                .entry(key)
                .or_insert_with(|| {
                    let name = unique(path.join(sep), &mut names);
                    // Enclosing type parameters the type mentions become its own.
                    let free = free_type_params(t);
                    let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None }).collect();
                    let args = free.into_iter().map(Type::Ref).collect();
                    let body = std::mem::replace(t, Type::This);
                    added.push(match body {
                        Type::Object(members) => Decl::Interface(Interface {
                            name: name.clone(), sym: None, type_params, extends: vec![], ancestors: vec![], members, class: None, synthetic: true,
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
        out = (required..=sig.params.len())
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
                        out.push(Decl::Function(Function { name: f.name.clone(), js_name: None, sig }));
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

// Names for overloads, in the style of web-sys: `f`, `f_with_x`, `f_with_x_and_y`.
fn overload_names(base: &str, sigs: &[&Signature], taken: &mut HashSet<String>, o: Options) -> Vec<String> {
    let sep = o.separator;
    let first = sigs[0];
    let mut out = vec![base.to_string()];
    for (n, s) in sigs.iter().enumerate().skip(1) {
        if o.numbered_overloads {
            out.push(unique(format!("{base}{sep}{}", n + 1), taken));
            continue;
        }
        let differing: Vec<&Param> = s
            .params
            .iter()
            .enumerate()
            .filter(|(i, p)| first.params.get(*i).is_none_or(|q| q.name != p.name || type_key(&q.ty) != type_key(&p.ty)))
            .map(|(_, p)| p)
            .collect();
        // `f(x: string)` / `f(x: number)`: the name says nothing, the type does.
        let part = |p: &Param| if first.params.iter().any(|q| q.name == p.name) { mnemonic(&p.ty) } else { p.name.clone() };
        let mut name = if differing.is_empty() {
            format!("{base}{sep}{}", s.params.len())
        } else {
            format!("{base}{sep}with{sep}{}", differing.iter().map(|p| part(p)).collect::<Vec<_>>().join(&format!("{sep}and{sep}")))
        };
        // Same parameter names, different types: name the types.
        if taken.contains(&name) {
            name = format!("{name}{sep}{}", differing.iter().map(|p| mnemonic(&p.ty)).collect::<Vec<_>>().join(sep));
        }
        out.push(unique(name, taken));
    }
    out
}

fn mnemonic(t: &Type) -> String {
    match t {
        Type::Primitive(n) => n.clone(),
        Type::Ref(r) => r.name.clone(),
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
        if !matches!(m.kind, MemberKind::Method | MemberKind::Construct | MemberKind::Call) {
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
        for (i, name) in idx.into_iter().zip(names).skip(1) {
            members[i].js_name = Some(base.clone());
            members[i].name = name;
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
        for (i, name) in idx.into_iter().zip(names).skip(1) {
            if let Decl::Function(f) = &mut decls[i] {
                f.js_name = Some(base.clone());
                f.name = name;
            }
        }
    }
}
