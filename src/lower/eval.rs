// ---- type-level expressions ----

use std::collections::{HashMap, HashSet};

use tsrs_ast::Symbol;
use tsrs_core::P;

use crate::front::Front;
use crate::ir::*;
use crate::xml::type_key;

use super::generics::{each_signature, for_each_type_params, keyed_map, keyed_syms};
use super::Options;
use super::walk::{each_member_list, walk, walk_signature, walk_type};

pub(super) fn evaluate(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    if o.typeops {
        drop_type_functions(decls);
    }
    // An alias body is evaluated without folding back into the alias itself.
    // Where an intersection alias (`type RectConfig = ShapeConfig & { ... }`)
    // evaluates to an object, keep its named object arms as bases so nominal
    // targets know `RectConfig` extends `ShapeConfig`.
    for_each_alias(decls, &mut |a| {
        let arms: Vec<Type> = match &a.ty {
            Type::Intersection(parts, _) if o.intersections => parts
                .iter()
                .filter_map(|p| match p {
                    Type::Ref(r) if !r.computed && r.file.is_none() && r.args.is_empty() && matches!(r.kind, "interface" | "typeAlias") && r.name != a.name => Some(Type::Ref(r.clone())),
                    _ => None,
                })
                .collect(),
            _ => Vec::new(),
        };
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
        if matches!(a.ty, Type::Object(_)) && !arms.is_empty() {
            a.extends = arms;
        }
    });
    // Keep only intersection bases that refer to object-shaped declarations
    // (interfaces or object-valued aliases) without cycles.
    fn collect_object_syms(decls: &[Decl], out: &mut HashSet<P<Symbol>>) {
        for d in decls {
            match d {
                Decl::Interface(i) if i.type_params.is_empty() => {
                    out.extend(i.sym);
                }
                Decl::Alias(a) if a.type_params.is_empty() && matches!(a.ty, Type::Object(_)) => {
                    out.extend(a.sym);
                }
                Decl::Namespace(n) => collect_object_syms(&n.decls, out),
                _ => {}
            }
        }
    }
    let mut obj_syms = HashSet::new();
    collect_object_syms(decls, &mut obj_syms);
    for_each_alias(decls, &mut |a| {
        if !a.extends.is_empty() {
            let mut seen = HashSet::new();
            a.extends.retain(|e| {
                let Type::Ref(r) = e else { return false };
                r.sym.is_some_and(|s| obj_syms.contains(&s) && Some(s) != a.sym && seen.insert(s))
            });
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
pub(super) fn expand_keyed(front: &mut Front, decls: &mut Vec<Decl>, o: Options) {
    let mut expanded = 0;
    // With `map`, the maps that are kept: those declared here.
    let kept: Option<HashSet<P<Symbol>>> = (!o.keyed).then(|| {
        let mut syms = HashSet::new();
        fn collect(decls: &[Decl], out: &mut HashSet<P<Symbol>>) {
            for d in decls {
                match d {
                    Decl::Interface(i) => out.extend(i.sym),
                    Decl::Namespace(n) => collect(&n.decls, out),
                    _ => {}
                }
            }
        }
        collect(decls, &mut syms);
        syms
    });
    let kept = kept.as_ref();
    each_member_list(decls, &mut |members| {
        let mut out = Vec::new();
        for m in members.drain(..) {
            match m.sig.as_ref().and_then(|s| keyed_variants(front, s, o.keyed_limit, kept)) {
                Some(variants) => {
                    expanded += 1;
                    out.extend(variants.into_iter().map(|s| Member { sig: Some(s), ..m.clone() }));
                }
                None => out.push(m),
            }
        }
        *members = out;
    });
    expand_keyed_functions(front, decls, o.keyed_limit, kept, &mut expanded);
    *front.stats.entry("expanded: keyed signature".into()).or_default() += expanded;
}

fn expand_keyed_functions(front: &mut Front, decls: &mut Vec<Decl>, limit: usize, kept: Option<&HashSet<P<Symbol>>>, expanded: &mut usize) {
    let mut out = Vec::new();
    for d in decls.drain(..) {
        match d {
            Decl::Function(f) => match keyed_variants(front, &f.sig, limit, kept) {
                Some(variants) => {
                    *expanded += 1;
                    out.extend(variants.into_iter().map(|sig| Decl::Function(Function { name: f.name.clone(), js_name: f.js_name.clone(), sig, doc: f.doc.clone() })));
                }
                None => out.push(Decl::Function(f)),
            },
            Decl::Namespace(mut n) => {
                expand_keyed_functions(front, &mut n.decls, limit, kept, expanded);
                out.push(Decl::Namespace(n));
            }
            d => out.push(d),
        }
    }
    *decls = out;
}

// Overloads told apart by a parameter of string literals are keyed signatures
// written by hand: `on(type: 'click' | 'dblclick', fn)` beside `on(type:
// 'load', fn)` is one overload per literal, the literal a fixed argument.
pub(super) fn split_literal_overloads(decls: &mut Vec<Decl>, limit: usize) {
    each_member_list(decls, &mut |members| split_members(members, limit));
    split_functions(decls, limit);
}

pub(super) fn split_members(members: &mut Vec<Member>, limit: usize) {
    let mut out: Vec<Member> = Vec::new();
    let mut done: HashSet<String> = HashSet::new();
    for m in members.iter() {
        if m.kind != MemberKind::Method || m.sig.is_none() {
            out.push(m.clone());
        } else if done.insert(m.name.clone()) {
            let group: Vec<&Member> = members.iter().filter(|g| g.kind == MemberKind::Method && g.sig.is_some() && g.name == m.name).collect();
            if group.len() < 2 {
                out.push(m.clone());
                continue;
            }
            let sigs: Vec<Signature> = group.iter().filter_map(|g| g.sig.clone()).collect();
            out.extend(split_group(&sigs, limit).into_iter().map(|(n, sig)| Member { sig: Some(sig), ..group[n].clone() }));
        }
    }
    *members = out;
}

fn string_literals(t: &Type) -> Option<Vec<String>> {
    match t {
        Type::Literal { kind: "string", value } => Some(vec![value.clone()]),
        Type::Union(parts) => parts.iter().map(|p| if let Type::Literal { kind: "string", value } = p { Some(value.clone()) } else { None }).collect(),
        _ => None,
    }
}

// The position whose literals tell some of `sigs` apart, if any.
fn discriminating(sigs: &[&Signature], from: usize) -> Option<usize> {
    let width = sigs.iter().map(|s| s.params.len()).max()?;
    (from..width).find(|&i| {
        let at: Vec<&Param> = sigs.iter().filter_map(|s| s.params.get(i)).filter(|p| !p.rest).collect();
        let keyed = at.iter().filter(|p| p.constant.is_none() && string_literals(&p.ty).is_some()).count();
        let kinds: HashSet<String> = at.iter().map(|p| format!("{:?}{}", p.constant, type_key(&p.ty))).collect();
        keyed >= 2 && kinds.len() > 1
    })
}

fn split_at(sig: &Signature, i: usize, limit: usize) -> Vec<Signature> {
    match sig.params.get(i).filter(|p| p.constant.is_none() && !p.rest).and_then(|p| string_literals(&p.ty)).filter(|k| k.len() <= limit) {
        Some(keys) => keys
            .into_iter()
            .map(|key| {
                let mut s = sig.clone();
                s.params[i].constant = Some(key);
                s.params[i].ty = Type::prim("string");
                s
            })
            .collect(),
        None => vec![sig.clone()],
    }
}

// Each result with the index of the overload it came from. Splitting at one
// position can leave overloads another position tells apart.
fn split_group(sigs: &[Signature], limit: usize) -> Vec<(usize, Signature)> {
    let mut out: Vec<(usize, Signature)> = sigs.iter().cloned().enumerate().collect();
    let mut from = 0;
    while let Some(i) = discriminating(&out.iter().map(|(_, s)| s).collect::<Vec<_>>(), from) {
        out = out.iter().flat_map(|(n, s)| split_at(s, i, limit).into_iter().map(move |s| (*n, s))).collect();
        from = i + 1;
    }
    out
}

fn split_functions(decls: &mut Vec<Decl>, limit: usize) {
    let mut sigs: HashMap<String, Vec<(Option<String>, Signature, Option<String>)>> = HashMap::new();
    for d in decls.iter() {
        if let Decl::Function(f) = d {
            sigs.entry(f.name.clone()).or_default().push((f.js_name.clone(), f.sig.clone(), f.doc.clone()));
        }
    }
    let mut out: Vec<Decl> = Vec::new();
    for d in decls.drain(..) {
        match d {
            Decl::Function(f) => {
                let Some(group) = sigs.remove(&f.name) else { continue };
                if group.len() < 2 {
                    out.push(Decl::Function(f));
                    continue;
                }
                let only: Vec<Signature> = group.iter().map(|g| g.1.clone()).collect();
                out.extend(split_group(&only, limit).into_iter().map(|(n, sig)| Decl::Function(Function { name: f.name.clone(), js_name: group[n].0.clone(), sig, doc: group[n].2.clone() })));
            }
            Decl::Namespace(mut n) => {
                split_functions(&mut n.decls, limit);
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
pub(super) fn drop_unreachable_overloads(decls: &mut Vec<Decl>) {
    // With its constraints: `wrap<T>(v: T)` after `wrap<T extends Named>(v: T)`
    // is reached by what is no Named.
    let call_key = |name: &str, s: &Signature| {
        let params: Vec<String> = s.params.iter().map(|p| format!("{:?}{}{}{}", p.constant, p.optional, p.rest, type_key(&p.ty))).collect();
        let tps: Vec<String> = s.type_params.iter().map(|t| t.constraint.as_ref().map(type_key).unwrap_or_default()).collect();
        format!("{name}<{}>({})", tps.join(","), params.join(","))
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

// `kept`: with `keyed_params = "map"`, the maps that stay; only a signature
// over another map is expanded.
fn keyed_variants(front: &mut Front, sig: &Signature, limit: usize, kept: Option<&HashSet<P<Symbol>>>) -> Option<Vec<Signature>> {
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
                    Some(s) if kept.is_some_and(|k| k.contains(&s)) => continue,
                    Some(s) => front.keys_of(s),
                    None => continue,
                },
                _ => continue,
            },
            // With `map`, keys written out stay as they are.
            Some(_) if kept.is_some() => continue,
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
pub(super) fn splice_union_aliases(decls: &mut [Decl]) {
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
pub(super) fn inline_object_bases(front: &mut Front, decls: &mut [Decl]) {
    // A base that is an alias, by what the alias lowered to: an object, a
    // class or interface (`type _Worker = Worker`), or nothing.
    // Those in namespaces by their path: `NS.Options`.
    fn collect(decls: &[Decl], prefix: &str, out: &mut HashMap<String, (Vec<TypeParam>, Type)>) {
        for d in decls {
            match d {
                Decl::Alias(a)
                    if a.type_params.is_empty() && matches!(a.ty, Type::Erased(_) | Type::Primitive(_))
                        || matches!(&a.ty, Type::Ref(r) if matches!(r.kind, "class" | "interface")) =>
                {
                    out.insert(format!("{prefix}{}", a.name), (a.type_params.clone(), a.ty.clone()));
                }
                Decl::Namespace(n) => collect(&n.decls, &format!("{prefix}{}.", n.name), out),
                _ => {}
            }
        }
    }
    let mut aliases = HashMap::new();
    collect(decls, "", &mut aliases);
    inline_object_bases_with(front, decls, &aliases);
}

// `constraints = "nominal"`: a declaration's type parameter keeps its
// constraint only where every use in the output satisfies it by name: a type
// declared to extend it, a type parameter so constrained, a member of a union
// it is. TypeScript checks by shape; a target that checks by name (Rust's
// `T: Is<Base>`, C++'s `ts::extends`) would reject a use TypeScript accepts.
// With `exact`, a type parameter meets only the constraint it has: Rust's
// bounds do not follow bases (`T: Is<Readable>` is not `T: Is<Stream>`).
// Dropping one can break another that relied on it: until settled.
pub(super) fn nominal_constraints(front: &mut Front, decls: &mut [Decl], exact: bool) {
    fn key(r: &Ref) -> &str {
        r.qualified.as_deref().unwrap_or(&r.name)
    }
    // Each declared type's bases, generic declarations, and union aliases.
    fn collect(decls: &[Decl], prefix: &str, f: &mut Facts, generic: &mut HashMap<String, Vec<Option<Type>>>) {
        for d in decls {
            match d {
                Decl::Interface(i) => {
                    let name = format!("{prefix}{}", i.name);
                    // A map of keys to types: `M[K]` is one of its values.
                    f.values.insert(name.clone(), i.members.iter().filter(|m| m.kind == MemberKind::Property).filter_map(|m| m.ty.clone()).collect());
                    let mut bs: Vec<String> = i.extends.iter().filter_map(|e| if let Type::Ref(r) = e { Some(key(r).to_string()) } else { None }).collect();
                    bs.extend(i.ancestors.iter().cloned());
                    f.bases.insert(name.clone(), bs);
                    f.base_refs.insert(name.clone(), i.extends.iter().filter_map(|e| if let Type::Ref(r) = e { Some(r.clone()) } else { None }).collect());
                    for p in &i.type_params {
                        f.named_params.insert(format!("{name}.{}", p.name), p.constraint.clone());
                    }
                    if i.type_params.iter().any(|p| p.constraint.is_some()) {
                        generic.insert(name, i.type_params.iter().map(|p| p.constraint.clone()).collect());
                    }
                }
                Decl::Alias(a) => {
                    let name = format!("{prefix}{}", a.name);
                    if !a.extends.is_empty() {
                        let bs: Vec<String> = a.extends.iter().filter_map(|e| if let Type::Ref(r) = e { Some(key(r).to_string()) } else { None }).collect();
                        f.bases.insert(name.clone(), bs);
                        f.base_refs.insert(name.clone(), a.extends.iter().filter_map(|e| if let Type::Ref(r) = e { Some(r.clone()) } else { None }).collect());
                    }
                    if let Type::Union(parts) = &a.ty {
                        f.unions.insert(name.clone(), parts.clone());
                    }
                    for p in &a.type_params {
                        f.named_params.insert(format!("{name}.{}", p.name), p.constraint.clone());
                    }
                    if a.type_params.iter().any(|p| p.constraint.is_some()) {
                        generic.insert(name, a.type_params.iter().map(|p| p.constraint.clone()).collect());
                    }
                }
                Decl::Namespace(n) => collect(&n.decls, &format!("{prefix}{}.", n.name), f, generic),
                _ => {}
            }
        }
    }
    #[derive(Default)]
    struct Facts {
        bases: HashMap<String, Vec<String>>,
        // The same, as written: with their arguments.
        base_refs: HashMap<String, Vec<Ref>>,
        unions: HashMap<String, Vec<Type>>,
        values: HashMap<String, Vec<Type>>,
        params: HashMap<P<Symbol>, Option<Type>>,
        // A declaration's own, by qualified name: `NodeListOf.TNode`.
        named_params: HashMap<String, Option<Type>>,
        exact: bool,
    }
    fn extends(f: &Facts, t: &str, c: &str) -> bool {
        let mut todo = vec![t.to_string()];
        let mut seen = HashSet::new();
        while let Some(x) = todo.pop() {
            if x == c {
                return true;
            }
            if seen.insert(x.clone()) {
                todo.extend(f.bases.get(&x).cloned().unwrap_or_default());
            }
        }
        false
    }
    // Whether `a` is, or extends, `c` as written: a generic constraint
    // (`AbstractRenderData<GraphicsBatcher>`) is met by that instantiation, as
    // a target tells them apart, not by another (`<SmoothBatcher>`).
    fn extends_ref(f: &Facts, a: &Ref, c: &Ref) -> bool {
        if c.args.is_empty() {
            return extends(f, key(a), key(c));
        }
        let same = |b: &Ref| key(b) == key(c) && b.args.len() == c.args.len() && b.args.iter().zip(&c.args).all(|(x, y)| type_key(x) == type_key(y));
        if same(a) {
            return true;
        }
        let mut todo = vec![key(a).to_string()];
        let mut seen = HashSet::new();
        while let Some(x) = todo.pop() {
            if !seen.insert(x.clone()) {
                continue;
            }
            for b in f.base_refs.get(&x).into_iter().flatten() {
                if same(b) {
                    return true;
                }
                todo.push(key(b).to_string());
            }
        }
        false
    }
    fn satisfies(f: &Facts, arg: &Type, c: &Type, depth: usize) -> bool {
        if depth > 8 {
            return false;
        }
        if type_key(arg) == type_key(c) || matches!(c, Type::Primitive(p) if p == "any" || p == "unknown") || matches!(c, Type::Erased(_)) {
            return true;
        }
        let parts = match c {
            Type::Union(ps) => Some(ps.clone()),
            Type::Ref(r) if r.kind == "typeAlias" => f.unions.get(key(r)).cloned(),
            _ => None,
        };
        if let Some(ps) = parts {
            return ps.iter().any(|p| satisfies(f, arg, p, depth + 1));
        }
        match arg {
            Type::Union(ps) => ps.iter().all(|p| satisfies(f, p, c, depth + 1)),
            Type::Op(op, _) => match &**op {
                Op::IndexedAccess(Type::Ref(m), _) => f.values.get(key(m)).is_some_and(|vs| !vs.is_empty() && vs.iter().all(|v| satisfies(f, v, c, depth + 1))),
                _ => false,
            },
            Type::Ref(r) if r.kind == "typeParameter" => {
                let own = r.sym.and_then(|s| f.params.get(&s).cloned().flatten());
                let pc = own.or_else(|| r.qualified.as_ref().and_then(|q| f.named_params.get(q).cloned().flatten()));
                pc.is_some_and(|pc| if f.exact { type_key(&pc) == type_key(c) } else { satisfies(f, &pc, c, depth + 1) })
            }
            // A named union: each of its members.
            Type::Ref(r) if r.kind == "typeAlias" && f.unions.contains_key(key(r)) => f.unions[key(r)].iter().all(|p| satisfies(f, p, c, depth + 1)),
            Type::Ref(a) if matches!(a.kind, "class" | "interface") => matches!(c, Type::Ref(b) if matches!(b.kind, "class" | "interface") && extends_ref(f, a, b)),
            _ => false,
        }
    }
    let mut dropped = 0;
    loop {
        let mut facts = Facts { exact, ..Facts::default() };
        let mut generic = HashMap::new();
        collect(decls, "", &mut facts, &mut generic);
        for_each_type_params(decls, true, &mut |ps| {
            for p in ps.iter() {
                if let Some(s) = p.sym {
                    facts.params.insert(s, p.constraint.clone());
                }
            }
        });
        let mut broken: HashSet<(String, usize)> = HashSet::new();
        walk(decls, &mut |t, _| {
            if let Type::Ref(r) = t
                && let Some(cs) = generic.get(key(r))
            {
                for (i, arg) in r.args.iter().enumerate() {
                    if let Some(Some(c)) = cs.get(i)
                        && !satisfies(&facts, arg, c, 0)
                    {
                        broken.insert((key(r).to_string(), i));
                    }
                }
            }
            true
        });
        // A parameter's own default is a use of it: three's `Points<TGeometry
        // extends BufferGeometry<NormalOrGLBufferAttributes> = BufferGeometry>`
        // must make `Points<>`.
        fn defaults(decls: &[Decl], prefix: &str, f: &Facts, broken: &mut HashSet<(String, usize)>) {
            for d in decls {
                let (name, ps) = match d {
                    Decl::Interface(i) => (format!("{prefix}{}", i.name), &i.type_params),
                    Decl::Alias(a) => (format!("{prefix}{}", a.name), &a.type_params),
                    Decl::Namespace(ns) => {
                        defaults(&ns.decls, &format!("{prefix}{}.", ns.name), f, broken);
                        continue;
                    }
                    _ => continue,
                };
                for (i, p) in ps.iter().enumerate() {
                    if let (Some(d), Some(c)) = (&p.default, &p.constraint)
                        && !satisfies(f, d, c, 0)
                    {
                        broken.insert((name.clone(), i));
                    }
                }
            }
        }
        defaults(decls, "", &facts, &mut broken);
        if broken.is_empty() {
            break;
        }
        fn drop(decls: &mut [Decl], prefix: &str, broken: &HashSet<(String, usize)>, n: &mut usize) {
            for d in decls {
                let (name, ps) = match d {
                    Decl::Interface(i) => (format!("{prefix}{}", i.name), &mut i.type_params),
                    Decl::Alias(a) => (format!("{prefix}{}", a.name), &mut a.type_params),
                    Decl::Namespace(ns) => {
                        drop(&mut ns.decls, &format!("{prefix}{}.", ns.name), broken, n);
                        continue;
                    }
                    _ => continue,
                };
                for (i, p) in ps.iter_mut().enumerate() {
                    if p.constraint.is_some() && broken.contains(&(name.clone(), i)) {
                        p.constraint = None;
                        *n += 1;
                    }
                }
            }
        }
        drop(decls, "", &broken, &mut dropped);
    }
    *front.stats.entry("dropped: constraint not met by name".into()).or_default() += dropped;
}

// `aliases = "expand"`: `List<number>`, where `type List<T> = T[]`, is
// `number[]`. Not an alias of an object, a function or a union, which are
// declarations generators keep generic; nor one that refers to itself. The
// aliases expanded are gone.
pub(super) fn expand_aliases(front: &mut Front, decls: &mut Vec<Decl>) {
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
                        Type::Ref(r) if r.kind == "typeAlias" => match aliases.get(r.qualified.as_ref().unwrap_or(&r.name)) {
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
