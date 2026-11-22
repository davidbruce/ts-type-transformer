// ---- unions ----

use std::collections::{HashMap, HashSet};

use crate::front::Front;
use crate::ir::*;
use crate::xml::type_key;

use super::generics::{SYNTH_NAME_MAX, unique_with_limit};
use super::hoist::{collect_bodies, compact_hoist_path, free_type_params};
use super::overloads::{describe, mnemonic};
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

pub(super) fn name_unions(decls: &mut Vec<Decl>, sep: &str, written: &HashSet<String>) {
    let mut names: HashSet<String> = HashSet::new();
    collect_declared_names(decls, "", sep, &mut names);
    // A union that a declared alias also is uses that name, but one the library
    // wrote as a union there (`string | HTMLElement` beside `type Content =
    // string | HTMLElement`) is named by its members, if a few say what it is.
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
        name_unions_once(decls, sep, &mut names, &mut by_key, &declared, written, &mut free_of);
        if decls.len() == before {
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

fn name_unions_once(decls: &mut Vec<Decl>, sep: &str, names: &mut HashSet<String>, by_key: &mut HashMap<String, String>, declared: &HashSet<String>, written: &HashSet<String>, free_of: &mut HashMap<String, Vec<Ref>>) {
    let mut bodies: HashSet<*const Type> = HashSet::new();
    collect_bodies(decls, &mut bodies);
    let mut added = Vec::new();
    walk(decls, &mut |t, path| {
        if !matches!(t, Type::Union(_)) || bodies.contains(&(t as *const Type)) {
            return true;
        }
        let Type::Union(parts) = &*t else { unreachable!() };
        let key = union_key(t);
        let alone = format!("written:{key}");
        let short = |names: &HashSet<String>| {
            let joined = parts.iter().map(describe).collect::<Option<Vec<_>>>().map(|ps| ps.join(&format!("{sep}or{sep}")));
            joined.is_some_and(|j| parts.len() <= 3 && j.len() <= SYNTH_NAME_MAX && !names.contains(&j))
        };
        let own = declared.contains(&key) && written.contains(&path.join("\u{1}")) && (by_key.contains_key(&alone) || short(names));
        let name = by_key
            .entry(if own { alone } else { key })
            .or_insert_with(|| {
                // By its members, if a few that say what they are; else where it is.
                let joined = parts.iter().map(describe).collect::<Option<Vec<_>>>().map(|ps| ps.join(&format!("{sep}or{sep}")));
                // Another's (F#'s `string | (a) => b` beside `string | (c) => d`,
                // both `string_or_fn`): where it is; a place another has, with what
                // else it holds (`string | FileHandle_readFile_options`).
                let at = compact_hoist_path(path, sep);
                let or_rest = || -> Option<String> {
                    let rest: Vec<String> = parts.iter().filter_map(describe).filter(|d| !d.starts_with(&at)).collect();
                    (!rest.is_empty() && rest.len() < parts.len()).then(|| format!("{at}{sep}or{sep}{}", rest.join(&format!("{sep}or{sep}"))))
                };
                let base = match joined {
                    Some(j) if parts.len() <= 3 && j.len() <= SYNTH_NAME_MAX && !names.contains(&j) => j,
                    _ if !path.is_empty() && !names.contains(&at) => at,
                    // Of the types of its place alone: what it is.
                    _ if !path.is_empty() => or_rest().filter(|n| !names.contains(n)).unwrap_or_else(|| format!("{at}{sep}union")),
                    j => j.unwrap_or_else(|| parts.iter().map(mnemonic).collect::<Vec<_>>().join(&format!("{sep}or{sep}"))),
                };
                let name = unique_with_limit(base, SYNTH_NAME_MAX, names);
                // Type parameters the union mentions become its own.
                let free = free_type_params(t);
                let type_params = free.iter().map(|r| TypeParam { name: r.name.clone(), sym: r.sym, tid: r.tid, constraint: None, default: None }).collect();
                free_of.insert(name.clone(), free);
                added.push(Decl::Alias(Alias { name: name.clone(), sym: None, type_params, extends: Vec::new(), ty: t.clone(), computed: false, synthetic: true, doc: None }));
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
    let mut enum_kinds = HashMap::new();
    enums(decls, &mut enum_kinds);
    let mut iface_shapes = HashMap::new();
    collect_iface_shapes(decls, &[], &mut iface_shapes);
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
                    Type::Ref(r) if matches!(r.kind, "interface" | "class") && r.args.is_empty() => {
                        r.qualified.as_ref().and_then(|q| iface_shapes.get(q)).or_else(|| iface_shapes.get(&r.name)).cloned()
                    }
                    _ => None,
                })
            })
            .collect();
        let (discriminant, tests) = match discriminant_tests(&shapes) {
            Some((d, tests)) => (Some(d), tests),
            None => (
                None,
                parts
                    .iter()
                    .enumerate()
                    .map(|(i, p)| enum_test(p).or_else(|| front.runtime_test(p)).or_else(|| unique_property(&shapes, i).map(Test::Has)))
                    .collect(),
            ),
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
pub(super) fn unique_property(shapes: &[Option<PropShapes>], i: usize) -> Option<String> {
    let own = shapes[i].as_ref()?;
    own.iter()
        .filter(|f| !f.1)
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
