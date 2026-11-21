// ---- anonymous types ----

use std::collections::{HashMap, HashSet};

use tsrs_ast::Symbol;
use tsrs_core::P;

use crate::ir::*;
use crate::xml::type_key;

use super::generics::{NAME_MAX, keyed_syms, mentions, unique};
use super::overloads::{describe, name_part};
use super::unions::collect_declared_names;
use super::walk::{visit, walk, walk_members, walk_type};

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

pub(super) fn hoist(decls: &mut Vec<Decl>, sep: &str) {
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
            by_key.entry(hoist_key(&a.ty)).or_insert_with(|| (a.name.clone(), Vec::new()));
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
        let mut order: Vec<String> = Vec::new();
        let mut sample: HashMap<String, Type> = HashMap::new();
        let mut opt_members: HashMap<String, HashSet<String>> = HashMap::new();
        walk(&mut decls[start..], &mut |t, path| {
            if matches!(t, Type::Function(_) | Type::Constructor(_)) && !bodies.contains(&(t as *const Type)) {
                *uses.entry(hoist_key(t)).or_default() += 1;
            }
            if matches!(t, Type::Object(_) | Type::Function(_) | Type::Constructor(_)) && !bodies.contains(&(t as *const Type)) {
                let key = hoist_key(t);
                if let Type::Object(ms) = t {
                    let entry = opt_members.entry(key.clone()).or_default();
                    for m in ms {
                        if m.optional && m.ty.as_ref().is_some_and(|ty| matches!(ty, Type::Nullable(..))) {
                            entry.insert(m.name.clone());
                        }
                    }
                }
                if !sample.contains_key(&key) {
                    order.push(key.clone());
                    sample.insert(key.clone(), t.clone());
                }
                let known = best.entry(key).or_insert_with(|| path.to_vec());
                if (path.len(), path) < (known.len(), &known[..]) {
                    *known = path.to_vec();
                }
            }
            true
        });
        // Types that would have one place's name (overloads' `options`, a
        // union's object and callback) are named apart by what they are.
        let mut at_path: HashMap<&Vec<String>, Vec<&String>> = HashMap::new();
        for k in &order {
            at_path.entry(&best[k]).or_default().push(k);
        }
        let mut apart: HashMap<String, String> = HashMap::new();
        for keys in at_path.values().filter(|ks| ks.len() > 1) {
            let types: Vec<&Type> = keys.iter().map(|k| &sample[*k]).collect();
            for (k, word) in keys.iter().zip(siblings_apart(&types, sep)) {
                if let Some(w) = word {
                    apart.insert((*k).clone(), w);
                }
            }
        }
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
            let key = hoist_key(t);
            let (name, args) = by_key
                .entry(key.clone())
                .or_insert_with(|| {
                    let shared = uses.get(&key).is_some_and(|n| *n > 1);
                    let mut at = best.get(&key).map_or_else(|| path.join(sep), |p| p.join(sep));
                    if let Some(w) = apart.get(&key) {
                        at = format!("{at}{sep}{w}");
                    // Its declaration's name only, which that has (`type ExpandedState
                    // = true | Record<string, boolean>`): by what it is.
                    } else if best.get(&key).map_or(path.len(), Vec::len) == 1 && names.contains(&at) {
                        at = format!("{at}{sep}{}", kind_word(t));
                    }
                    // A shared callback by what it takes, if that is short; else where it is first.
                    let base = match t {
                        // Another of that name (pixi's `(width, localWidth)` beside
                        // `(height, localHeight)`): where it is.
                        Type::Function(s) | Type::Constructor(s) if shared => callback_name(s, sep).filter(|n| n.len() <= NAME_MAX && !names.contains(n)).unwrap_or(at),
                        _ => at,
                    };
                    let name = unique(base, &mut names);
                    // Enclosing type parameters the type mentions become its own.
                    let free = free_type_params(t);
                    let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None }).collect();
                    let args = free.into_iter().map(Type::Ref).collect();
                    let body = std::mem::replace(t, Type::This);
                    added.push(match body {
                        Type::Object(mut members) => {
                            if let Some(opts) = opt_members.get(&key) {
                                for m in &mut members {
                                    if !m.optional && opts.contains(&m.name) && m.ty.as_ref().is_some_and(|ty| matches!(ty, Type::Nullable(..))) {
                                        m.optional = true;
                                    }
                                }
                            }
                            Decl::Interface(Interface {
                                name: name.clone(), sym: None, type_params, extends: vec![], ancestors: vec![], ancestor_syms: vec![], members, class: None, synthetic: true, js_name: None,
                            })
                        }
                        ty => Decl::Alias(Alias { name: name.clone(), sym: None, type_params, extends: Vec::new(), ty, computed: false, synthetic: true }),
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

// A dictionary of `S` or of itself, unrolled (echarts' `MapToType<any, S>`
// is `{ [k]: S | { [k]: S | ... } }`), once: the second's own is `any`.
// `Record<string, Record<string, S>>`, of nothing beside, is two.
pub(super) fn cut_recursive_dictionary(t: &mut Type) {
    fn dictionary(t: &mut Type) -> Option<(String, &mut Type)> {
        let Type::Object(ms) = t else { return None };
        let [m] = ms.as_mut_slice() else { return None };
        if m.kind != MemberKind::Index {
            return None;
        }
        let sig = m.sig.as_mut()?;
        let key = type_key(&sig.params.first()?.ty);
        Some((key, sig.ret.as_mut()?))
    }
    // A value's other members, and its dictionary.
    fn split(v: &mut Type) -> Option<(Vec<String>, &mut Type)> {
        let Type::Union(parts) = v else { return None };
        let others: Vec<String> = parts.iter().filter(|p| !matches!(p, Type::Object(_))).map(type_key).collect();
        let inner = parts.iter_mut().find(|p| matches!(p, Type::Object(_)))?;
        Some((others, inner))
    }
    let Some((key, value)) = dictionary(t) else { return };
    let Some((others, inner)) = split(value) else { return };
    if others.is_empty() {
        return;
    }
    let Some((inner_key, inner_value)) = dictionary(inner) else { return };
    if inner_key != key {
        return;
    }
    if let Some((inner_others, innermost)) = split(inner_value)
        && inner_others == others
        && dictionary(innermost).is_some()
    {
        *innermost = Type::Erased("recursive anonymous type");
    }
}

// What an inline type is, in a word.
fn kind_word(t: &Type) -> &'static str {
    match t {
        Type::Object(ms) if !ms.is_empty() && ms.iter().all(|m| m.kind == MemberKind::Index) => "map",
        Type::Object(_) => "object",
        Type::Constructor(_) => "ctor",
        _ => "fn",
    }
}

// Words for each of several inline types one place would name, to name them
// apart: the first keeps the place's name, each other is named by how it
// differs from it (`lstatSync_options_bigint`, `stat_callback_BigIntStats`).
fn siblings_apart(ts: &[&Type], sep: &str) -> Vec<Option<String>> {
    let first = ts[0];
    // `{place}_{member}` is the first's member where names are flat (C's
    // `lstatSync_options_bigint`): no word of those.
    let members: HashSet<String> = match first {
        Type::Object(ms) => ms.iter().map(|m| m.name.clone()).chain(["create".to_string()]).collect(),
        _ => HashSet::new(),
    };
    let mut out: Vec<Option<String>> = vec![None];
    for t in &ts[1..] {
        let cands: Vec<String> = differences(first, t, sep).into_iter().filter(|c| !members.contains(c)).collect();
        let taken = |c: &String, out: &Vec<Option<String>>| out.iter().flatten().any(|o| o == c);
        // The first word no earlier one has, else two.
        let word = cands.iter().find(|c| !taken(c, &out)).cloned().or_else(|| {
            cands.iter().enumerate().flat_map(|(i, a)| cands[i + 1..].iter().map(move |b| format!("{a}{sep}{b}"))).find(|c| !taken(c, &out))
        });
        out.push(word);
    }
    out
}

// How `t` differs from `first`, most telling first.
fn differences(first: &Type, t: &Type, sep: &str) -> Vec<String> {
    let props = |ms: &Vec<Member>| ms.iter().filter(|m| m.kind == MemberKind::Property).cloned().collect::<Vec<_>>();
    let mut out = Vec::new();
    match (first, t) {
        (Type::Object(a), Type::Object(b)) => {
            let (a, b) = (props(a), props(b));
            let find = |ms: &Vec<Member>, n: &str| ms.iter().find(|m| m.name == n).cloned();
            // A value it fixes: `bigint: true` is `bigint`, `throwIfNoEntry: false` `no_throwIfNoEntry`.
            for m in &b {
                if let Some(c) = &m.constant
                    && find(&a, &m.name).and_then(|o| o.constant) != Some(c.clone())
                {
                    out.push(match c.as_str() {
                        "true" if find(&a, &m.name).is_some() => format!("{}{sep}true", m.name),
                        "true" => m.name.clone(),
                        "false" => format!("no{sep}{}", m.name),
                        c if c.parse::<f64>().is_ok() => format!("{}{sep}{}", m.name, name_part(c)),
                        c => name_part(c).trim_matches('_').to_string(),
                    });
                }
            }
            // What only it has, required first.
            let mut added: Vec<&Member> = b.iter().filter(|m| find(&a, &m.name).is_none()).collect();
            added.sort_by_key(|m| m.optional);
            out.extend(added.iter().map(|m| m.name.clone()));
            // What it has otherwise.
            for m in &b {
                if let Some(o) = find(&a, &m.name)
                    && m.constant.is_none()
                    && let (Some(x), Some(y)) = (&o.ty, &m.ty)
                    && (type_key(x) != type_key(y) || o.constant.is_some())
                    && let Some(d) = describe(y)
                {
                    out.push(format!("{}{sep}{d}", m.name));
                }
            }
            for m in &b {
                match find(&a, &m.name) {
                    Some(o) if o.optional && !m.optional => out.push(m.name.clone()),
                    Some(o) if !o.optional && m.optional => out.push(format!("opt{sep}{}", m.name)),
                    _ => {}
                }
            }
            // What it lacks.
            out.extend(a.iter().filter(|m| find(&b, &m.name).is_none()).map(|m| format!("no{sep}{}", m.name)));
        }
        (Type::Function(a) | Type::Constructor(a), Type::Function(b) | Type::Constructor(b)) => {
            for (i, p) in b.params.iter().enumerate() {
                match a.params.get(i) {
                    Some(q) if type_key(&q.ty) == type_key(&p.ty) => {}
                    Some(_) => out.extend(describe(&p.ty)),
                    None => out.push(format!("with{sep}{}", p.name)),
                }
            }
            if let (Some(x), Some(y)) = (&a.ret, &b.ret)
                && type_key(x) != type_key(y)
                && let Some(d) = describe(y)
            {
                out.push(format!("to{sep}{d}"));
            }
        }
        (a, b) if kind_word(a) != kind_word(b) => out.push(kind_word(b).to_string()),
        _ => {}
    }
    out.retain(|w| !w.is_empty());
    out
}

// An inline type's key when hoisted: one `any` is another, whyever erased,
// and an already-nullable property (`attrs: Attrs | null` vs `attrs?: Attrs | null`)
// is treated as optional in the key so identical shapes share one struct.
fn hoist_key(t: &Type) -> String {
    let mut t = t.clone();
    visit(&mut t, &mut |x| {
        if matches!(x, Type::Erased(_)) {
            *x = Type::prim("any");
        } else if let Type::Object(members) = x {
            for m in members {
                if !m.optional && m.ty.as_ref().is_some_and(|ty| matches!(ty, Type::Nullable(..))) {
                    m.optional = true;
                }
            }
        }
        true
    });
    type_key(&t)
}

// `(this: HTMLElement, ev: MouseEvent) => any` -> `HTMLElement_MouseEvent_fn`,
// `(t: Token) => boolean` -> `Token_to_boolean_fn`; none if a type has no
// few words to say it.
fn callback_name(s: &Signature, sep: &str) -> Option<String> {
    let mut parts: Vec<String> = s.this.iter().chain(s.params.iter().map(|p| &p.ty)).map(describe).collect::<Option<_>>()?;
    let no_params = parts.is_empty();
    if no_params {
        parts.push("void".into());
    }
    let says_nothing = |t: &Type| {
        !no_params
            && (matches!(t, Type::Erased(_)) || matches!(t, Type::Primitive(p) if matches!(p.as_str(), "void" | "undefined" | "any" | "unknown")))
            || no_params && matches!(t, Type::Primitive(p) if matches!(p.as_str(), "void" | "undefined"))
    };
    if let Some(ret) = s.ret.as_ref().filter(|r| !says_nothing(r)) {
        parts.push("to".into());
        parts.push(describe(ret)?);
    }
    parts.push("fn".into());
    Some(parts.join(sep))
}

pub(super) fn collect_bodies(decls: &[Decl], out: &mut HashSet<*const Type>) {
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
pub(super) fn free_type_params(t: &Type) -> Vec<Ref> {
    let mut used: Vec<Ref> = Vec::new();
    collect_free(t, &mut Vec::new(), &mut used);
    used
}

// A type parameter is bound only within the signature that declares it: a
// method `extend<Options>` beside `new (o: Options)` leaves that one free.
fn collect_free(t: &Type, bound: &mut Vec<P<Symbol>>, used: &mut Vec<Ref>) {
    fn signature(s: &Signature, bound: &mut Vec<P<Symbol>>, used: &mut Vec<Ref>) {
        let depth = bound.len();
        bound.extend(s.type_params.iter().filter_map(|p| p.sym));
        for p in &s.type_params {
            p.constraint.iter().chain(p.default.iter()).for_each(|t| collect_free(t, bound, used));
        }
        s.params.iter().for_each(|p| collect_free(&p.ty, bound, used));
        s.this.iter().chain(s.ret.iter()).for_each(|t| collect_free(t, bound, used));
        bound.truncate(depth);
    }
    let mut probe = t.clone();
    walk_type(&mut probe, &mut Vec::new(), &mut |t, _| match t {
        Type::Function(s) | Type::Constructor(s) => {
            signature(s, bound, used);
            false
        }
        Type::Object(ms) => {
            for m in ms.iter() {
                m.ty.iter().for_each(|t| collect_free(t, bound, used));
                m.sig.iter().for_each(|s| signature(s, bound, used));
            }
            false
        }
        Type::Ref(r) if r.kind == "typeParameter" => {
            if r.sym.is_none_or(|s| !bound.contains(&s)) && !used.iter().any(|u| u.sym == r.sym) {
                used.push(r.clone());
            }
            true
        }
        _ => true,
    });
}
