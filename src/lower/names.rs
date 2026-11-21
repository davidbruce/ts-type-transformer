// ---- names ----

use std::collections::HashSet;

use crate::ir::*;

use super::generics::unique;
use super::walk::{each_member_list, walk};

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

// Generated types named in PascalCase, as a target names types:
// `boolean_or_AddEventListenerOptions` is `BooleanOrAddEventListenerOptions`,
// `Shape_kind` is `ShapeKind`. Apart from every name declared.
pub fn pascal_types(decls: &mut [Decl]) {
    fn collect(decls: &[Decl], all: &mut HashSet<String>, generated: &mut Vec<String>) {
        for d in decls {
            let (name, synthetic) = match d {
                Decl::Interface(i) => (&i.name, i.synthetic),
                Decl::Alias(a) => (&a.name, a.synthetic),
                Decl::Enum(e) => (&e.name, e.synthetic),
                Decl::Function(f) => (&f.name, false),
                Decl::Var(v) => (&v.name, false),
                Decl::Namespace(n) => {
                    all.insert(n.name.clone());
                    collect(&n.decls, all, generated);
                    continue;
                }
            };
            all.insert(name.clone());
            if synthetic && !generated.contains(name) {
                generated.push(name.clone());
            }
        }
    }
    let (mut taken, mut generated) = (HashSet::new(), Vec::new());
    collect(decls, &mut taken, &mut generated);
    let mut renames = Vec::new();
    for old in generated {
        let pascal = crate::generators::case::pascal_underscore(&old);
        if pascal == old || pascal.is_empty() {
            continue;
        }
        let new = unique(pascal, &mut taken);
        renames.push((old, new));
    }
    rename_everywhere(decls, &renames);
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
