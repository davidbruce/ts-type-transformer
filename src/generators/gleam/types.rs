use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::bounds::generic_args;
use super::{codec_var, lower, relative, upper, JS_RESERVED, KEYWORDS, OPAQUE_BUILTINS, OWN};
use super::super::js::{call_expr, js_test, trim_trailing_undef};
use super::super::model::{mentions, subst, Class, Func, Model, NamedUnion, TParam, Ty};

pub(super) struct Gen<'m> {
    pub m: &'m Model,
    pub pkg: String,
    // Gleam's name of each type the model names, and of each union's or enum's variants.
    pub types: HashMap<String, String>,
    pub variants: HashMap<(String, usize), String>,
    // Each callback, by the places of the type parameters it uses in Gleam, and those needing JS codecs.
    pub callbacks: HashMap<String, Vec<usize>>,
    pub callback_codecs: HashMap<String, Vec<usize>>,
    pub union_codecs: HashMap<String, Vec<usize>>,
    // Values given out, by module; and names no module's value may have.
    pub values: HashSet<(String, String)>,
    pub reserved_values: HashSet<String>,
    // The module being written: its Gleam, its JavaScript, and the Gleam
    // types its Gleam names (for its imports). Those written, by path.
    pub unit: String,
    pub out: String,
    pub ffi: String,
    pub used: RefCell<BTreeSet<String>>,
    pub used_key_ctor: bool,
    pub units: BTreeMap<String, Unit>,
    // The module each Gleam type is declared in.
    pub home: HashMap<String, String>,
    // Each class's type parameter count; and the class its chain continues
    // from, with the arguments it gives it.
    pub classes: HashMap<String, usize>,
    pub parents: HashMap<String, (String, Vec<Ty>)>,
    // Class type parameters proven strictly class-bound across the entire model.
    pub class_bound: HashMap<(String, usize), (String, Vec<Ty>)>,
    // Where a class's type is open, `Element(a)`, any subtype (a function's
    // parameters): the variables given out. None where it is closed,
    // `Element(Nil)`, that type and no more (what comes back, what a callback
    // is given, what a list or variant holds).
    pub open: RefCell<Option<usize>>,
    // A function's own or class-bound type parameters bound to a class (`T extends Node`):
    // that class, open in the parameter's variable.
    pub bound: RefCell<HashMap<String, (String, Vec<Ty>)>>,
    // Function type parameters constrained to concrete `$same` types (primitives or non-inherited opaque types).
    pub fixed_params: RefCell<HashMap<String, Ty>>,
    // Function type parameters constrained to `ArrayOf(Class)` (`U extends ContainerChild[]`).
    pub bound_lists: RefCell<HashMap<String, (String, Vec<Ty>)>>,
    // Type variables allocated for `Ty::Key(map)` / `Ty::Mapped(map)` within a single signature.
    pub key_vars: RefCell<HashMap<String, String>>,
    // Recursion guard while expanding default type arguments in `class_type`.
    pub expanding_defaults: RefCell<HashSet<String>>,
    pub skipped: BTreeMap<String, usize>,
}

#[derive(Default)]
pub(super) struct Unit {
    pub out: String,
    pub ffi: String,
    pub used: BTreeSet<String>,
    pub used_key_ctor: bool,
}

impl<'m> Gen<'m> {
    pub(super) fn skip(&mut self, why: &str) {
        *self.skipped.entry(why.to_string()).or_default() += 1;
    }

    // A module-level value's name: `get_size` in a class's module.
    // `names::settle` made them apart; one given out twice in a module is a
    // mistake in its naming.
    pub(super) fn value_name(&mut self, base: &str) -> String {
        let name = lower(base);
        assert!(!self.reserved_values.contains(&name) && self.values.insert((self.unit.clone(), name.clone())), "Gleam: the value `{name}` is named twice in {}", self.unit);
        name
    }

    // A value's name in the module being written, the first of `base`,
    // `base_2`, ... it has not given out, and no keyword.
    pub(super) fn fresh_value(&mut self, base: &str) -> String {
        let mut base = lower(base);
        if KEYWORDS.contains(&base.as_str()) {
            base.push('_');
        }
        let mut name = base.clone();
        let mut i = 2;
        while self.reserved_values.contains(&name) || self.values.contains(&(self.unit.clone(), name.clone())) {
            name = format!("{base}_{i}");
            i += 1;
        }
        self.values.insert((self.unit.clone(), name.clone()));
        name
    }

    pub(super) fn tname(&self, n: &str) -> String {
        self.types.get(n).cloned().unwrap_or_else(|| upper(n))
    }

    // A type's name, written in the module's Gleam: noted, for its import.
    pub(super) fn gt(&self, n: &str) -> String {
        let t = self.tname(n);
        self.note(&t);
        t
    }

    pub(super) fn note(&self, gleam_type: &str) {
        self.used.borrow_mut().insert(gleam_type.to_string());
    }

    // Writes to another module from here on.
    pub(super) fn switch(&mut self, unit: &str) {
        let done = Unit {
            out: std::mem::take(&mut self.out),
            ffi: std::mem::take(&mut self.ffi),
            used: self.used.take(),
            used_key_ctor: std::mem::take(&mut self.used_key_ctor),
        };
        let was = self.units.entry(std::mem::replace(&mut self.unit, unit.to_string())).or_default();
        was.out += &done.out;
        was.ffi += &done.ffi;
        was.used.extend(done.used);
        was.used_key_ctor |= done.used_key_ctor;
        let next = self.units.remove(unit).unwrap_or_default();
        self.out = next.out;
        self.ffi = next.ffi;
        *self.used.borrow_mut() = next.used;
        self.used_key_ctor = next.used_key_ctor;
    }

    // The module's own JavaScript, as an external names it.
    pub(super) fn own_ffi(&self) -> String {
        format!("./{}_ffi.mjs", self.unit.rsplit('/').next().unwrap_or_default())
    }

    // The runtime's JavaScript, from the module being written.
    pub(super) fn runtime_ffi(&self) -> String {
        relative(&self.unit, &format!("{}/runtime_ffi.mjs", self.pkg))
    }

    // A generic type named without its arguments: `Polygon(Dynamic)`.
    pub(super) fn bare(&self, n: &str) -> String {
        if self.classes.contains_key(n) {
            return self.class_type(n, &[]);
        }
        let arity = self.m.classes.iter().find(|c| c.name == n).map(|c| c.type_params.len())
            .or_else(|| self.m.opaque_arity.get(n).copied())
            .or_else(|| self.m.unions.iter().find(|u| u.name == n).map(|u| u.type_params.len()))
            .or_else(|| self.callbacks.get(n).map(Vec::len))
            .unwrap_or(0);
        let name = self.gt(n);
        if arity == 0 {
            name
        } else {
            self.note("Dynamic");
            format!("{name}({})", vec!["Dynamic"; arity].join(", "))
        }
    }

    pub(super) fn tvar(n: &str) -> String {
        lower(n)
    }

    // A class type's last argument: a fresh variable where open, else `Nil`.
    fn phantom(&self) -> String {
        let mut open = self.open.borrow_mut();
        match open.as_mut() {
            Some(k) => {
                *k += 1;
                format!("sub_{k}")
            }
            None => "Nil".into(),
        }
    }

    // `f`, with class types closed.
    pub(super) fn closed<T>(&self, f: impl FnOnce() -> T) -> T {
        let was = self.open.replace(None);
        let out = f();
        *self.open.borrow_mut() = was;
        out
    }

    // A class's type with its arguments (using parameter defaults, or `Dynamic`
    // when unavailable/cyclic) and its last, phantom one.
    pub(super) fn class_type(&self, n: &str, args: &[Ty]) -> String {
        let count = self.classes[n];
        let cls = self.m.classes.iter().find(|c| c.name == n);
        let mut parts: Vec<String> = Vec::with_capacity(count + 1);
        let mut resolved: Vec<Ty> = Vec::with_capacity(count);
        for i in 0..count {
            if let Some(a) = args.get(i) {
                resolved.push(a.clone());
                parts.push(self.closed(|| self.ty(a)));
                continue;
            }
            if let Some(c) = cls
                && let Some(tp) = c.type_params.get(i)
                && let Some(def) = &tp.default
                && !self.expanding_defaults.borrow().contains(n)
            {
                let def_sub = subst(def, &c.type_params[..i], &resolved);
                let unresolved = c.type_params[i..].iter().any(|rem| mentions(&def_sub, &rem.name));
                if !unresolved {
                    self.expanding_defaults.borrow_mut().insert(n.to_string());
                    let s = self.closed(|| self.ty(&def_sub));
                    self.expanding_defaults.borrow_mut().remove(n);
                    resolved.push(def_sub);
                    parts.push(s);
                    continue;
                }
            }
            self.note("Dynamic");
            resolved.push(Ty::Any);
            parts.push("Dynamic".to_string());
        }
        parts.push(self.phantom());
        format!("{}({})", self.gt(n), parts.join(", "))
    }

    fn decodable(&self, n: &str) -> bool {
        self.m.unions.iter().any(|u| u.name == n && u.decodable())
    }

    fn is_callback(&self, n: &str) -> bool {
        self.callbacks.contains_key(n)
    }

    // Whether Gleam's type of a value names a type parameter.
    pub(super) fn shows(&self, t: &Ty, param: &str) -> bool {
        match t {
            Ty::Param(n) => n == param,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => self.shows(inner, param),
            Ty::Tuple(es) => es.iter().any(|e| self.shows(e, param)),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => false,
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => self.callbacks[n].iter().filter_map(|&i| args.get(i)).any(|a| self.shows(a, param)),
            Ty::Generic(_, args) => args.iter().any(|a| self.shows(a, param)),
            _ => false,
        }
    }

    fn key_var(&self, map: &str) -> String {
        let mut vars = self.key_vars.borrow_mut();
        if let Some(v) = vars.get(map) {
            return v.clone();
        }
        let v = format!("kv_{}", vars.len());
        vars.insert(map.to_string(), v.clone());
        v
    }

    fn bound_class_ty(&self, class: &str, args: &[Ty], param: &str) -> String {
        let tvar = Self::tvar(param);
        let count = self.classes[class];
        let mut parts: Vec<String> = (0..count)
            .map(|i| match args.get(i) {
                Some(a) if !matches!(a, Ty::Any) => self.closed(|| self.ty(a)),
                _ => format!("{tvar}_a{i}"),
            })
            .collect();
        parts.push(tvar);
        format!("{}({})", self.gt(class), parts.join(", "))
    }

    // Gleam's type of a value.
    pub(super) fn ty(&self, t: &Ty) -> String {
        let dynamic = || {
            self.note("Dynamic");
            "Dynamic".to_string()
        };
        match t {
            Ty::Void => "Nil".into(),
            Ty::Num => "Float".into(),
            Ty::Bool => "Bool".into(),
            Ty::Str => "String".into(),
            Ty::Any | Ty::Union(_) => dynamic(),
            Ty::Key(map) => {
                self.note("Key");
                let map_ty = self.closed(|| self.bare(map));
                let v = self.key_var(map);
                format!("Key({map_ty}, {v})")
            }
            Ty::Mapped(map) => self.key_var(map),
            Ty::Listener(inner) => {
                let ev = self.closed(|| self.ty(inner));
                format!("fn({ev}) -> Nil")
            }
            Ty::Array => format!("List({})", dynamic()),
            Ty::ArrayOf(e) => format!("List({})", self.ty(e)),
            Ty::Tuple(es) => format!("#({})", es.iter().map(|e| self.ty(e)).collect::<Vec<_>>().join(", ")),
            // A promise of what the model does not say.
            Ty::Obj(n) if n == "Promise" || n == "PromiseLike" => {
                self.note("Promise");
                format!("Promise({})", dynamic())
            }
            Ty::Obj(n) => self.bare(n),
            Ty::Enum(n) | Ty::NumEnum(n) => self.gt(n),
            Ty::Nullable(inner, _) => {
                self.note("Option");
                format!("Option({})", self.ty(inner))
            }
            Ty::NamedUnion(n) if self.decodable(n) => self.bare(n),
            Ty::NamedUnion(_) => dynamic(),
            Ty::Param(n) => {
                if let Some((class, args)) = self.bound.borrow().get(n).cloned() {
                    return self.bound_class_ty(&class, &args, n);
                }
                if let Some(fixed_ty) = self.fixed_params.borrow().get(n).cloned() {
                    return self.ty(&fixed_ty);
                }
                if let Some((class, args)) = self.bound_lists.borrow().get(n).cloned() {
                    return format!("List({})", self.bound_class_ty(&class, &args, n));
                }
                Self::tvar(n)
            }
            Ty::Generic(n, args) if self.classes.contains_key(n) => self.class_type(n, args),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => dynamic(),
            // A callback takes the type arguments it uses.
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => {
                let args: Vec<String> = self.closed(|| self.callbacks[n].iter().filter_map(|&i| args.get(i)).map(|a| self.ty(a)).collect());
                if args.is_empty() { self.gt(n) } else { format!("{}({})", self.gt(n), args.join(", ")) }
            }
            Ty::Generic(n, args) if args.is_empty() => self.bare(n),
            Ty::Generic(n, args) => {
                let args = self.closed(|| args.iter().map(|a| self.ty(a)).collect::<Vec<_>>());
                format!("{}({})", self.gt(n), args.join(", "))
            }
        }
    }

    // A JavaScript expression that is `x`, JavaScript's value, as Gleam has it.
    pub(super) fn to_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            // A type parameter's values cross by the codec the function was given,
            // except when bound to a class, concrete `$same` type, or class list.
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => format!("toList(Array.from({x} ?? []))"),
            Ty::Param(n) => format!("{}.to({x})", codec_var(n)),
            Ty::Void => format!("({x}, undefined)"),
            Ty::Key(_) => format!("$key({x})"),
            Ty::Mapped(_) => x.to_string(),
            Ty::Listener(inner) => {
                let conv = self.from_gleam(inner, "a0");
                if conv == "a0" {
                    x.to_string()
                } else {
                    format!("(typeof {x} === \"function\" ? (a0) => {x}({conv}) : {x})")
                }
            }
            Ty::Nullable(inner, _) => format!("((v) => v == null ? new None() : new Some({}))({x})", self.to_gleam(inner, "v")),
            Ty::Array => format!("toList(Array.from({x} ?? []))"),
            Ty::ArrayOf(e) => format!("toList(Array.from({x} ?? [], (v) => {}))", self.to_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.to_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("$T.{}_to({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("$T.{}_to({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("$T.{}_to({x}{})", self.tname(n), self.codec_args(n, args)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("$T.{}_to({x}{})", self.tname(n), self.codec_args(n, generic_args(t))),
            Ty::Generic(n, args) if (n == "Promise" || n == "PromiseLike") && args.len() == 1 => {
                format!("Promise.resolve({x}).then((v) => {})", self.to_gleam(&args[0], "v"))
            }
            _ => x.to_string(),
        }
    }

    // The codecs a generic union's or callback's conversions take, for the
    // type parameters they use: `, $same, $c_T`.
    fn codec_args(&self, n: &str, args: &[Ty]) -> String {
        let used: &[usize] = self
            .callback_codecs
            .get(n)
            .or_else(|| self.union_codecs.get(n))
            .map(Vec::as_slice)
            .unwrap_or(&[]);
        // Named without its arguments, it is of `Dynamic`: values as they are.
        used.iter().map(|&i| format!(", {}", args.get(i).map_or("$same".to_string(), |a| self.codec_expr(a)))).collect()
    }

    // A JavaScript codec for values of `t`: how they cross each way.
    fn codec_expr(&self, t: &Ty) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => "$same".into(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => "{ to: (v) => toList(Array.from(v ?? [])), from: (v) => v.toArray() }".into(),
            Ty::Param(n) => codec_var(n),
            t if self.to_gleam(t, "v") == "v" && self.from_gleam(t, "v") == "v" => "$same".into(),
            t => format!("{{ to: (v) => {}, from: (v) => {} }}", self.to_gleam(t, "v"), self.from_gleam(t, "v")),
        }
    }

    // Whether a type parameter's values cross where `t` is: bare, in an
    // option, list, tuple or promise, through a callback or a union. Not
    // inside a handle (`Box(t)`), which crosses as it is.
    pub(super) fn crosses(&self, t: &Ty, p: &str) -> bool {
        if self.bound.borrow().contains_key(p) || self.fixed_params.borrow().contains_key(p) || self.bound_lists.borrow().contains_key(p) {
            return false;
        }
        match t {
            Ty::Param(n) => n == p,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => self.crosses(inner, p),
            Ty::Tuple(es) => es.iter().any(|e| self.crosses(e, p)),
            Ty::Generic(n, args) if n == "Promise" || n == "PromiseLike" => args.first().is_some_and(|a| self.crosses(a, p)),
            Ty::Generic(n, args) if self.is_callback(n) => self.callback_codecs.get(n).is_some_and(|used| used.iter().filter_map(|&i| args.get(i)).any(|a| self.crosses(a, p))),
            Ty::Generic(n, args) if self.decodable(n) => self.union_codecs.get(n).is_some_and(|used| used.iter().filter_map(|&i| args.get(i)).any(|a| self.crosses(a, p))),
            _ => false,
        }
    }

    // A JavaScript expression that is `x`, Gleam's value, as JavaScript has it.
    #[allow(clippy::wrong_self_convention)]
    pub(super) fn from_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) || self.fixed_params.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) if self.bound_lists.borrow().contains_key(n) => format!("{x}.toArray()"),
            Ty::Param(n) => format!("{}.from({x})", codec_var(n)),
            Ty::Key(_) => format!("{x}.raw"),
            Ty::Mapped(_) => x.to_string(),
            Ty::Listener(inner) => {
                let conv = self.to_gleam(inner, "e");
                if conv == "e" {
                    x.to_string()
                } else {
                    format!("$wrap({x}, (f) => (e) => f({conv}))")
                }
            }
            Ty::Nullable(inner, undefined) => format!("((v) => v instanceof Some ? {} : {})({x})", self.from_gleam(inner, "v[0]"), if *undefined { "undefined" } else { "null" }),
            Ty::Array => format!("{x}.toArray()"),
            Ty::ArrayOf(e) => format!("{x}.toArray().map((v) => {})", self.from_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.from_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("$T.{}_from({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("$T.{}_from({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("$T.{}_from({x}{})", self.tname(n), self.codec_args(n, args)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("$T.{}_from({x}{})", self.tname(n), self.codec_args(n, generic_args(t))),
            Ty::Generic(n, args) if (n == "Promise" || n == "PromiseLike") && args.len() == 1 => {
                format!("Promise.resolve({x}).then((v) => {})", self.from_gleam(&args[0], "v"))
            }
            _ => x.to_string(),
        }
    }

    pub(super) fn type_params(params: &[TParam]) -> String {
        if params.is_empty() { String::new() } else { format!("({})", params.iter().map(|p| Self::tvar(&p.name)).collect::<Vec<_>>().join(", ")) }
    }

    fn is_fixed_opaque(&self, o: &str) -> bool {
        !self.classes.contains_key(o)
            && self.m.opaque.iter().any(|x| x == o)
            && !OPAQUE_BUILTINS.contains(&o)
            && self.m.opaque_arity.get(o).copied().unwrap_or(0) == 0
    }

    pub(super) fn clear_fn_bounds(&self) {
        self.bound.borrow_mut().clear();
        self.fixed_params.borrow_mut().clear();
        self.bound_lists.borrow_mut().clear();
        self.key_vars.borrow_mut().clear();
    }

    pub(super) fn setup_fn_bounds(&self, f: &Func, owner: Option<(&str, &[TParam])>) {
        self.clear_fn_bounds();
        let has_self = Class::has_self(f);
        let class_param_names: HashSet<&str> = owner.map_or_else(HashSet::new, |(_, tps)| tps.iter().map(|p| p.name.as_str()).collect());
        if let Some((owner_name, class_params)) = owner {
            for (i, cp) in class_params.iter().enumerate() {
                if !f.type_params.iter().any(|fp| fp.name == cp.name)
                    && let Some(binding) = self.class_bound.get(&(owner_name.to_string(), i))
                {
                    self.bound.borrow_mut().insert(cp.name.clone(), binding.clone());
                }
            }
        }
        for p in &f.type_params {
            if has_self && class_param_names.contains(p.name.as_str()) {
                continue;
            }
            match &p.constraint {
                Some(Ty::Obj(c)) if self.classes.contains_key(c) => {
                    self.bound.borrow_mut().insert(p.name.clone(), (c.clone(), Vec::new()));
                }
                Some(Ty::Generic(c, args)) if self.classes.contains_key(c) => {
                    self.bound.borrow_mut().insert(p.name.clone(), (c.clone(), args.clone()));
                }
                Some(Ty::Str) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Str);
                }
                Some(Ty::Num) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Num);
                }
                Some(Ty::Bool) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Bool);
                }
                Some(Ty::Obj(o)) if self.is_fixed_opaque(o) => {
                    self.fixed_params.borrow_mut().insert(p.name.clone(), Ty::Obj(o.clone()));
                }
                Some(Ty::ArrayOf(inner)) => {
                    let elem = match &**inner {
                        Ty::Obj(c) if self.classes.contains_key(c) => Some((c.clone(), Vec::new())),
                        Ty::Generic(c, args) if self.classes.contains_key(c) => Some((c.clone(), args.clone())),
                        Ty::Param(cp_name) => owner.and_then(|(_, tps)| {
                            tps.iter().find(|tp| tp.name == *cp_name).and_then(|tp| match &tp.constraint {
                                Some(Ty::Obj(c)) if self.classes.contains_key(c) => Some((c.clone(), Vec::new())),
                                Some(Ty::Generic(c, args)) if self.classes.contains_key(c) => Some((c.clone(), args.clone())),
                                _ => None,
                            })
                        }),
                        _ => None,
                    };
                    if let Some(binding) = elem {
                        self.bound_lists.borrow_mut().insert(p.name.clone(), binding);
                    }
                }
                _ => {}
            }
        }
    }

    pub(super) fn crossing_params(&self, f: &Func, owner: Option<(&str, &[TParam])>) -> Vec<TParam> {
        let class_params: &[TParam] = owner.map_or(&[], |o| o.1);
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        for p in class_params.iter().chain(&f.type_params).filter(|p| seen.insert(p.name.clone())) {
            if f.params.iter().map(|x| &x.1).chain([&f.ret]).any(|t| self.crosses(t, &p.name)) {
                out.push(p.clone());
            }
        }
        out
    }

    // One external function, `name`, and its JavaScript; returns its JS export name.
    pub(super) fn function(&mut self, name: &str, f: &Func, owner: Option<(&str, &[TParam])>) -> String {
        self.setup_fn_bounds(f, owner);
        let mut params = Vec::new();
        let mut js_params = Vec::new();
        // A class's type is open in the parameters: any subtype.
        *self.open.borrow_mut() = Some(0);
        let mut self_ty = None;
        if let Some((class, tps)) = owner.filter(|_| Class::has_self(f)) {
            let own: Vec<Ty> = tps.iter().map(|p| Ty::Param(p.name.clone())).collect();
            let st = self.class_type(class, &own);
            params.push(format!("self: {st}"));
            self_ty = Some(st);
            js_params.push("self".to_string());
        }
        let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
        let mut args = Vec::new();
        let mut may_undef = Vec::new();
        for (i, (p, t)) in f.params.iter().enumerate() {
            // A parameter of the FFI's JavaScript too: `default_` there.
            let mut id = lower(p);
            while JS_RESERVED.contains(&id.as_str()) || !taken.insert(id.clone()) {
                id.push('_');
            }
            let last_rest = f.rest && i == f.params.len() - 1;
            let param_ty = self.ty(t);
            params.push(format!("{id}: {param_ty}"));
            js_params.push(id.clone());
            let value = self.from_gleam(t, &id);
            args.push(if last_rest { format!("...({value} ?? [])") } else { value });
            may_undef.push(!last_rest && matches!(t, Ty::Nullable(_, true)));
        }
        // A codec for each type parameter whose values cross: the class's, then the function's own.
        *self.open.borrow_mut() = None;
        for p in self.crossing_params(f, owner) {
            self.note("Codec");
            params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
            js_params.push(codec_var(&p.name));
        }
        for (position, literal) in &f.fixed {
            let pos = (*position).min(args.len());
            args.insert(pos, literal.clone());
            may_undef.insert(pos, false);
        }
        trim_trailing_undef(&f.call, f.rest, &mut args, &may_undef, "$trim");
        let call = call_expr(&f.call, "self", &args);
        let body = match &f.ret {
            Ty::Void => format!("{call};\n  return undefined;"),
            t => format!("return {};", self.to_gleam(t, &call)),
        };
        let ret = if f.returns_this { self_ty.unwrap_or_else(|| self.ty(&f.ret)) } else { self.ty(&f.ret) };
        // A name JavaScript reserves (`new`, `delete`) is `new_` there.
        let js = if JS_RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() };
        self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        self.ffi += &format!("export function {js}({}) {{\n  {body}\n}}\n", js_params.join(", "));
        self.clear_fn_bounds();
        js
    }

    // A union as a custom type, and its conversions: told apart by its tests.
    pub(super) fn union(&mut self, u: &NamedUnion) {
        let name = self.tname(&u.name);
        let params = Self::type_params(&u.type_params);
        let variants: Vec<String> = (0..u.parts.len()).map(|i| self.variants[&(u.name.clone(), i)].clone()).collect();
        let cases: Vec<String> = u.parts.iter().zip(&variants).map(|(p, v)| format!("  {v}({})", self.ty(p))).collect();
        self.out += &format!("pub type {name}{params} {{\n{}\n}}\n\n", cases.join("\n"));
        // A library's name for it, where that is free.
        if u.type_params.is_empty() {
            let mut seen = HashSet::new();
            for (alias, _) in self.m.type_aliases.iter().filter(|(_, to)| *to == u.name) {
                let a = upper(alias);
                if !OWN.contains(&a.as_str()) && !self.types.values().any(|t| *t == a) && seen.insert(a.clone()) {
                    self.out += &format!("pub type {a} =\n  {name}\n\n");
                }
            }
        }
        let mut to = String::new();
        let mut fallback = None;
        for ((p, test), v) in u.parts.iter().zip(&u.tests).zip(&variants) {
            match test {
                Some(t) => to += &format!("  if ({}) return new B.{v}({});\n", js_test(t, u.discriminant.as_deref(), &self.m.js_refs), self.to_gleam(p, "v")),
                None => fallback = Some(format!("  return new B.{v}({});\n", self.to_gleam(p, "v"))),
            }
        }
        to += &fallback.unwrap_or_else(|| format!("  throw new TypeError(\"not a {name}: \" + v);\n"));
        let from: String = u.parts.iter().zip(&variants).map(|(p, v)| format!("  if (u instanceof B.{v}) return {};\n", self.from_gleam(p, "u[0]"))).collect();
        let codecs: String = self.union_codecs[&u.name].iter().map(|&i| format!(", {}", codec_var(&u.type_params[i].name))).collect();
        self.ffi += &format!("export function {name}_to(v{codecs}) {{\n{to}}}\nexport function {name}_from(u{codecs}) {{\n{from}}}\n");
    }
}
