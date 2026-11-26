use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::bounds::generic_args;
use super::{codec_var, lower, relative, snake, upper, JS_RESERVED, KEYWORDS, OPAQUE_BUILTINS, OWN};
use super::super::comments::{Style, comment};
use super::super::js::{call_expr, js_test, trim_trailing_undef};
use super::super::model::{mentions, subst, Absence, Call, Class, Func, Model, NamedUnion, TParam, Ty, suffixed};

pub(super) struct Gen<'m> {
    pub m: &'m Model,
    // Unions whose values the user is given (a result, a callback's argument):
    // they go back in too, through `name_of`.
    pub given_back: HashSet<String>,
    // Opaque types no referenced bindings declare: nothing here makes one
    // (pts' `CanvasLineJoin`, without the DOM's bindings).
    pub unmakeable: HashSet<String>,
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
    // Where a value goes into JavaScript (a parameter, a setter's value): a
    // union no test tells apart is its own type there, `Dynamic` elsewhere.
    pub going_in: Cell<bool>,
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
    // `base`, else `other` where `base` is taken and it is not, else `base` numbered.
    pub(super) fn fresh_value_or(&mut self, base: &str, other: &str) -> String {
        let taken = |g: &Self, n: &str| {
            let mut n = lower(n);
            if KEYWORDS.contains(&n.as_str()) {
                n.push('_');
            }
            g.reserved_values.contains(&n) || g.values.contains(&(g.unit.clone(), n))
        };
        if taken(self, base) && !taken(self, other) { self.fresh_value(other) } else { self.fresh_value(base) }
    }

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
        let in_was = self.going_in.replace(false);
        let out = f();
        *self.open.borrow_mut() = was;
        self.going_in.set(in_was);
        out
    }

    // `f`, for a value going into JavaScript.
    pub(super) fn going_in<T>(&self, f: impl FnOnce() -> T) -> T {
        let was = self.going_in.replace(true);
        let out = f();
        self.going_in.set(was);
        out
    }

    fn made_in(&self, n: &str) -> bool {
        self.going_in.get() && self.m.unions.iter().any(|u| u.name == n && !u.decodable() && u.typable())
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
            Ty::NamedUnion(n) if self.decodable(n) || self.made_in(n) => self.bare(n),
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
            Ty::Nullable(inner, Absence::False) => format!("((v) => v == null || v === false ? new None() : new Some({}))({x})", self.to_gleam(inner, "v")),
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
            Ty::Nullable(inner, absent) => format!("((v) => v instanceof Some ? {} : {})({x})", self.from_gleam(inner, "v[0]"), absent.js()),
            Ty::Array => format!("{x}.toArray()"),
            Ty::ArrayOf(e) => format!("{x}.toArray().map((v) => {})", self.from_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.from_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("$T.{}_from({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) || self.made_in(n) => format!("$T.{}_from({x})", self.tname(n)),
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

    // `name` and its forms for union arguments (see `member_forms`): returns
    // the JS export of the first, and every name given out.
    // With `with_as`, only the form with its optional tail, by that name: an
    // overload another leads (see `Model::tail_overloads`).
    pub(super) fn function(&mut self, name: &str, f: &Func, owner: Option<(&str, &[TParam])>, with_as: Option<&str>) -> (String, Vec<String>) {
        let forms = self.member_forms(&f.params, f.rest, optional_tail(f));
        if forms.is_empty() {
            let (js, with) = self.function_form(name, f, owner, with_as.is_some(), &[], with_as);
            let plain = (with_as.is_none() || with.is_none()).then(|| name.to_string());
            return (js, plain.into_iter().chain(with).collect());
        }
        let mut first = None;
        let mut names = Vec::new();
        for form in forms {
            let mut g = f.clone();
            let mut nulls = Vec::new();
            for (i, t) in form.args {
                match t {
                    Some(t) => g.params[i].1 = t,
                    None => nulls.push(i),
                }
            }
            let n = match &form.suffix {
                None => name.to_string(),
                Some(s) => self.fresh_value(&suffixed(name, s)),
            };
            let with_name = with_as.map(|w| match &form.suffix {
                None => w.to_string(),
                Some(s) => suffixed(w, s),
            });
            let with_only = form.with_only || with_as.is_some();
            let (js, with) = self.function_form(&n, &g, owner, with_only, &nulls, with_name.as_deref());
            first.get_or_insert(js);
            if !(with_only && with.is_some()) {
                names.push(n);
            }
            names.extend(with);
        }
        (first.unwrap(), names)
    }

    // A union going in is a value of one of its members, as Gleam writes it:
    // the plain form takes each union's primary member (`with_left(16.0)`),
    // `name_<member>` another of one union no other parameter has
    // (`with_left_string("10%")`), `name_null` a nullable one's null, and
    // `name_of` the unions themselves where there are several (date-fns'
    // `difference_in_days(later, earlier)`), or a union of objects comes back.
    // A union in the optional tail (from `tail`) is in `name_<member>_with`
    // alone. None: no forms.
    pub(super) fn member_forms(&self, params: &[(String, Ty)], rest: bool, tail: usize) -> Vec<Form> {
        #[derive(PartialEq)]
        enum Place {
            Required,
            // In the optional tail: its members are optional too.
            Tail(Absence),
            // May be null: `name_null`.
            Nullable,
        }
        let find = |n: &str| self.m.unions.iter().find(|u| u.name == n && u.type_params.is_empty());
        let unions: Vec<(usize, &NamedUnion, Place)> = params
            .iter()
            .enumerate()
            .filter(|(i, _)| !(rest && *i == params.len() - 1))
            .filter_map(|(i, (_, t))| match t {
                Ty::NamedUnion(n) => find(n).map(|u| (i, u, Place::Required)),
                Ty::Nullable(inner, absent) => match &**inner {
                    Ty::NamedUnion(n) => find(n).map(|u| (i, u, if i >= tail { Place::Tail(*absent) } else { Place::Nullable })),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        if unions.is_empty() {
            return Vec::new();
        }
        let named: Vec<(Option<usize>, Vec<String>)> = unions.iter().map(|(_, u, _)| self.m.member_names(u, &|n| self.tname(n))).collect();
        let primary: Vec<Option<usize>> = named.iter().map(|(p, _)| *p).collect();
        let member = |place: &Place, t: &Ty| match place {
            Place::Tail(absent) => Ty::Nullable(Box::new(t.clone()), *absent),
            _ => t.clone(),
        };
        // Every union but `skip` at its primary member; one in the tail without one stays the union.
        let at_primary = |skip: Option<usize>| -> Option<Vec<(usize, Option<Ty>)>> {
            let mut out = Vec::new();
            for (k, ((i, u, place), p)) in unions.iter().zip(&primary).enumerate() {
                if Some(k) == skip {
                    continue;
                }
                match (p, place) {
                    (Some(p), _) => out.push((*i, Some(member(place, &u.parts[*p])))),
                    (None, Place::Tail(_)) => {}
                    (None, _) => return None,
                }
            }
            Some(out)
        };
        let mut forms = Vec::new();
        if let Some(args) = at_primary(None) {
            forms.push(Form { suffix: None, args, with_only: false });
        }
        let mut suffixes: Vec<String> = Vec::new();
        let mut of = unions.len() > 1;
        for (k, (i, u, place)) in unions.iter().enumerate() {
            if unions.iter().filter(|(_, v, _)| v.name == u.name).count() > 1 {
                of = true;
                continue;
            }
            let Some(others) = at_primary(Some(k)) else { continue };
            let names = &named[k].1;
            let with_only = matches!(place, Place::Tail(_));
            let mut add = |suffix: String, full: Option<String>, t: Option<Ty>, forms: &mut Vec<Form>| {
                // Taken by another parameter's member: by its whole name (node's `BufferEncoding` beside `Buffer`).
                let suffix = match full {
                    Some(full) if suffixes.contains(&suffix) && !suffixes.contains(&full) => full,
                    _ => suffix,
                };
                if suffixes.contains(&suffix) {
                    of = true;
                    return;
                }
                suffixes.push(suffix.clone());
                let mut args = others.clone();
                args.push((*i, t));
                forms.push(Form { suffix: Some(suffix), args, with_only });
            };
            for (m, part) in u.parts.iter().enumerate().filter(|(m, _)| Some(*m) != primary[k]) {
                let full = match part {
                    Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) | Ty::NamedUnion(n) => Some(snake(&self.tname(n))),
                    _ => None,
                };
                add(names[m].clone(), full, Some(member(place, part)), &mut forms);
            }
            if *place == Place::Nullable {
                add("null".into(), None, None, &mut forms);
            }
        }
        // A union of objects that comes back goes back in as it is (the fixture's `area(shape)`).
        let returned = unions.iter().zip(&primary).any(|((_, u, _), p)| p.is_none() && self.given_back.contains(&u.name));
        if of || returned {
            let with_only = unions.iter().all(|(_, _, place)| matches!(place, Place::Tail(_)));
            forms.push(Form { suffix: Some("of".to_string()), args: Vec::new(), with_only });
        }
        forms
    }

    // One external function, `name`, and its JavaScript; returns its JS export
    // name, and the name of its form with its trailing optional arguments
    // where `name` leaves them out (`create_element(doc, tag)`,
    // `create_element_with(doc, tag, Some(options))`): one export for both.
    fn function_form(&mut self, name: &str, f: &Func, owner: Option<(&str, &[TParam])>, with_only: bool, nulls: &[usize], with_as: Option<&str>) -> (String, Option<String>) {
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
        // The optional tail, where `name` leaves it out: `name_with` takes it bare
        // (`vector3.set_with(v, 6.0, 10.0, 8.0)`), but a callback or an interface
        // of methods that ends a tail of two (`world.step_with(w, q, None)`).
        let codecs = self.crossing_params(f, owner);
        let tail = if codecs.is_empty() && nulls.is_empty() { optional_tail(f) } else { f.params.len() };
        // So does one of a type nothing here makes (pts' `CanvasLineJoin`, without the DOM's bindings).
        let kept_open = |i: usize, t: &Ty| {
            matches!(t, Ty::Nullable(inner, _) if matches!(&**inner, Ty::Obj(n) | Ty::Generic(n, _) if self.unmakeable.contains(n)))
                || f.params.len() - tail >= 2 && i == f.params.len() - 1 && matches!(t, Ty::Nullable(inner, _) if self.behaviour(inner))
        };
        for (i, (p, t)) in f.params.iter().enumerate() {
            if nulls.contains(&i) {
                args.push(if let Ty::Nullable(_, a) = t { a.js() } else { "null" }.to_string());
                may_undef.push(false);
                continue;
            }
            // A parameter of the FFI's JavaScript too: `default_` there.
            let mut id = lower(p);
            while JS_RESERVED.contains(&id.as_str()) || !taken.insert(id.clone()) {
                id.push('_');
            }
            let last_rest = f.rest && i == f.params.len() - 1;
            let (param_ty, value) = match t {
                Ty::Nullable(inner, Absence::Undefined) if i >= tail && !kept_open(i, t) => {
                    let (ty, v) = self.going_in(|| (self.ty(inner), self.from_gleam(inner, &id)));
                    (ty, if v == id { v } else { format!("({id} === undefined ? undefined : {v})") })
                }
                _ => self.going_in(|| (self.ty(t), self.from_gleam(t, &id))),
            };
            params.push(format!("{id}: {param_ty}"));
            js_params.push(id.clone());
            // An option bag of the optional arguments: each its own, in order.
            if let Some(props) = self.m.args_bags.get(&f.import).filter(|_| i == f.params.len() - 1) {
                for prop in props {
                    args.push(format!("({id} === undefined ? undefined : {id}[{prop:?}])"));
                    may_undef.push(true);
                }
                continue;
            }
            args.push(if last_rest { format!("...({value} ?? [])") } else { value });
            may_undef.push(!last_rest && matches!(t, Ty::Nullable(_, a) if a.optional()));
        }
        // A codec for each type parameter whose values cross: the class's, then the function's own.
        *self.open.borrow_mut() = None;
        for p in codecs {
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
        let first = usize::from(self_ty.is_some());
        let ret = if f.returns_this { self_ty.unwrap_or_else(|| self.ty(&f.ret)) } else { self.ty(&f.ret) };
        // A name JavaScript reserves (`new`, `delete`) is `new_` there.
        let js = if JS_RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() };
        // Not a setter's value, a rest, or what codecs follow: JavaScript takes those by position.
        let optional = f.params.iter().rev().take_while(|(_, t)| matches!(t, Ty::Nullable(_, a) if a.optional())).count();
        let shortened = optional > 0 && !f.rest && params.len() + nulls.len() == first + f.params.len() && !matches!(f.call, Call::Set(_) | Call::IndexSet);
        let doc = f.doc.as_deref().map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default();
        // `_with`, else `_full` where a member has that name (ProseMirror's `can_replace_with`).
        let free = |g: &Self, n: &str| !g.reserved_values.contains(n) && !g.values.contains(&(g.unit.clone(), n.to_string()));
        let with = shortened.then(|| {
            let with = match with_as {
                Some(w) => w.to_string(),
                None => [format!("{name}_with"), format!("{name}_full")].into_iter().find(|n| free(self, n)).unwrap_or_else(|| format!("{name}_with")),
            };
            self.fresh_value(&with)
        });
        if let Some(with) = &with {
            self.out += &doc;
            if !with_only {
                self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params[..params.len() - optional].join(", "));
            }
            self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {with}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        } else {
            self.out += &doc;
            self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        }
        self.ffi += &format!("export function {js}({}) {{\n  {body}\n}}\n", js_params.join(", "));
        self.clear_fn_bounds();
        (js, with)
    }

    // A union as a custom type, and its conversions: told apart by its tests.
    pub(super) fn union(&mut self, u: &NamedUnion) {
        let name = self.tname(&u.name);
        // Made of a member where it goes in, never read back: no decoder.
        let decodes = u.decodable();
        let params = Self::type_params(&u.type_params);
        let variants: Vec<String> = (0..u.parts.len()).map(|i| self.variants[&(u.name.clone(), i)].clone()).collect();
        let cases: Vec<String> = u.parts.iter().zip(&variants).map(|(p, v)| format!("  {v}({})", if decodes { self.ty(p) } else { self.going_in(|| self.ty(p)) })).collect();
        self.out += &format!("pub type {name}{params} {{\n{}\n}}\n\n", cases.join("\n"));
        // A variant holding a class, made of any subtype: `scene_entity_camera(cam)`,
        // not `SceneEntityCamera(camera.upcast(cam))`.
        if u.type_params.is_empty() {
            for (p, v) in u.parts.iter().zip(&variants) {
                let payload = |g: &Self| if decodes { g.ty(p) } else { g.going_in(|| g.ty(p)) };
                let closed = payload(self);
                *self.open.borrow_mut() = Some(0);
                let open = payload(self);
                *self.open.borrow_mut() = None;
                let id = snake(v);
                if open == closed || id.ends_with("_codec") {
                    continue;
                }
                let id = self.fresh_value(&id);
                self.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(x: {open}) -> {name}\n\n", self.own_ffi());
                self.ffi += &format!("export function {id}(x) {{\n  return new B.{v}(x);\n}}\n");
            }
        }
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
        let from: String = u.parts.iter().zip(&variants).map(|(p, v)| format!("  if (u instanceof B.{v}) return {};\n", if decodes { self.from_gleam(p, "u[0]") } else { self.going_in(|| self.from_gleam(p, "u[0]")) })).collect();
        if !decodes {
            self.ffi += &format!("export function {name}_from(u) {{\n{from}}}\n");
            return;
        }
        let codecs: String = self.union_codecs[&u.name].iter().map(|&i| format!(", {}", codec_var(&u.type_params[i].name))).collect();
        self.ffi += &format!("export function {name}_to(v{codecs}) {{\n{to}}}\nexport function {name}_from(u{codecs}) {{\n{from}}}\n");
    }
}

impl Gen<'_> {
    // A callback, or an object of methods none would make empty (Rapier's
    // `PhysicsHooks`): what a tail may well leave out after another.
    fn behaviour(&self, t: &Ty) -> bool {
        match t {
            Ty::Obj(n) if self.is_callback(n) => true,
            Ty::Obj(n) => self.m.classes.iter().find(|c| c.name == *n).is_some_and(|c| {
                c.funcs.iter().any(|f| matches!(f.call, Call::Method(_))) && !c.funcs.iter().any(|f| matches!(f.call, Call::Set(_) | Call::New(_)))
            }),
            _ => false,
        }
    }
}

// A form of a function for one union member: by parameter, the member type
// it takes, or none for its absent value (`name_null`).
pub(super) struct Form {
    pub suffix: Option<String>,
    pub args: Vec<(usize, Option<Ty>)>,
    // Of a union in the optional tail: only `name_<member>_with` takes it.
    pub with_only: bool,
}

// Where a function's optional tail starts, as `function_form` leaves it out.
fn optional_tail(f: &Func) -> usize {
    if f.rest || matches!(f.call, Call::Set(_) | Call::IndexSet) {
        return f.params.len();
    }
    f.params.len() - f.params.iter().rev().take_while(|(_, t)| matches!(t, Ty::Nullable(_, a) if a.optional())).count()
}
