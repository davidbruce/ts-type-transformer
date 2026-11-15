// Identifiers, each once in its scope, as a target spells them.
//
// The lowering names declarations apart in the XML. A target then spells
// them its own way: `foo_bar` and `fooBar` are one `foo_bar` in snake case,
// `Shape.area` is the C function `Shape_area` beside any type of that name, a
// namespace's `Node` flattens to `ns_Node`. `settle` renames the model until
// every identifier a target makes of it is unique in its scope: what the
// library declares first, then functions, then what lowering made up, so a
// name made up gives way. The glue and the generator both write from the
// renamed model, so they agree.

use std::collections::{HashMap, HashSet};

use super::model::{Class, Func, Model, TParam, Ty};

// One scope's identifiers.
#[derive(Clone, Default)]
pub struct Scope {
    taken: HashSet<String>,
}

impl Scope {
    pub fn with<'a>(reserved: impl IntoIterator<Item = &'a str>) -> Self {
        Scope { taken: reserved.into_iter().map(str::to_string).collect() }
    }

    pub fn free(&self, names: &[String]) -> bool {
        names.iter().all(|n| !self.taken.contains(n)) && names.iter().enumerate().all(|(i, n)| !names[..i].contains(n))
    }

    pub fn take(&mut self, names: Vec<String>) {
        self.taken.extend(names);
    }

    // `base`, or `base_2`, `base_3`, ...: the first free one, taken.
    pub fn fresh(&mut self, base: &str) -> String {
        let mut name = base.to_string();
        let mut n = 2;
        while self.taken.contains(&name) {
            name = format!("{base}_{n}");
            n += 1;
        }
        self.taken.insert(name.clone());
        name
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Kind {
    Class,
    Callback,
    Union,
    Enum,
    NumEnum,
    Opaque,
}

// How a target makes identifiers of the model's names.
pub struct Naming {
    // What the target has already: keywords, its runtime's names.
    pub reserved: fn() -> Vec<String>,
    // The identifiers a type makes if named `candidate`, the type's own
    // first (C's `Shape`, `AS_Shape`, `Shape_from_handle`; Rust's `Shape`).
    // `name` is its name in the model, to find it by.
    pub type_names: fn(candidate: &str, name: &str, Kind, &Model) -> Vec<String>,
    // The functions and constants a type makes, given its identifier
    // (Gleam's `shape_as_named`, `shape_to_list`), where they are apart.
    pub values_of: fn(ident: &str, name: &str, Kind, &Model) -> Vec<String>,
    // Types and functions are in separate namespaces (Rust, Gleam).
    pub types_apart: bool,
    // Functions may share a name, told apart by their parameters (C++);
    // never a type's.
    pub overloads: bool,
    // A function's identifier with the model name `name`, given its type's
    // identifier, if it has a type. None where the target does not name it
    // from the model's name (C++ names a getter by its property): renaming
    // it would not change it.
    pub func_name: fn(Option<&str>, &str, &Func) -> Option<String>,
    // A type's functions are top-level identifiers (C's `Shape_area`,
    // Gleam's `shape_area`), else each type is a scope of its own.
    pub funcs_flat: bool,
    // What a type's own scope holds before its functions (Zig's `h`).
    pub members: fn(&Class) -> Vec<String>,
}

// Renames the model so that `naming` makes each identifier once.
pub fn settle(m: &mut Model, naming: &Naming) {
    let reserved = (naming.reserved)();
    let reserved = || reserved.iter().map(String::as_str);
    let mut top = Tops { types: Scope::with(reserved()), values: Scope::with(reserved()), apart: naming.types_apart, overloads: naming.overloads };
    let mut type_renames: HashMap<String, String> = HashMap::new();
    // (class, or none for a top-level function; index): its new model name.
    let mut func_renames: Vec<(Option<usize>, usize, String)> = Vec::new();
    {
        let m: &Model = m;
        let all: Vec<(&String, Kind)> = m
            .classes
            .iter()
            .map(|c| (&c.name, Kind::Class))
            .chain(m.callbacks.iter().map(|c| (&c.name, Kind::Callback)))
            .chain(m.unions.iter().map(|u| (&u.name, Kind::Union)))
            .chain(m.enums.iter().map(|e| (&e.0, Kind::Enum)))
            .chain(m.num_enums.iter().map(|e| (&e.0, Kind::NumEnum)))
            .chain(m.opaque.iter().map(|o| (o, Kind::Opaque)))
            .collect();
        let claim_types = |synthetic: bool, top: &mut Tops, renames: &mut HashMap<String, String>| {
            for (name, kind) in all.iter().filter(|(n, _)| m.synthetic.contains(*n) == synthetic) {
                let chosen = claim(name, &mut |candidate| {
                    let ids = (naming.type_names)(candidate, name, *kind, m);
                    let values = ids.first().map(|own| (naming.values_of)(own, name, *kind, m)).unwrap_or_default();
                    top.take_type(ids, values)
                });
                if chosen != **name {
                    renames.insert((*name).clone(), chosen);
                }
            }
        };
        let claim_funcs = |of_synthetic: bool, top: &mut Tops, renames: &HashMap<String, String>, out: &mut Vec<(Option<usize>, usize, String)>| {
            for (ci, c) in m.classes.iter().enumerate().filter(|(_, c)| m.synthetic.contains(&c.name) == of_synthetic) {
                let owner = renames.get(&c.name).unwrap_or(&c.name);
                let owner = (naming.type_names)(owner, &c.name, Kind::Class, m).into_iter().next().unwrap_or_default();
                let mut own = Scope::with(reserved());
                own.take((naming.members)(c));
                for (fi, f) in c.funcs.iter().enumerate() {
                    if (naming.func_name)(Some(&owner), &f.name, f).is_none() {
                        continue;
                    }
                    let chosen = claim(&f.name, &mut |candidate| {
                        let id = (naming.func_name)(Some(&owner), candidate, f).unwrap_or_default();
                        if naming.funcs_flat {
                            top.take_value(vec![id])
                        } else if naming.overloads {
                            own.free(&[id])
                        } else {
                            take(&mut own, vec![id])
                        }
                    });
                    if chosen != f.name {
                        out.push((Some(ci), fi, chosen));
                    }
                }
            }
        };
        // What the library declares, then functions, then what lowering made up.
        claim_types(false, &mut top, &mut type_renames);
        claim_funcs(false, &mut top, &type_renames, &mut func_renames);
        for (fi, f) in m.funcs.iter().enumerate().filter(|(_, f)| (naming.func_name)(None, &f.name, f).is_some()) {
            let chosen = claim(&f.name, &mut |candidate| top.take_value(vec![(naming.func_name)(None, candidate, f).unwrap_or_default()]));
            if chosen != f.name {
                func_renames.push((None, fi, chosen));
            }
        }
        claim_types(true, &mut top, &mut type_renames);
        claim_funcs(true, &mut top, &type_renames, &mut func_renames);
    }
    for (class, fi, name) in func_renames {
        match class {
            Some(ci) => m.classes[ci].funcs[fi].name = name,
            None => m.funcs[fi].name = name,
        }
    }
    rename_types(m, &type_renames);
}

// The top-level identifiers, of types and of functions: in one namespace,
// or two apart; functions may share theirs where they overload.
struct Tops {
    types: Scope,
    values: Scope,
    apart: bool,
    overloads: bool,
}

impl Tops {
    fn take_type(&mut self, ids: Vec<String>, values: Vec<String>) -> bool {
        let free = self.types.free(&ids) && (self.apart || self.values.free(&ids)) && self.values.free(&values) && (self.apart || self.types.free(&values));
        if free {
            self.types.take(ids);
            self.values.take(values);
        }
        free
    }

    fn take_value(&mut self, ids: Vec<String>) -> bool {
        let free = (self.overloads || self.values.free(&ids)) && (self.apart || self.types.free(&ids));
        if free {
            self.values.take(ids);
        }
        free
    }
}

fn take(scope: &mut Scope, ids: Vec<String>) -> bool {
    let free = scope.free(&ids);
    if free {
        scope.take(ids);
    }
    free
}

// `name`, or `name_2`, `name_3`, ...: the first `try_take` takes. A naming
// whose identifier does not follow the name would never find one.
fn claim(name: &str, try_take: &mut dyn FnMut(&str) -> bool) -> String {
    let mut candidate = name.to_string();
    let mut n = 2;
    while !try_take(&candidate) {
        assert!(n < 10_000, "names: no identifier for `{name}`: does the naming follow the name?");
        candidate = format!("{name}_{n}");
        n += 1;
    }
    candidate
}

// The identifiers source declares or uses, outside its comments: what a
// runtime written by a generator has, for it to reserve.
pub fn identifiers(src: &str) -> Vec<String> {
    let mut out = Vec::new();
    for line in src.lines() {
        let code = line.split("//").next().unwrap_or_default();
        let mut word = String::new();
        for c in code.chars().chain([' ']) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
            } else if !word.is_empty() {
                if !word.as_bytes()[0].is_ascii_digit() {
                    out.push(std::mem::take(&mut word));
                }
                word.clear();
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

// Renames types everywhere the model names them.
pub fn rename_types(m: &mut Model, renames: &HashMap<String, String>) {
    if renames.is_empty() {
        return;
    }
    let to = |n: &mut String| {
        if let Some(new) = renames.get(n) {
            *n = new.clone();
        }
    };
    fn ty(t: &mut Ty, to: &dyn Fn(&mut String)) {
        match t {
            Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => to(n),
            Ty::Generic(n, args) => {
                to(n);
                args.iter_mut().for_each(|a| ty(a, to));
            }
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => ty(inner, to),
            Ty::Union(parts) | Ty::Tuple(parts) => parts.iter_mut().for_each(|p| ty(p, to)),
            Ty::Void | Ty::Num | Ty::Bool | Ty::Str | Ty::Any | Ty::Array | Ty::Param(_) => {}
        }
    }
    fn tparams(ps: &mut [TParam], to: &dyn Fn(&mut String)) {
        for p in ps {
            p.default.iter_mut().chain(p.constraint.iter_mut()).for_each(|t| ty(t, to));
        }
    }
    fn func(f: &mut Func, to: &dyn Fn(&mut String)) {
        tparams(&mut f.type_params, to);
        f.params.iter_mut().for_each(|p| ty(&mut p.1, to));
        ty(&mut f.ret, to);
    }
    for c in &mut m.classes {
        to(&mut c.name);
        tparams(&mut c.type_params, &to);
        c.bases.iter_mut().for_each(|b| ty(b, &to));
        c.ancestors.iter_mut().for_each(to);
        c.funcs.iter_mut().for_each(|f| func(f, &to));
        c.iterates.iter_mut().chain(c.iterates_async.iter_mut()).chain(c.elements.iter_mut()).for_each(|t| ty(t, &to));
    }
    for c in &mut m.callbacks {
        to(&mut c.name);
        tparams(&mut c.type_params, &to);
        c.params.iter_mut().for_each(|p| ty(&mut p.1, &to));
        ty(&mut c.ret, &to);
    }
    for u in &mut m.unions {
        to(&mut u.name);
        tparams(&mut u.type_params, &to);
        u.parts.iter_mut().for_each(|p| ty(p, &to));
    }
    for k in &mut m.maps {
        to(&mut k.name);
        k.values.iter_mut().for_each(|v| ty(v, &to));
    }
    m.enums.iter_mut().for_each(|e| to(&mut e.0));
    m.num_enums.iter_mut().for_each(|e| to(&mut e.0));
    m.opaque.iter_mut().for_each(to);
    m.opaque_arity = std::mem::take(&mut m.opaque_arity).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.synthetic = std::mem::take(&mut m.synthetic).into_iter().map(|mut k| (to(&mut k), k).1).collect();
    m.funcs.iter_mut().for_each(|f| func(f, &to));
}
