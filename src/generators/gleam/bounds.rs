use std::collections::{HashMap, HashSet};

use super::types::Gen;
use super::{func_unit, lower, snake};
use super::super::model::{base_args, map_types, mentions, subst, Call, Class, Func, Model, TParam, Ty};

// The class a class's chain continues from, and the arguments it gives it:
// its first base that is a class of the model.
pub(super) fn parent(m: &Model, c: &Class) -> Option<(String, Vec<Ty>)> {
    c.bases.iter().find_map(|b| match b {
        Ty::Obj(n) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), Vec::new())),
        Ty::Generic(n, args) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), args.clone())),
        _ => None,
    })
}

// The top of a class's chain.
pub(super) fn chain_root(g: &Gen, class: &str) -> String {
    let mut at = class.to_string();
    while let Some((p, _)) = g.parents.get(&at) {
        at = p.clone();
    }
    at
}

// Every class a class extends, through any base.
fn ancestors(m: &Model, c: &Class) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo: Vec<&Class> = vec![c];
    while let Some(k) = todo.pop() {
        for b in &k.bases {
            let (Ty::Obj(n) | Ty::Generic(n, _)) = b else { continue };
            if let Some(base) = m.classes.iter().find(|x| x.name == *n)
                && !out.contains(n)
            {
                out.push(n.clone());
                todo.push(base);
            }
        }
    }
    out
}

// The classes a class extends off its chain: neither on the chain nor
// extended by a class on it. Their members it takes as its own.
pub(super) fn secondary(m: &Model, c: &Class) -> Vec<String> {
    let mut chain: Vec<&Class> = Vec::new();
    let mut at = c;
    while let Some((p, _)) = parent(m, at) {
        let Some(k) = m.classes.iter().find(|k| k.name == p) else { break };
        chain.push(k);
        at = k;
    }
    let mut covered: HashSet<String> = chain.iter().map(|k| k.name.clone()).collect();
    for k in &chain {
        covered.extend(ancestors(m, k));
    }
    ancestors(m, c).into_iter().filter(|a| !covered.contains(a)).collect()
}

// Each class takes its secondary bases' members, their type arguments
// applied; its own of a name win. Those on its chain it has as a subtype.
pub(super) fn take_secondary(m: &mut Model) {
    let mut taken: Vec<(usize, Vec<Func>)> = Vec::new();
    for (i, c) in m.classes.iter().enumerate() {
        let mut names: HashSet<String> = c.funcs.iter().map(|f| f.name.clone()).collect();
        let mut funcs = Vec::new();
        for s in secondary(m, c) {
            let (Some(k), Some(args)) = (m.classes.iter().find(|k| k.name == s), base_args(m, c, &s)) else { continue };
            for f in k.funcs.iter().filter(|f| Class::has_self(f)) {
                if names.insert(f.name.clone()) {
                    funcs.push(map_types(f, &|t| subst(t, &k.type_params, &args)));
                }
            }
        }
        taken.push((i, funcs));
    }
    for (i, funcs) in taken {
        m.classes[i].funcs.extend(funcs);
    }
}

// A callback holding itself, through its own types or another's: a Gleam
// alias may not, so within such a cycle a callback is `Dynamic`.
pub(super) fn break_cycles(m: &mut Model) {
    fn refs(t: &Ty, names: &HashSet<String>, out: &mut HashSet<String>) {
        match t {
            Ty::Obj(n) if names.contains(n) => {
                out.insert(n.clone());
            }
            Ty::Generic(n, args) => {
                if names.contains(n) {
                    out.insert(n.clone());
                }
                args.iter().for_each(|a| refs(a, names, out));
            }
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => refs(inner, names, out),
            Ty::Tuple(es) | Ty::Union(es) => es.iter().for_each(|e| refs(e, names, out)),
            _ => {}
        }
    }
    fn cut(t: &mut Ty, cyclic: &HashSet<String>) {
        match t {
            Ty::Obj(n) | Ty::Generic(n, _) if cyclic.contains(n) => *t = Ty::Any,
            Ty::Generic(_, args) => args.iter_mut().for_each(|a| cut(a, cyclic)),
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => cut(inner, cyclic),
            Ty::Tuple(es) | Ty::Union(es) => es.iter_mut().for_each(|e| cut(e, cyclic)),
            _ => {}
        }
    }
    let names: HashSet<String> = m.callbacks.iter().map(|c| c.name.clone()).collect();
    let edges: HashMap<String, HashSet<String>> = m
        .callbacks
        .iter()
        .map(|c| {
            let mut out = HashSet::new();
            c.params.iter().map(|p| &p.1).chain([&c.ret]).for_each(|t| refs(t, &names, &mut out));
            (c.name.clone(), out)
        })
        .collect();
    let reach = |from: &str| {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut todo: Vec<&str> = edges[from].iter().map(String::as_str).collect();
        while let Some(n) = todo.pop() {
            if seen.insert(n) {
                todo.extend(edges[n].iter().map(String::as_str));
            }
        }
        seen.into_iter().map(str::to_string).collect::<HashSet<String>>()
    };
    let reaches: HashMap<String, HashSet<String>> = names.iter().map(|n| (n.clone(), reach(n))).collect();
    for c in &mut m.callbacks {
        // Those it reaches that reach it back.
        let cycle: HashSet<String> = reaches[&c.name].iter().filter(|d| reaches[*d].contains(&c.name)).cloned().collect();
        if !cycle.is_empty() {
            c.params.iter_mut().for_each(|p| cut(&mut p.1, &cycle));
            cut(&mut c.ret, &cycle);
        }
    }
}

pub(super) fn generic_args(t: &Ty) -> &[Ty] {
    match t {
        Ty::Generic(_, args) => args,
        _ => &[],
    }
}

// Whether `t` names any type parameter in `params`.
pub(super) fn mentions_any(t: &Ty, params: &[TParam]) -> bool {
    params.iter().any(|p| mentions(t, &p.name))
}

fn is_chain_subtype(parents: &HashMap<String, (String, Vec<Ty>)>, sub: &str, base: &str) -> bool {
    let mut at = sub;
    loop {
        if at == base {
            return true;
        }
        let Some((p, _)) = parents.get(at) else { return false };
        at = p;
    }
}

fn resolve_default_args(tparams: &[TParam], args: &[Ty]) -> Vec<Ty> {
    let mut resolved = Vec::with_capacity(tparams.len());
    for (i, tp) in tparams.iter().enumerate() {
        if let Some(a) = args.get(i) {
            resolved.push(a.clone());
            continue;
        }
        if let Some(def) = &tp.default {
            let def_sub = subst(def, &tparams[..i], &resolved);
            let unresolved = tparams[i..].iter().any(|rem| mentions(&def_sub, &rem.name));
            if !unresolved {
                resolved.push(def_sub);
                continue;
            }
        }
        resolved.push(Ty::Any);
    }
    resolved
}

fn effective_base_args(m: &Model, ty: &Ty, base: &str) -> Option<Vec<Ty>> {
    let (name, args) = match ty {
        Ty::Obj(n) => (n.as_str(), Vec::new()),
        Ty::Generic(n, args) => (n.as_str(), args.clone()),
        _ => return None,
    };
    let Some(cls) = m.classes.iter().find(|c| c.name == name) else {
        return (name == base).then_some(args);
    };
    let resolved = resolve_default_args(&cls.type_params, &args);
    if name == base {
        return Some(resolved);
    }
    let bargs = base_args(m, cls, base)?;
    let subbed: Vec<Ty> = bargs.into_iter().map(|a| subst(&a, &cls.type_params, &resolved)).collect();
    Some(match m.classes.iter().find(|c| c.name == base) {
        Some(base_cls) => resolve_default_args(&base_cls.type_params, &subbed),
        None => subbed,
    })
}

fn arg_satisfies(
    m: &Model,
    classes: &HashMap<String, usize>,
    parents: &HashMap<String, (String, Vec<Ty>)>,
    arg: &Ty,
    base: &str,
    cargs: &[Ty],
    scope_tparams: &[TParam],
) -> bool {
    let concrete_check = |candidate: &Ty| -> bool {
        if cargs.is_empty() {
            return true;
        }
        let Some(eff) = effective_base_args(m, candidate, base) else { return false };
        cargs.iter().enumerate().all(|(k, ca)| matches!(ca, Ty::Any) || eff.get(k) == Some(ca))
    };
    match arg {
        Ty::Obj(x) | Ty::Generic(x, _) => classes.contains_key(x) && is_chain_subtype(parents, x, base) && concrete_check(arg),
        Ty::Mapped(map_name) => m
            .maps
            .iter()
            .find(|k| k.name == *map_name)
            .is_some_and(|km| !km.values.is_empty() && km.values.iter().all(|v| arg_satisfies(m, classes, parents, v, base, cargs, scope_tparams))),
        Ty::Param(tp_name) => {
            let mut cur = tp_name.as_str();
            let mut seen = HashSet::new();
            while seen.insert(cur) {
                let Some(tp) = scope_tparams.iter().find(|p| p.name == cur) else { return false };
                match &tp.constraint {
                    Some(c @ (Ty::Obj(x) | Ty::Generic(x, _))) => {
                        return classes.contains_key(x) && is_chain_subtype(parents, x, base) && concrete_check(c);
                    }
                    Some(Ty::Param(next)) => cur = next.as_str(),
                    _ => return false,
                }
            }
            false
        }
        _ => false,
    }
}

struct ClassBoundQuery<'a> {
    base: &'a str,
    cargs: &'a [Ty],
    m: &'a Model,
    classes: &'a HashMap<String, usize>,
    parents: &'a HashMap<String, (String, Vec<Ty>)>,
}

#[derive(Clone, Copy)]
enum BoundTarget<'a> {
    Class(&'a str, usize),
    Alias(&'a str, usize, &'a [TParam]),
}

fn check_bound_arg(
    q: &ClassBoundQuery<'_>,
    arg: &Ty,
    scope_tparams: &[TParam],
    owner_alias: Option<(&str, &[TParam])>,
    needed_aliases: &mut Vec<(String, usize)>,
) -> bool {
    if arg_satisfies(q.m, q.classes, q.parents, arg, q.base, q.cargs, scope_tparams) {
        return true;
    }
    if let Some((alias_name, alias_tps)) = owner_alias
        && let Ty::Param(tp_name) = arg
        && let Some(idx) = alias_tps.iter().position(|p| p.name == *tp_name)
    {
        needed_aliases.push((alias_name.to_string(), idx));
        return true;
    }
    false
}

fn check_bound_target(
    target: BoundTarget<'_>,
    q: &ClassBoundQuery<'_>,
    t: &Ty,
    scope_tparams: &[TParam],
    owner_alias: Option<(&str, &[TParam])>,
    needed_aliases: &mut Vec<(String, usize)>,
) -> bool {
    match t {
        Ty::Obj(cn) | Ty::Generic(cn, _) if matches!(target, BoundTarget::Class(tc, _) if q.classes.contains_key(cn) && is_chain_subtype(q.parents, cn, tc)) => {
            let BoundTarget::Class(tc, idx) = target else { unreachable!() };
            let Some(eff) = effective_base_args(q.m, t, tc) else {
                return false;
            };
            let ok = match eff.get(idx) {
                Some(a) => check_bound_arg(q, a, scope_tparams, owner_alias, needed_aliases),
                None => false,
            };
            if !ok {
                return false;
            }
            if let Ty::Generic(_, args) = t {
                args.iter().all(|a| check_bound_target(target, q, a, scope_tparams, owner_alias, needed_aliases))
            } else {
                true
            }
        }
        Ty::Obj(an) | Ty::Generic(an, _) if matches!(target, BoundTarget::Alias(ta, _, _) if an == ta) => {
            let BoundTarget::Alias(_, idx, atps) = target else { unreachable!() };
            let args = generic_args(t);
            let resolved = resolve_default_args(atps, args);
            let ok = match resolved.get(idx) {
                Some(a) => check_bound_arg(q, a, scope_tparams, owner_alias, needed_aliases),
                None => false,
            };
            if !ok {
                return false;
            }
            args.iter().all(|a| check_bound_target(target, q, a, scope_tparams, owner_alias, needed_aliases))
        }
        Ty::Generic(_, args) | Ty::Tuple(args) | Ty::Union(args) => {
            args.iter().all(|a| check_bound_target(target, q, a, scope_tparams, owner_alias, needed_aliases))
        }
        Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) | Ty::Record(inner) => {
            check_bound_target(target, q, inner, scope_tparams, owner_alias, needed_aliases)
        }
        _ => true,
    }
}

// Identifies class type parameters that are strictly constrained to a class
// across every instantiation in the model (including via hoisted callback/union aliases),
// so codecs can be omitted safely.
pub(super) fn compute_class_bound(
    m: &Model,
    classes: &HashMap<String, usize>,
    parents: &HashMap<String, (String, Vec<Ty>)>,
) -> HashMap<(String, usize), (String, Vec<Ty>)> {
    let alias_tparams: HashMap<&str, &[TParam]> = m
        .callbacks
        .iter()
        .map(|cb| (cb.name.as_str(), cb.type_params.as_slice()))
        .chain(m.unions.iter().map(|u| (u.name.as_str(), u.type_params.as_slice())))
        .collect();
    let mut out = HashMap::new();
    for c in &m.classes {
        for (i, p) in c.type_params.iter().enumerate() {
            let Some((base, cargs)) = (match &p.constraint {
                Some(con @ Ty::Obj(base)) if classes.contains_key(base) && !mentions_any(con, &c.type_params) => Some((base.clone(), Vec::new())),
                Some(con @ Ty::Generic(base, args)) if classes.contains_key(base) && !mentions_any(con, &c.type_params) => Some((base.clone(), args.clone())),
                _ => None,
            }) else {
                continue;
            };
            let query = ClassBoundQuery { base: &base, cargs: &cargs, m, classes, parents };
            let mut needed_aliases: Vec<(String, usize)> = Vec::new();
            // `from` and `cast` on `c` or any runtime-constructible subclass instantiate
            // defaults (falling back to `Dynamic`), so every such subclass must satisfy `c`'s bound.
            if m.classes.iter().any(|sub| {
                sub.js.is_some()
                    && is_chain_subtype(parents, &sub.name, &c.name)
                    && !check_bound_target(BoundTarget::Class(&c.name, i), &query, &Ty::Obj(sub.name.clone()), &[], None, &mut needed_aliases)
            }) {
                continue;
            }
            let mut checked_aliases: HashSet<(String, usize)> = HashSet::new();
            let mut pending: Vec<BoundTarget<'_>> = vec![BoundTarget::Class(&c.name, i)];
            let mut ok = true;
            while let Some(target) = pending.pop() {
                needed_aliases.clear();
                'classes: for c2 in &m.classes {
                    for tp in &c2.type_params {
                        if tp.default.as_ref().is_some_and(|d| !check_bound_target(target, &query, d, &c2.type_params, None, &mut needed_aliases))
                            || tp.constraint.as_ref().is_some_and(|ct| !check_bound_target(target, &query, ct, &c2.type_params, None, &mut needed_aliases))
                        {
                            ok = false;
                            break 'classes;
                        }
                    }
                    for b in &c2.bases {
                        if !check_bound_target(target, &query, b, &c2.type_params, None, &mut needed_aliases) {
                            ok = false;
                            break 'classes;
                        }
                    }
                    for t in c2.iterates.iter().chain(&c2.iterates_async).chain(&c2.elements) {
                        if !check_bound_target(target, &query, t, &c2.type_params, None, &mut needed_aliases) {
                            ok = false;
                            break 'classes;
                        }
                    }
                    for f in &c2.funcs {
                        let mut scope: Vec<TParam> = f.type_params.clone();
                        scope.extend(c2.type_params.iter().cloned());
                        if f.type_params.iter().any(|tp| {
                            tp.default.as_ref().is_some_and(|d| !check_bound_target(target, &query, d, &scope, None, &mut needed_aliases))
                                || tp.constraint.as_ref().is_some_and(|ct| !check_bound_target(target, &query, ct, &scope, None, &mut needed_aliases))
                        }) || f.params.iter().any(|(_, pt)| !check_bound_target(target, &query, pt, &scope, None, &mut needed_aliases))
                            || !check_bound_target(target, &query, &f.ret, &scope, None, &mut needed_aliases)
                        {
                            ok = false;
                            break 'classes;
                        }
                    }
                }
                if ok {
                    for f in &m.funcs {
                        let owner = if matches!(f.call, Call::Apply)
                            && let Some((_, Ty::Obj(cb) | Ty::Generic(cb, _))) = f.params.first()
                            && alias_tparams.contains_key(cb.as_str())
                        {
                            Some((cb.as_str(), f.type_params.as_slice()))
                        } else {
                            None
                        };
                        if f.type_params.iter().any(|tp| {
                            tp.default.as_ref().is_some_and(|d| !check_bound_target(target, &query, d, &f.type_params, None, &mut needed_aliases))
                                || tp.constraint.as_ref().is_some_and(|ct| !check_bound_target(target, &query, ct, &f.type_params, None, &mut needed_aliases))
                        }) || f.params.iter().any(|(_, pt)| !check_bound_target(target, &query, pt, &f.type_params, owner, &mut needed_aliases))
                            || !check_bound_target(target, &query, &f.ret, &f.type_params, owner, &mut needed_aliases)
                        {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    for cb in &m.callbacks {
                        let owner = Some((cb.name.as_str(), cb.type_params.as_slice()));
                        if cb.type_params.iter().any(|tp| {
                            tp.default.as_ref().is_some_and(|d| !check_bound_target(target, &query, d, &cb.type_params, None, &mut needed_aliases))
                                || tp.constraint.as_ref().is_some_and(|ct| !check_bound_target(target, &query, ct, &cb.type_params, None, &mut needed_aliases))
                        }) || cb.params.iter().any(|(_, pt)| !check_bound_target(target, &query, pt, &cb.type_params, owner, &mut needed_aliases))
                            || !check_bound_target(target, &query, &cb.ret, &cb.type_params, owner, &mut needed_aliases)
                        {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok {
                    for u in &m.unions {
                        let owner = Some((u.name.as_str(), u.type_params.as_slice()));
                        if u.type_params.iter().any(|tp| {
                            tp.default.as_ref().is_some_and(|d| !check_bound_target(target, &query, d, &u.type_params, None, &mut needed_aliases))
                                || tp.constraint.as_ref().is_some_and(|ct| !check_bound_target(target, &query, ct, &u.type_params, None, &mut needed_aliases))
                        }) || u.parts.iter().any(|pt| !check_bound_target(target, &query, pt, &u.type_params, owner, &mut needed_aliases))
                        {
                            ok = false;
                            break;
                        }
                    }
                }
                if !ok {
                    break;
                }
                for (an, aidx) in needed_aliases.drain(..) {
                    if let Some((&an_ref, &atps)) = alias_tparams.get_key_value(an.as_str())
                        && checked_aliases.insert((an, aidx))
                    {
                        pending.push(BoundTarget::Alias(an_ref, aidx, atps));
                    }
                }
            }
            if ok {
                out.insert((c.name.clone(), i), (base, cargs));
            }
        }
    }
    out
}

fn collect_ty_names(t: &Ty, out: &mut HashSet<String>) {
    match t {
        Ty::Obj(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => {
            out.insert(n.clone());
        }
        Ty::Generic(n, args) => {
            out.insert(n.clone());
            for a in args {
                collect_ty_names(a, out);
            }
        }
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => collect_ty_names(inner, out),
        Ty::Tuple(es) | Ty::Union(es) => {
            for e in es {
                collect_ty_names(e, out);
            }
        }
        _ => {}
    }
}

fn reachable_synthetics(seeds: HashSet<String>, type_edges: &HashMap<String, HashSet<String>>, synthetic: &HashSet<String>) -> HashSet<String> {
    let mut visited = HashSet::new();
    let mut todo: Vec<String> = seeds.into_iter().collect();
    let mut out = HashSet::new();
    while let Some(n) = todo.pop() {
        if !visited.insert(n.clone()) {
            continue;
        }
        let is_syn = synthetic.contains(&n);
        if is_syn {
            out.insert(n.clone());
        }
        if (is_syn || type_edges.contains_key(&n))
            && let Some(next) = type_edges.get(&n)
        {
            for tgt in next {
                if !visited.contains(tgt) {
                    todo.push(tgt.clone());
                }
            }
        }
    }
    out
}

pub(super) fn plan_folded_modules(
    m: &Model,
    g: &Gen,
    paths: &HashMap<String, String>,
    pkg: &str,
    class_paths: &HashMap<String, String>,
) -> HashMap<String, (String, String)> {
    let mut has_subclasses: HashSet<String> = g.parents.values().map(|(p, _)| p.clone()).collect();
    for c in &m.classes {
        has_subclasses.extend(secondary(m, c));
    }
    let mut type_edges: HashMap<String, HashSet<String>> = HashMap::new();
    for u in &m.unions {
        let entry = type_edges.entry(u.name.clone()).or_default();
        for p in &u.parts {
            collect_ty_names(p, entry);
        }
        for tp in &u.type_params {
            if let Some(d) = &tp.default {
                collect_ty_names(d, entry);
            }
            if let Some(ct) = &tp.constraint {
                collect_ty_names(ct, entry);
            }
        }
    }
    for cb in &m.callbacks {
        let entry = type_edges.entry(cb.name.clone()).or_default();
        for (_, pt) in &cb.params {
            collect_ty_names(pt, entry);
        }
        collect_ty_names(&cb.ret, entry);
        for tp in &cb.type_params {
            if let Some(d) = &tp.default {
                collect_ty_names(d, entry);
            }
            if let Some(ct) = &tp.constraint {
                collect_ty_names(ct, entry);
            }
        }
    }
    for sc in m.classes.iter().filter(|c| m.synthetic.contains(&c.name)) {
        let entry = type_edges.entry(sc.name.clone()).or_default();
        for f in &sc.funcs {
            for (_, pt) in &f.params {
                collect_ty_names(pt, entry);
            }
            collect_ty_names(&f.ret, entry);
            for tp in &f.type_params {
                if let Some(d) = &tp.default {
                    collect_ty_names(d, entry);
                }
                if let Some(ct) = &tp.constraint {
                    collect_ty_names(ct, entry);
                }
            }
        }
        for tp in &sc.type_params {
            if let Some(d) = &tp.default {
                collect_ty_names(d, entry);
            }
            if let Some(ct) = &tp.constraint {
                collect_ty_names(ct, entry);
            }
        }
    }
    let mut syn_class_users: HashMap<String, HashSet<String>> = HashMap::new();
    for c in m.classes.iter().filter(|c| !m.synthetic.contains(&c.name)) {
        let mut seeds = HashSet::new();
        for b in &c.bases {
            collect_ty_names(b, &mut seeds);
        }
        for t in c.iterates.iter().chain(&c.iterates_async).chain(&c.elements) {
            collect_ty_names(t, &mut seeds);
        }
        for tp in &c.type_params {
            if let Some(d) = &tp.default {
                collect_ty_names(d, &mut seeds);
            }
            if let Some(ct) = &tp.constraint {
                collect_ty_names(ct, &mut seeds);
            }
        }
        for f in &c.funcs {
            for (_, pt) in &f.params {
                collect_ty_names(pt, &mut seeds);
            }
            collect_ty_names(&f.ret, &mut seeds);
            for tp in &f.type_params {
                if let Some(d) = &tp.default {
                    collect_ty_names(d, &mut seeds);
                }
                if let Some(ct) = &tp.constraint {
                    collect_ty_names(ct, &mut seeds);
                }
            }
        }
        for s in reachable_synthetics(seeds, &type_edges, &m.synthetic) {
            syn_class_users.entry(s).or_default().insert(c.name.clone());
        }
    }
    let mut syn_area_users: HashMap<String, HashSet<String>> = HashMap::new();
    for f in &m.funcs {
        let mut seeds = HashSet::new();
        for (_, pt) in &f.params {
            collect_ty_names(pt, &mut seeds);
        }
        collect_ty_names(&f.ret, &mut seeds);
        for tp in &f.type_params {
            if let Some(d) = &tp.default {
                collect_ty_names(d, &mut seeds);
            }
            if let Some(ct) = &tp.constraint {
                collect_ty_names(ct, &mut seeds);
            }
        }
        let unit = func_unit(m, paths, pkg, f);
        for s in reachable_synthetics(seeds, &type_edges, &m.synthetic) {
            syn_area_users.entry(s).or_default().insert(unit.clone());
        }
    }
    let mut folded: HashMap<String, (String, String)> = HashMap::new();
    for c in &m.classes {
        let eligible = m.synthetic.contains(&c.name)
            && c.js.is_none()
            && c.bases.is_empty()
            && !m.maps.iter().any(|k| k.name == c.name)
            && !m.namespaced.contains_key(&c.name)
            && !has_subclasses.contains(&c.name)
            && c.iterates.is_none()
            && c.iterates_async.is_none()
            && c.elements.is_none()
            && c.funcs.len() <= 20;
        if !eligible {
            continue;
        }
        let empty = HashSet::new();
        let class_users = syn_class_users.get(&c.name).unwrap_or(&empty);
        let area_users = syn_area_users.get(&c.name).unwrap_or(&empty);
        if area_users.is_empty() && !class_users.is_empty() {
            if let Some(owner) = class_users.iter().find(|&owner| class_users.iter().all(|u| is_chain_subtype(&g.parents, u, owner))) {
                let cname_snake = snake(&g.tname(&c.name));
                let owner_snake = snake(&g.tname(owner));
                let prefix = if let Some(rem) = c.name.strip_prefix(&format!("{owner}_")).map(|r| r.trim_start_matches('_')).filter(|r| !r.is_empty()) {
                    lower(rem)
                } else if let Some(rem) = cname_snake.strip_prefix(&format!("{owner_snake}_")).filter(|r| !r.is_empty()) {
                    lower(rem)
                } else {
                    lower(&g.tname(&c.name))
                };
                folded.insert(c.name.clone(), (class_paths[owner].clone(), prefix));
            }
        } else if class_users.is_empty() && area_users.len() == 1 {
            let area_unit = area_users.iter().next().unwrap().clone();
            let prefix = lower(&g.tname(&c.name));
            folded.insert(c.name.clone(), (area_unit, prefix));
        }
    }
    folded
}
