// ---- classes ----

use std::collections::{HashMap, HashSet};

use tsrs_ast::Symbol;
use tsrs_core::P;

use crate::front::Front;
use crate::ir::*;

use super::eval::split_members;
use super::walk::{walk, walk_type};

// `interface X` + `var X: { prototype: X; new(): X; statics }` is a class.
pub(super) fn pair_classes(decls: &mut Vec<Decl>) {
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

// A class with no members of its own that extends an interface of its own
// name from a namespace is that interface: node's `events.EventEmitter`
// (a constructor and statics) and the global `NodeJS.EventEmitter`,
// declared apart so a module and the globals can each name it. One type
// has the members, under the class's name; what extended or named the
// interface extends or names the class. A class of its own that a
// namespace's interface adds to (pixi's `Container` and
// `PixiMixins.Container`) stays apart.
pub(super) fn fold_namesake_bases(decls: &mut Vec<Decl>) {
    struct Base {
        name: String,
        params: Vec<Sym>,
        members: Vec<Member>,
        extends: Vec<Type>,
    }
    fn bases(decls: &[Decl], nested: bool, out: &mut HashMap<P<Symbol>, Base>) {
        for d in decls {
            match d {
                Decl::Interface(i) if nested && i.class.is_none() => {
                    if let Some(s) = i.sym {
                        out.insert(s, Base { name: i.name.clone(), params: i.type_params.iter().map(|p| p.sym).collect(), members: i.members.clone(), extends: i.extends.clone() });
                    }
                }
                Decl::Namespace(n) => bases(&n.decls, true, out),
                _ => {}
            }
        }
    }
    let mut found = HashMap::new();
    bases(decls, false, &mut found);
    // Each folded interface's symbol: the class's name, symbol and type parameters.
    let mut folded: HashMap<P<Symbol>, (String, Sym, Vec<TypeParam>)> = HashMap::new();
    fn fold(decls: &mut [Decl], found: &HashMap<P<Symbol>, Base>, folded: &mut HashMap<P<Symbol>, (String, Sym, Vec<TypeParam>)>) {
        for d in decls {
            match d {
                Decl::Interface(c) if c.class.is_some() && c.members.is_empty() => {
                    let Some(pos) = c.extends.iter().position(|e| matches!(e, Type::Ref(r) if r.sym.is_some_and(|s| found.get(&s).is_some_and(|b| b.name == c.name && b.params.len() == c.type_params.len())))) else { continue };
                    let Type::Ref(r) = c.extends.remove(pos) else { continue };
                    let sym = r.sym.unwrap();
                    let base = &found[&sym];
                    // Its type parameters are the class's.
                    let to: HashMap<Sym, &TypeParam> = base.params.iter().copied().zip(&c.type_params).collect();
                    let mut t = Type::Object(base.members.clone());
                    walk_type(&mut t, &mut Vec::new(), &mut |x, _| {
                        if let Type::Ref(p) = x
                            && p.kind == "typeParameter"
                            && let Some(tp) = to.get(&p.sym)
                        {
                            p.name = tp.name.clone();
                            p.sym = tp.sym;
                            p.tid = tp.tid;
                        }
                        true
                    });
                    let Type::Object(members) = t else { unreachable!() };
                    let own: Vec<(String, MemberKind)> = c.members.iter().map(|m| (m.name.clone(), m.kind)).collect();
                    c.members.extend(members.into_iter().filter(|m| !own.contains(&(m.name.clone(), m.kind))));
                    for e in &base.extends {
                        c.extends.insert(pos, e.clone());
                    }
                    folded.insert(sym, (c.name.clone(), c.sym, c.type_params.clone()));
                }
                Decl::Namespace(n) => fold(&mut n.decls, found, folded),
                _ => {}
            }
        }
    }
    fold(decls, &found, &mut folded);
    if folded.is_empty() {
        return;
    }
    fn drop(decls: &mut Vec<Decl>, folded: &HashMap<P<Symbol>, (String, Sym, Vec<TypeParam>)>) {
        decls.retain_mut(|d| match d {
            Decl::Interface(i) => !i.sym.is_some_and(|s| folded.contains_key(&s)),
            Decl::Namespace(n) => {
                drop(&mut n.decls, folded);
                true
            }
            _ => true,
        });
    }
    drop(decls, &folded);
    walk(decls, &mut |t, _| {
        if let Type::Ref(r) = t
            && let Some((name, sym, _)) = r.sym.and_then(|s| folded.get(&s))
        {
            r.name = name.clone();
            r.sym = *sym;
            r.qualified = None;
            r.tid = None;
        }
        true
    });
    fn ancestors(decls: &mut [Decl], folded: &HashMap<P<Symbol>, (String, Sym, Vec<TypeParam>)>) {
        for d in decls {
            match d {
                Decl::Interface(i) => {
                    for k in 0..i.ancestor_syms.len() {
                        if let Some((name, sym, _)) = i.ancestor_syms[k].and_then(|s| folded.get(&s)) {
                            i.ancestor_syms[k] = *sym;
                            if let Some(a) = i.ancestors.get_mut(k) {
                                *a = name.clone();
                            }
                        }
                    }
                    let own = i.sym;
                    let mut seen = HashSet::new();
                    let keep: Vec<bool> = i.ancestor_syms.iter().map(|s| *s != own && seen.insert(*s)).collect();
                    let mut it = keep.iter();
                    i.ancestor_syms.retain(|_| *it.next().unwrap());
                    if i.ancestors.len() == keep.len() {
                        let mut it = keep.iter();
                        i.ancestors.retain(|_| *it.next().unwrap());
                    }
                }
                Decl::Namespace(n) => ancestors(&mut n.decls, folded),
                _ => {}
            }
        }
    }
    ancestors(decls, &folded);
}

// References to a paired interface are references to a class. Runs last:
// later passes bring in references the checker made.
pub(super) fn mark_class_refs(decls: &mut [Decl]) {
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

pub(super) fn flatten(front: &mut Front, decls: &mut [Decl]) {
    for d in decls {
        match d {
            Decl::Interface(i) => {
                if let Some(sym) = i.sym {
                    let (members, ancestors) = front.flattened(sym);
                    i.members = members;
                    if front.expand_keyed || front.map_keyed {
                        split_members(&mut i.members, front.keyed_limit);
                    }
                    i.ancestor_syms = ancestors.iter().map(|a| a.1).collect();
                    i.ancestors = ancestors.into_iter().map(|a| a.0).collect();
                    i.extends.clear();
                }
            }
            Decl::Namespace(n) => flatten(front, &mut n.decls),
            _ => {}
        }
    }
}

// Structural conformance for nominal targets: where a class in the same scope
// has every required property of a pure-property interface (like Three's
// `Vector3` and `Vector3Like`, or Pixi's `Point` and `PointData`), record it
// as an inherited interface so nominal targets allow the conversion.
type ClassShape = (HashMap<String, (bool, bool, Type)>, HashSet<String>, HashSet<String>);

pub(super) fn structural_extends(decls: &mut [Decl], flattened: bool) {
    fn type_eq(a: &Type, b: &Type) -> bool {
        match (a, b) {
            (Type::Primitive(x), Type::Primitive(y)) => x == y,
            (Type::Literal { kind: k1, value: v1 }, Type::Literal { kind: k2, value: v2 }) => k1 == k2 && v1 == v2,
            (Type::Ref(r1), Type::Ref(r2)) => {
                let same = match (r1.sym, r2.sym) {
                    (Some(s1), Some(s2)) => s1 == s2,
                    (None, None) => r1.name == r2.name,
                    _ => false,
                };
                same && r1.args.len() == r2.args.len() && r1.args.iter().zip(&r2.args).all(|(x, y)| type_eq(x, y))
            }
            (Type::Array(x), Type::Array(y)) | (Type::Readonly(x), Type::Readonly(y)) => type_eq(x, y),
            (Type::Nullable(x, ax), Type::Nullable(y, ay)) => ax == ay && type_eq(x, y),
            (Type::This, Type::This) => true,
            _ => false,
        }
    }

    // Recurse into namespaces first.
    for d in decls.iter_mut() {
        if let Decl::Namespace(n) = d {
            structural_extends(&mut n.decls, flattened);
        }
    }

    // Index interfaces in this scope by name.
    let by_name: HashMap<String, usize> = decls
        .iter()
        .enumerate()
        .filter_map(|(idx, d)| if let Decl::Interface(i) = d { Some((i.name.clone(), idx)) } else { None })
        .collect();

    // Candidate pure-property structural target interfaces (at least 2 required properties).
    struct Target {
        name: String,
        sym: Sym,
        req_names: HashSet<String>,
        props: Vec<(String, bool, bool, Type)>, // (name, optional, readonly, ty)
    }
    let mut targets: Vec<Target> = Vec::new();
    for d in decls.iter() {
        let Decl::Interface(i) = d else { continue };
        if i.class.is_some() || i.synthetic || !i.type_params.is_empty() || !i.extends.is_empty() || !i.ancestors.is_empty() || i.members.is_empty() {
            continue;
        }
        if !i.members.iter().all(|m| m.kind == MemberKind::Property && m.ty.is_some()) {
            continue;
        }
        let req_names: HashSet<String> = i.members.iter().filter(|m| !m.optional).map(|m| m.name.clone()).collect();
        if req_names.len() < 2 {
            continue;
        }
        let props = i.members.iter().map(|m| (m.name.clone(), m.optional, m.readonly, m.ty.clone().unwrap())).collect();
        targets.push(Target { name: i.name.clone(), sym: i.sym, req_names, props });
    }
    if targets.is_empty() {
        return;
    }

    // Collect effective properties, methods, and nominal ancestors of each class in this scope.
    let collect_class = |idx: usize| -> ClassShape {
        let mut props: HashMap<String, (bool, bool, Type)> = HashMap::new();
        let mut methods: HashSet<String> = HashSet::new();
        let mut bases: HashSet<String> = HashSet::new();
        let mut todo = vec![idx];
        let mut visited = HashSet::new();
        while let Some(cur) = todo.pop() {
            if !visited.insert(cur) {
                continue;
            }
            let Decl::Interface(i) = &decls[cur] else { continue };
            for m in &i.members {
                match m.kind {
                    MemberKind::Property => {
                        if let Some(ty) = &m.ty {
                            props.entry(m.name.clone()).or_insert_with(|| (m.optional, m.readonly, ty.clone()));
                        }
                    }
                    MemberKind::Method => {
                        methods.insert(m.name.clone());
                    }
                    _ => {}
                }
            }
            if flattened {
                for a in &i.ancestors {
                    bases.insert(a.clone());
                }
            } else {
                for e in &i.extends {
                    if let Type::Ref(r) = e {
                        bases.insert(r.name.clone());
                        if let Some(&bidx) = by_name.get(&r.name) {
                            todo.push(bidx);
                        }
                    }
                }
            }
        }
        (props, methods, bases)
    };

    // First compute maximal matching target indices for every class in this scope.
    let mut matched: HashMap<String, Vec<usize>> = HashMap::new();
    let mut nominal_bases: HashMap<String, HashSet<String>> = HashMap::new();
    for (idx, d) in decls.iter().enumerate() {
        let Decl::Interface(c) = d else { continue };
        if c.class.is_none() {
            continue;
        }
        let (props, methods, bases) = collect_class(idx);
        let mut hits: Vec<usize> = Vec::new();
        for (ti, t) in targets.iter().enumerate() {
            if t.name == c.name || bases.contains(&t.name) {
                continue;
            }
            let ok = t.props.iter().all(|(pname, popt, pro, pty)| {
                if methods.contains(pname) {
                    return false;
                }
                match props.get(pname) {
                    Some((copt, cro, cty)) => {
                        if !*popt && *copt {
                            return false;
                        }
                        (*pro || !*cro) && type_eq(cty, pty)
                    }
                    None => *popt,
                }
            });
            if ok {
                hits.push(ti);
            }
        }
        if hits.is_empty() {
            continue;
        }
        // Keep only maximal structural interfaces (drop any whose required set is a strict subset of another's).
        let keep: Vec<usize> = hits
            .iter()
            .copied()
            .filter(|&a| !hits.iter().any(|&b| b != a && targets[a].req_names.len() < targets[b].req_names.len() && targets[a].req_names.is_subset(&targets[b].req_names)))
            .collect();
        let mut keep = keep;
        // Prefer an interface named after this class (`Vector3Like` for `Vector3`, `QuaternionLike` for `Quaternion`).
        keep.sort_by_key(|&ti| (!targets[ti].name.starts_with(&c.name), targets[ti].name.clone()));
        nominal_bases.insert(c.name.clone(), bases);
        matched.insert(c.name.clone(), keep);
    }

    // Apply structural extends, skipping any target already inherited via a nominal base class.
    for d in decls.iter_mut() {
        let Decl::Interface(c) = d else { continue };
        let Some(hits) = matched.get(&c.name) else { continue };
        let bases = &nominal_bases[&c.name];
        for &ti in hits {
            if !flattened && bases.iter().any(|b| matched.get(b).is_some_and(|bh| bh.contains(&ti))) {
                continue;
            }
            let t = &targets[ti];
            if flattened {
                if !c.ancestors.contains(&t.name) {
                    c.ancestors.push(t.name.clone());
                    c.ancestor_syms.push(t.sym);
                }
            } else {
                c.extends.push(Type::Ref(Ref {
                    name: t.name.clone(),
                    kind: "interface",
                    sym: t.sym,
                    args: Vec::new(),
                    tid: None,
                    qualified: None,
                    file: None,
                    computed: false,
                    node: None,
                }));
            }
        }
    }
}
