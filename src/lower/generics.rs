// ---- generics ----

use std::collections::{HashMap, HashSet};

use tsrs_ast::Symbol;
use tsrs_core::P;

use crate::ir::*;
use crate::xml::type_key;

use super::hoist::collect_bodies;
use super::overloads::name_part;
use super::walk::{each_member_list, walk, walk_type};

// With `keep_keyed`, type parameters over the keys of a map stay: `keyed_params = "map"`.
pub(super) fn erase_generics(decls: &mut [Decl], keep_keyed: bool) {
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
pub(super) fn keyed_syms(decls: &mut [Decl]) -> HashSet<P<Symbol>> {
    let mut out = HashSet::new();
    each_signature(decls, &mut |s| out.extend(s.type_params.iter().filter(|p| keyed_map(p).is_some()).filter_map(|p| p.sym)));
    out
}

// Whether a type mentions one of these type parameters.
pub(super) fn mentions(t: &Type, syms: &HashSet<P<Symbol>>) -> bool {
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
            // Marked: a result of it is one the caller reads (`map<U>(f: (v: T) => U)`).
            _ => Type::Erased("type parameter"),
        };
        false
    });
    replaced
}

pub(super) fn for_each_type_params(decls: &mut [Decl], declarations: bool, f: &mut dyn FnMut(&mut Vec<TypeParam>)) {
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
pub(super) fn each_signature(decls: &mut [Decl], f: &mut dyn FnMut(&mut Signature)) {
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

pub(super) fn nullable(t: &mut Type) {
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

pub(super) fn literals(decls: &mut Vec<Decl>, sep: &str) -> usize {
    let mut widened = 0;
    let mut enums: Vec<Enum> = Vec::new();
    let mut by_values: HashMap<Vec<String>, String> = HashMap::new();
    // Generated names must not collide with declared ones. An alias that is
    // itself a string union gives its name to the enum.
    let mut names: HashSet<String> = HashSet::new();
    for d in decls.iter() {
        if !matches!(d, Decl::Alias(a) if string_union(&a.ty).is_some())
            && !matches!(d, Decl::Function(_) | Decl::Var(_))
        {
            names.insert(d.name().to_string());
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
                // Another's place (leaflet's event names by kind, all `type`): with its first.
                let at = path.join(sep);
                let first = strings.first().map(|v| name_part(v).trim_matches('_').to_string()).filter(|v| !v.is_empty());
                let base = match first {
                    Some(f) if names.contains(&at) => format!("{at}{sep}{f}"),
                    _ => at,
                };
                let name = unique(base, &mut names);
                let members = strings.iter().map(|v| EnumMember { name: v.clone(), kind: "string", value: v.clone() }).collect();
                enums.push(Enum { name: name.clone(), sym: None, members, synthetic: true });
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
    if strings.len() != parts.len() {
        return None;
    }
    // Once each, as TypeScript has them: a library may write one twice.
    let mut seen = HashSet::new();
    let strings: Vec<String> = strings.into_iter().filter(|s| seen.insert(s.clone())).collect();
    (strings.len() > 1).then_some(strings)
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

// The longest name made of others' before it is cut and hashed.
pub(super) const NAME_MAX: usize = 64;

pub(super) fn unique(base: String, taken: &mut HashSet<String>) -> String {
    // A name made of a long path (echarts' `MapToType` of `Path`'s properties,
    // `Path__transitionState_animationProps___value_changed_…`) keeps where it
    // starts and where it ends: names become file names.
    let parts: Vec<&str> = base.split('_').collect();
    let base = if base.len() > NAME_MAX && parts.len() > 2 {
        let mut tail = Vec::new();
        let head = parts[0];
        for p in parts[1..].iter().rev() {
            if head.len() + tail.iter().map(|t: &&str| t.len() + 1).sum::<usize>() + p.len() + 1 > NAME_MAX {
                break;
            }
            tail.insert(0, *p);
        }
        if tail.iter().all(|t| t.is_empty()) { hashed(&base) } else { format!("{head}_{}", tail.join("_")) }
    } else if base.len() > NAME_MAX {
        hashed(&base)
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

// A name too long to keep: its start and a hash of it.
fn hashed(base: &str) -> String {
    let hash = crate::fnv1a64(base);
    let mut cut = 48;
    while !base.is_char_boundary(cut) {
        cut -= 1;
    }
    format!("{}_{:08x}", &base[..cut], hash as u32)
}
