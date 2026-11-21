// ---- traversal ----

use crate::ir::*;

use super::overloads::name_part;
use super::unions::{PropShapes, unique_property};

type TypeFn<'a> = dyn FnMut(&mut Type, &[String]) -> bool + 'a;

// Visits every type in `t`, pre-order; returning false skips its children.
pub fn visit(t: &mut Type, f: &mut dyn FnMut(&mut Type) -> bool) {
    walk_type(t, &mut Vec::new(), &mut |t, _| f(t));
}

// Visits every type position, pre-order, with the naming path to it.
// The callback may replace the type; returning false skips its children.
pub(super) fn walk(decls: &mut [Decl], f: &mut TypeFn) {
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
                for e in &mut a.extends {
                    walk_type(e, path, f);
                }
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

pub(super) fn walk_members(members: &mut [Member], path: &mut Vec<String>, f: &mut TypeFn) {
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

pub(super) fn walk_signature(s: &mut Signature, path: &mut Vec<String>, f: &mut TypeFn) {
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

pub(super) fn walk_type(t: &mut Type, path: &mut Vec<String>, f: &mut TypeFn) {
    if !f(t, path) {
        return;
    }
    match t {
        Type::Primitive(_) | Type::Erased(_) | Type::Literal { .. } | Type::This => {}
        Type::Ref(r) => r.args.iter_mut().for_each(|a| walk_type(a, path, f)),
        Type::Union(ts) => {
            let segs = arm_names(ts);
            for (a, seg) in ts.iter_mut().zip(segs) {
                if let Some(seg) = &seg {
                    path.push(seg.clone());
                }
                walk_type(a, path, f);
                if seg.is_some() {
                    path.pop();
                }
            }
        }
        Type::Intersection(ts, _) => ts.iter_mut().for_each(|a| walk_type(a, path, f)),
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

// A union's inline objects, each named apart from the others: by the value
// of the property that tags them (katex's `TrustContext` of `command:
// "\\href"` is `TrustContext_href`), where it has one; else by a property
// only it has. Those still alike are named apart where hoisted.
fn arm_names(parts: &[Type]) -> Vec<Option<String>> {
    let objects: Vec<Option<&Vec<Member>>> = parts.iter().map(|p| if let Type::Object(ms) = p { Some(ms) } else { None }).collect();
    if objects.iter().flatten().count() < 2 {
        return vec![None; parts.len()];
    }
    let props = |ms: &Vec<Member>| ms.iter().filter(|m| m.kind == MemberKind::Property).map(|m| (m.name.clone(), m.optional, m.constant.clone())).collect::<Vec<_>>();
    let shapes: Vec<Option<PropShapes>> = objects.iter().map(|o| o.map(props)).collect();
    // The property most of them fix, if two do.
    let mut fixed: Vec<(String, usize)> = Vec::new();
    for f in shapes.iter().flatten().flatten().filter(|f| f.2.is_some()) {
        match fixed.iter_mut().find(|(n, _)| *n == f.0) {
            Some(e) => e.1 += 1,
            None => fixed.push((f.0.clone(), 1)),
        }
    }
    let tag = fixed.iter().filter(|f| f.1 >= 2).max_by_key(|f| f.1).map(|f| f.0.clone());
    (0..parts.len())
        .map(|i| {
            let own = shapes[i].as_ref()?;
            let tagged = tag.as_ref().and_then(|t| own.iter().find(|g| &g.0 == t)).and_then(|g| g.2.as_deref()).map(|v| name_part(v).trim_matches('_').to_string()).filter(|v| !v.is_empty());
            tagged.or_else(|| unique_property(&shapes, i))
        })
        .collect()
}

// Visits every list of members: interfaces, class statics and constructors, object types.
pub(super) fn each_member_list(decls: &mut [Decl], f: &mut dyn FnMut(&mut Vec<Member>)) {
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
