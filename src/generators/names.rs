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

use super::model::{Call, Class, Func, Model, TParam, Ty};

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
    pub members: fn(&Class, &Model) -> Vec<String>,
    // A namespace's types are in a scope of their own (Rust's
    // `stream::PipeOptions`, Zig's `stream.PipeOptions`), not the top level's.
    pub nests: bool,
}

// Renames the model so that `naming` makes each identifier once.
pub fn settle(m: &mut Model, naming: &Naming) {
    settle_with(m, naming, &[]);
}

pub fn settle_with(m: &mut Model, naming: &Naming, extra_reserved: &[String]) {
    let mut reserved = (naming.reserved)();
    reserved.extend(extra_reserved.iter().cloned());
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
        // What the library declares at the top level, then in namespaces
        // (`Stream.PipeOptions` gives way to a top-level `StreamPipeOptions`);
        // what lowering made up, after.
        let ranked: Vec<&(&String, Kind)> = {
            let mut v: Vec<&(&String, Kind)> = all.iter().collect();
            v.sort_by_key(|(n, _)| m.namespaced.contains_key(*n));
            v
        };
        let claim_types = |synthetic: bool, top: &mut Tops, renames: &mut HashMap<String, String>| {
            for (name, kind) in ranked.iter().copied().filter(|(n, _)| m.synthetic.contains(*n) == synthetic && !(naming.nests && m.namespaced.contains_key(*n))) {
                // Where it must give way, it is named by where it comes from:
                // node's `readline` `Key` is `ReadlineKey`.
                let mut fallbacks: Vec<String> = m.origins.get(*name).and_then(|o| origin_name(o, name)).into_iter().collect();
                if *kind == Kind::Callback && !name.ends_with("_cb") {
                    fallbacks.push(format!("{name}_cb"));
                }
                let chosen = claim_as(name, &fallbacks, &mut |candidate| {
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
                own.take((naming.members)(c, m));
                // The library's methods first: a property's setter does not
                // take `setTranslation` from the method of that name.
                let mut order: Vec<(usize, &Func)> = c.funcs.iter().enumerate().collect();
                order.sort_by_key(|(_, f)| claim_rank(f));
                for (fi, f) in order {
                    if (naming.func_name)(Some(&owner), &f.name, f).is_none() {
                        continue;
                    }
                    // A static the instance's member of that name has taken is
                    // the class's (node's `EventEmitter.once(emitter, name)`: `static_once`).
                    let fallbacks: Vec<String> = if matches!(f.call, Call::Static(_)) { vec![format!("static_{}", f.name)] } else { Vec::new() };
                    let chosen = claim_as(&f.name, &fallbacks, &mut |candidate| {
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
    claim_as(name, &[], try_take)
}

// As `claim`, trying the `others` after `name`, before numbers.
fn claim_as(name: &str, others: &[String], try_take: &mut dyn FnMut(&str) -> bool) -> String {
    if let Some(o) = std::iter::once(name).chain(others.iter().map(String::as_str)).find(|c| try_take(c)) {
        return o.to_string();
    }
    let mut candidate = format!("{name}_2");
    let mut n = 3;
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
    for (a, u) in &mut m.type_aliases {
        to(a);
        to(u);
    }
    for k in &mut m.maps {
        to(&mut k.name);
        k.values.iter_mut().for_each(|v| ty(v, &to));
    }
    m.enums.iter_mut().for_each(|e| to(&mut e.0));
    m.num_enums.iter_mut().for_each(|e| to(&mut e.0));
    m.opaque.iter_mut().for_each(to);
    m.opaque_names = std::mem::take(&mut m.opaque_names).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.opaque_arity = std::mem::take(&mut m.opaque_arity).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.synthetic = std::mem::take(&mut m.synthetic).into_iter().map(|mut k| (to(&mut k), k).1).collect();
    m.namespaced = std::mem::take(&mut m.namespaced).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.namespace_modules = std::mem::take(&mut m.namespace_modules).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.type_modules = std::mem::take(&mut m.type_modules).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.origins = std::mem::take(&mut m.origins).into_iter().map(|(mut k, v)| (to(&mut k), (k, v)).1).collect();
    m.funcs.iter_mut().for_each(|f| func(f, &to));
}

pub(crate) fn stem(w: &str) -> String {
    let w = w.to_lowercase();
    if w.len() > 4 && w.ends_with("es") {
        w[..w.len() - 2].to_string()
    } else if w.len() > 3 && w.ends_with('s') {
        w[..w.len() - 1].to_string()
    } else {
        w
    }
}

/// Whether `name` already carries `prefix` at word boundaries (by word stems or a contiguous word slice).
pub(crate) fn has_prefix_word(prefix: &str, name: &str) -> bool {
    let pw = words(prefix);
    if pw.is_empty() {
        return true;
    }
    let nw = words(name);
    let name_stems: HashSet<String> = nw.iter().map(|w| stem(w)).collect();
    if pw.iter().all(|w| name_stems.contains(&stem(w))) {
        return true;
    }
    let p_joined: String = pw.iter().map(|w| w.to_lowercase()).collect();
    for i in 0..nw.len() {
        let mut acc = String::new();
        for w in &nw[i..] {
            acc.push_str(&w.to_lowercase());
            if acc == p_joined {
                return true;
            }
            if acc.len() >= p_joined.len() {
                break;
            }
        }
    }
    false
}

// A type named by its origin, less what its name says already:
// `readline` and `Key` are `ReadlineKey`, `stream/web` and
// `StreamPipeOptions` `WebStreamPipeOptions`, `GlContextSystem` and
// `ContextSystemOptions` `GlContextSystemOptions`. None where the name says
// all of it (`AddDaysOptions` of addDays.d.ts): the origin adds nothing.
pub(crate) fn origin_name(origin: &str, name: &str) -> Option<String> {
    let upper = |w: &str| {
        let mut cs = w.chars();
        cs.next().map(|c| c.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default()
    };
    let said: HashSet<String> = words(name).iter().map(|w| stem(w)).collect();
    let prefix: String = words(origin).iter().filter(|w| !said.contains(&stem(w))).map(|w| upper(w)).collect();
    (!prefix.is_empty()).then(|| format!("{prefix}{}", upper(name)))
}

// The words of a name: `fs/promises` is fs, promises; `HTMLElement` HTML,
// Element; `addISOWeekYears` add, ISO, Week, Years.
pub(crate) fn words(s: &str) -> Vec<String> {
    let mut out = Vec::new();
    for part in s.split(|c: char| !c.is_alphanumeric()).filter(|p| !p.is_empty()) {
        let cs: Vec<char> = part.chars().collect();
        let mut word = String::new();
        for (i, &c) in cs.iter().enumerate() {
            let prev = i.checked_sub(1).map(|j| cs[j]);
            let next = cs.get(i + 1);
            let starts = prev.is_some_and(|p| {
                (c.is_uppercase() && (p.is_lowercase() || p.is_ascii_digit()))
                    || (c.is_uppercase() && p.is_uppercase() && next.is_some_and(|n| n.is_lowercase()))
                    || (c.is_ascii_digit() != p.is_ascii_digit())
            });
            if starts && !word.is_empty() {
                out.push(std::mem::take(&mut word));
            }
            word.push(c);
        }
        if !word.is_empty() {
            out.push(word);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn origin_names_say_nothing_twice() {
        assert_eq!(origin_name("readline", "Key").as_deref(), Some("ReadlineKey"));
        assert_eq!(origin_name("stream/web", "StreamPipeOptions").as_deref(), Some("WebStreamPipeOptions"));
        assert_eq!(origin_name("GlContextSystem", "ContextSystemOptions").as_deref(), Some("GlContextSystemOptions"));
        assert_eq!(origin_name("fs/promises", "FileHandle").as_deref(), Some("FsPromisesFileHandle"));
        assert_eq!(origin_name("addDays", "AddDaysOptions"), None);
        assert_eq!(origin_name("PointerEvents", "KonvaPointerEvent"), None);
        assert_eq!(words("addISOWeekYears"), ["add", "ISO", "Week", "Years"]);
        assert_eq!(words("HTMLElement2D"), ["HTML", "Element", "2", "D"]);
        assert!(has_prefix_word("echarts", "ECharts"));
        assert!(has_prefix_word("echarts", "EChartsOption"));
        assert!(has_prefix_word("katex", "KatexOptions"));
        assert!(has_prefix_word("marked", "MarkedOptions"));
        assert!(has_prefix_word("konva", "KonvaEventObject"));
        assert!(has_prefix_word("table", "Table_Core"));
        assert!(has_prefix_word("pts", "PtsCanvas"));
        assert!(!has_prefix_word("pm", "StepMap"));
        assert!(!has_prefix_word("konva", "Node"));
        assert!(!has_prefix_word("babylon", "Vector3"));
        assert!(!has_prefix_word("pts", "Pt"));
    }
}

// Which of a type's members claims a name first: its methods, then its
// statics, then what else it is called by, then its properties' accessors.
fn claim_rank(f: &Func) -> u8 {
    match f.call {
        Call::Method(_) => 0,
        Call::Static(_) => 1,
        Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::StaticGet(_) | Call::StaticSet(_) => 3,
        _ => 2,
    }
}
