// XML -> Gleam bindings for Gleam's JavaScript target: modules of opaque
// types and external functions, each with an FFI module that calls the API
// and converts between JavaScript's values and Gleam's.
//
// Gleam modules may not import each other in a cycle. Every type is in
// `bindings/types`; `Promise`, `JsError` and `Codec` in `bindings/runtime`.
// An area's functions, in `bindings/<area>`, import only types, so no
// module imports one that imports it.
//
// Subtypes are phantom types along each class's chain of first bases:
// `Element(a) = Node(ElementTag(a))`. Parameters are open (`Node(a)`, any
// subtype); results, callbacks' parameters and what lists and variants hold
// are closed (`Node(Nil)`). See DESIGN.md.
//
// Gleam has functions and custom types; no methods, subtypes, overloads or
// optional arguments. So, from the shared model (`generators::model`): a class
// is an opaque type and its members functions named for it
// (`shape_get_size(s)`), an ancestor is reached by `shape_as_named(s)`, an
// optional argument or a nullable value is an `Option(a)`, an array a
// `List(a)`, a tuple a tuple, a union or string enum a custom type, a
// callback a Gleam function, a promise a `Promise(a)` to `promise_then`.
// Gleam's values are JavaScript's, but for these: the FFI converts them.

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use roxmltree::Document;

use super::js::{access, global, js_test};
use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, model, subst, walk, with_self_args};
use super::names::{Kind, Naming};

const KEYWORDS: &[&str] = &[
    "as", "assert", "auto", "case", "const", "delegate", "derive", "echo", "else", "fn", "if", "implement", "import", "let", "macro", "opaque", "panic", "pub", "test", "todo", "type", "use",
];

// Types the bindings declare whatever the API: what a module of Gleam can
// name of JavaScript's values. Not opaque types of the API.
const OWN: &[&str] = &["Promise", "JsError", "Dynamic", "Option", "List", "Result", "Nil", "Int", "Float", "String", "Bool", "BitArray", "Codec"];

// The fixed values of every module of bindings.
const OWN_VALUES: &[&str] = &[
    "promise_then", "attempt", "js_error_message", "dynamic", "codec_float", "codec_string", "codec_bool", "codec_dynamic", "codec_option", "codec_list",
];

// Types (and variants) and values are apart; a type's functions, constants,
// upcasts and `to_list` are values named for it: `shape_area`.
pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().chain(OWN).chain(OWN_VALUES).map(|k| k.to_string()).collect(),
    // An external `Promise` is Gleam's own; a class the API declares as
    // `Option` is not, and gives way.
    type_names: |candidate, _, kind, _| match kind {
        Kind::Opaque if OWN.contains(&candidate) || candidate == "PromiseLike" => Vec::new(),
        // A class's marker for the phantom of its subtypes.
        Kind::Class => vec![upper(candidate), format!("{}Tag", upper(candidate))],
        _ => vec![upper(candidate)],
    },
    // Every type has a codec in the types module: `shape_codec`.
    values_of: |own, _, _, _| vec![lower(&format!("{}_codec", snake(own)))],
    types_apart: true,
    overloads: false,
    // A class's functions are in its own module: `shape.area`.
    func_name: |_, name, f| Some(lower(&function_name(f, name))),
    funcs_flat: false,
    nests: false,
    // Beside its functions there: its constants, list helpers and casts.
    members: |c, m| {
        let mut out: Vec<String> = c.consts.iter().map(|(k, _)| lower(k)).collect();
        out.push("upcast".into());
        if c.iterates.is_some() || c.elements.is_some() {
            out.push("to_list".into());
        }
        if c.elements.is_some() {
            out.extend(["get_at", "length"].map(String::from));
        }
        if c.js.is_some() {
            out.extend(["from", "cast"].map(String::from));
        }
        out.extend(secondary(m, c).iter().map(|a| lower(&format!("as_{}", snake(&upper(a))))));
        out
    },
};

pub struct Output {
    pub files: Vec<(String, String)>,
    pub skipped: BTreeMap<String, usize>,
    // What each binding reaches, for a page to check (`wasm::manifest`).
    pub manifest: String,
}

// `getElementById` -> `get_element_by_id`, `HTMLElement` -> `html_element`.
fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1);
            let after_word = prev.is_some_and(|p| p.is_lowercase() || (p.is_ascii_digit() && next.is_some_and(|n| n.is_lowercase())));
            let ends_acronym = prev.is_some_and(|p| p.is_uppercase()) && next.is_some_and(|n| n.is_lowercase());
            if (after_word || ends_acronym) && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else if c.is_ascii_alphanumeric() {
            out.push(c);
        } else if !out.ends_with('_') {
            out.push('_');
        }
    }
    out
}

// A Gleam name for a value: snake_case, starting with a letter, not a keyword.
fn lower(name: &str) -> String {
    let s = snake(name).trim_matches('_').to_string();
    let s = if s.starts_with(|c: char| c.is_ascii_lowercase()) { s } else { format!("x{s}") };
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// A Gleam name for a type or a variant: UpperCamelCase, letters and digits only.
fn upper(name: &str) -> String {
    let out: String = name
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut cs = s.chars();
            cs.next().map(|c| c.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        })
        .collect();
    if out.starts_with(|c: char| c.is_ascii_uppercase()) { out } else { format!("X{out}") }
}

struct Gen<'m> {
    m: &'m Model,
    // Gleam's name of each type the model names, and of each union's or enum's variants.
    types: HashMap<String, String>,
    variants: HashMap<(String, usize), String>,
    // Each callback, by the places of the type parameters it uses.
    callbacks: HashMap<String, Vec<usize>>,
    // Values given out, by module; and names no module's value may have.
    values: HashSet<(String, String)>,
    reserved_values: HashSet<String>,
    // The module being written: its Gleam, its JavaScript, and the Gleam
    // types its Gleam names (for its imports). Those written, by path.
    unit: String,
    out: String,
    ffi: String,
    used: RefCell<BTreeSet<String>>,
    units: BTreeMap<String, Unit>,
    // The module each Gleam type is declared in.
    home: HashMap<String, String>,
    // Each class's type parameter count; and the class its chain continues
    // from, with the arguments it gives it.
    classes: HashMap<String, usize>,
    parents: HashMap<String, (String, Vec<Ty>)>,
    // Where a class's type is open, `Element(a)`, any subtype (a function's
    // parameters): the variables given out. None where it is closed,
    // `Element(Nil)`, that type and no more (what comes back, what a callback
    // is given, what a list or variant holds).
    open: RefCell<Option<usize>>,
    // A function's own type parameters bound to a class (`T extends Node`):
    // that class, open in the parameter's variable.
    bound: RefCell<HashMap<String, (String, Vec<Ty>)>>,
    skipped: BTreeMap<String, usize>,
}

#[derive(Default)]
struct Unit {
    out: String,
    ffi: String,
    used: BTreeSet<String>,
}

impl<'m> Gen<'m> {
    fn skip(&mut self, why: &str) {
        *self.skipped.entry(why.to_string()).or_default() += 1;
    }

    // A module-level value's name: `get_size` in a class's module.
    // `names::settle` made them apart; one given out twice in a module is a
    // mistake in its naming.
    fn value_name(&mut self, base: &str) -> String {
        let name = lower(base);
        assert!(!self.reserved_values.contains(&name) && self.values.insert((self.unit.clone(), name.clone())), "Gleam: the value `{name}` is named twice in {}", self.unit);
        name
    }

    // A value's name in the module being written, the first of `base`,
    // `base_2`, ... it has not given out, and no keyword.
    fn fresh_value(&mut self, base: &str) -> String {
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

    fn tname(&self, n: &str) -> String {
        self.types.get(n).cloned().unwrap_or_else(|| upper(n))
    }

    // A type's name, written in the module's Gleam: noted, for its import.
    fn gt(&self, n: &str) -> String {
        let t = self.tname(n);
        self.note(&t);
        t
    }

    fn note(&self, gleam_type: &str) {
        self.used.borrow_mut().insert(gleam_type.to_string());
    }

    // Writes to another module from here on.
    fn switch(&mut self, unit: &str) {
        let done = Unit { out: std::mem::take(&mut self.out), ffi: std::mem::take(&mut self.ffi), used: self.used.take() };
        let was = self.units.entry(std::mem::replace(&mut self.unit, unit.to_string())).or_default();
        was.out += &done.out;
        was.ffi += &done.ffi;
        was.used.extend(done.used);
        let next = self.units.remove(unit).unwrap_or_default();
        self.out = next.out;
        self.ffi = next.ffi;
        *self.used.borrow_mut() = next.used;
    }

    // The module's own JavaScript, as an external names it.
    fn own_ffi(&self) -> String {
        format!("./{}_ffi.mjs", self.unit.rsplit('/').next().unwrap_or_default())
    }

    // The runtime's JavaScript, from the module being written.
    fn runtime_ffi(&self) -> String {
        relative(&self.unit, "bindings/runtime_ffi.mjs")
    }

    // A generic type named without its arguments: `Polygon(Dynamic)`.
    fn bare(&self, n: &str) -> String {
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

    fn tvar(n: &str) -> String {
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
    fn closed<T>(&self, f: impl FnOnce() -> T) -> T {
        let was = self.open.replace(None);
        let out = f();
        *self.open.borrow_mut() = was;
        out
    }

    // A class's type with its arguments (`Dynamic` for those not given) and
    // its last, phantom one.
    fn class_type(&self, n: &str, args: &[Ty]) -> String {
        let count = self.classes[n];
        let mut parts: Vec<String> = (0..count).map(|i| match args.get(i) {
            Some(a) => self.closed(|| self.ty(a)),
            None => {
                self.note("Dynamic");
                "Dynamic".to_string()
            }
        }).collect();
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
    fn shows(&self, t: &Ty, param: &str) -> bool {
        match t {
            Ty::Param(n) => n == param,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) => self.shows(inner, param),
            Ty::Tuple(es) => es.iter().any(|e| self.shows(e, param)),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => false,
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => self.callbacks[n].iter().filter_map(|&i| args.get(i)).any(|a| self.shows(a, param)),
            Ty::Generic(_, args) => args.iter().any(|a| self.shows(a, param)),
            _ => false,
        }
    }

    // Gleam's type of a value.
    fn ty(&self, t: &Ty) -> String {
        let dynamic = || {
            self.note("Dynamic");
            "Dynamic".to_string()
        };
        match t {
            Ty::Void => "Nil".into(),
            Ty::Num => "Float".into(),
            Ty::Bool => "Bool".into(),
            Ty::Str => "String".into(),
            Ty::Any | Ty::Union(_) | Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => dynamic(),
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
            Ty::Param(n) => match self.bound.borrow().get(n).cloned() {
                Some((class, args)) => {
                    let inner = self.closed(|| args.iter().map(|a| self.ty(a)).collect::<Vec<_>>());
                    let mut parts = inner;
                    parts.extend((parts.len()..self.classes[&class]).map(|_| "Dynamic".to_string()));
                    parts.push(Self::tvar(n));
                    format!("{}({})", self.gt(&class), parts.join(", "))
                }
                None => Self::tvar(n),
            },
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
    fn to_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            // A type parameter's values cross by the codec the function was given.
            Ty::Param(n) if self.bound.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) => format!("{}.to({x})", codec_var(n)),
            Ty::Void => format!("({x}, undefined)"),
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
        let used: Vec<usize> = match self.callbacks.get(n) {
            Some(used) => used.clone(),
            None => (0..args.len()).collect(),
        };
        // Named without its arguments, it is of `Dynamic`: values as they are.
        used.iter().map(|&i| format!(", {}", args.get(i).map_or("$same".to_string(), |a| self.codec_expr(a)))).collect()
    }

    // A JavaScript codec for values of `t`: how they cross each way.
    fn codec_expr(&self, t: &Ty) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) => "$same".into(),
            Ty::Param(n) => codec_var(n),
            t if self.to_gleam(t, "v") == "v" && self.from_gleam(t, "v") == "v" => "$same".into(),
            t => format!("{{ to: (v) => {}, from: (v) => {} }}", self.to_gleam(t, "v"), self.from_gleam(t, "v")),
        }
    }

    // Whether a type parameter's values cross where `t` is: bare, in an
    // option, list, tuple or promise, through a callback or a union. Not
    // inside a handle (`Box(t)`), which crosses as it is.
    fn crosses(&self, t: &Ty, p: &str) -> bool {
        if self.bound.borrow().contains_key(p) {
            return false;
        }
        match t {
            Ty::Param(n) => n == p,
            Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => self.crosses(inner, p),
            Ty::Tuple(es) | Ty::Union(es) => es.iter().any(|e| self.crosses(e, p)),
            Ty::Generic(n, args) if self.is_callback(n) || self.decodable(n) || n == "Promise" || n == "PromiseLike" => args.iter().any(|a| mentions(a, p)),
            _ => false,
        }
    }

    // A JavaScript expression that is `x`, Gleam's value, as JavaScript has it.
    fn from_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            Ty::Param(n) if self.bound.borrow().contains_key(n) => x.to_string(),
            Ty::Param(n) => format!("{}.from({x})", codec_var(n)),
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

    fn type_params(params: &[TParam]) -> String {
        if params.is_empty() { String::new() } else { format!("({})", params.iter().map(|p| Self::tvar(&p.name)).collect::<Vec<_>>().join(", ")) }
    }

    // One external function, `name`, and its JavaScript.
    fn function(&mut self, name: &str, f: &Func, owner: Option<(&str, &[TParam])>) {
        // Its own type parameters bound to a class are that class's type, open
        // in the parameter: `append_child(self: Node(a), node: Node(t)) -> Node(t)`.
        *self.bound.borrow_mut() = f
            .type_params
            .iter()
            .filter_map(|p| match &p.constraint {
                Some(Ty::Obj(c)) if self.classes.contains_key(c) => Some((p.name.clone(), (c.clone(), Vec::new()))),
                Some(Ty::Generic(c, args)) if self.classes.contains_key(c) => Some((p.name.clone(), (c.clone(), args.clone()))),
                _ => None,
            })
            .collect();
        let mut params = Vec::new();
        let mut js_params = Vec::new();
        // A class's type is open in the parameters: any subtype.
        *self.open.borrow_mut() = Some(0);
        if let Some((class, tps)) = owner.filter(|_| Class::has_self(f)) {
            let own: Vec<Ty> = tps.iter().map(|p| Ty::Param(p.name.clone())).collect();
            params.push(format!("self: {}", self.class_type(class, &own)));
            js_params.push("self".to_string());
        }
        let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
        let mut args = Vec::new();
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
        }
        // A codec for each type parameter whose values cross: the class's, then the function's own.
        *self.open.borrow_mut() = None;
        let class_params: &[TParam] = owner.map_or(&[], |o| o.1);
        let mut seen = HashSet::new();
        for p in class_params.iter().chain(&f.type_params).filter(|p| seen.insert(p.name.clone())) {
            if f.params.iter().map(|x| &x.1).chain([&f.ret]).any(|t| self.crosses(t, &p.name)) {
                self.note("Codec");
                params.push(format!("{}_codec: Codec({})", lower(&p.name), Self::tvar(&p.name)));
                js_params.push(codec_var(&p.name));
            }
        }
        for (position, literal) in &f.fixed {
            args.insert((*position).min(args.len()), literal.clone());
        }
        let (first, second) = (args.first().cloned().unwrap_or_default(), args.get(1).cloned().unwrap_or_default());
        let list = args.join(", ");
        let call = match &f.call {
            Call::Method(js) => format!("self{}({list})", access(js)),
            Call::Get(p) => format!("self{}", access(p)),
            Call::GetBound(p) => format!("((v) => typeof v === \"function\" ? v.bind(self) : v)(self{})", access(p)),
            Call::Set(p) => format!("self{} = {list}", access(p)),
            Call::New(path) => format!("new ({})({list})", global(path)),
            Call::Static(path) => format!("{}({list})", global(path)),
            Call::StaticGet(path) => global(path),
            // A top-level variable is assigned where it is: a module's cannot be.
            Call::StaticSet(path) if !path.contains('.') && !path.contains("::") => format!("globalThis{} = {list}", access(path)),
            Call::StaticSet(path) => format!("{} = {list}", global(path)),
            Call::Create(literal) => format!("({literal})"),
            Call::Invoke => format!("self({list})"),
            Call::Construct => format!("new self({list})"),
            Call::IndexGet => format!("self[{first}]"),
            Call::IndexSet => format!("self[{first}] = {second}"),
            Call::Apply => format!("({first})({})", args.get(1..).unwrap_or_default().join(", ")),
        };
        let body = match &f.ret {
            Ty::Void => format!("{call};\n  return undefined;"),
            t => format!("return {};", self.to_gleam(t, &call)),
        };
        let ret = self.ty(&f.ret);
        // A name JavaScript reserves (`new`, `delete`) is `new_` there.
        let js = if JS_RESERVED.contains(&name) { format!("{name}_") } else { name.to_string() };
        self.out += &format!("@external(javascript, \"{}\", \"{js}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        self.ffi += &format!("export function {js}({}) {{\n  {body}\n}}\n", js_params.join(", "));
        self.bound.borrow_mut().clear();
    }

    // A union as a custom type, and its conversions: told apart by its tests.
    fn union(&mut self, u: &NamedUnion) {
        let name = self.tname(&u.name);
        let params = Self::type_params(&u.type_params);
        let variants: Vec<String> = (0..u.parts.len()).map(|i| self.variants[&(u.name.clone(), i)].clone()).collect();
        let cases: Vec<String> = u.parts.iter().zip(&variants).map(|(p, v)| format!("  {v}({})", self.ty(p))).collect();
        self.out += &format!("pub type {name}{params} {{\n{}\n}}\n\n", cases.join("\n"));
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
        let codecs: String = u.type_params.iter().map(|p| format!(", {}", codec_var(&p.name))).collect();
        self.ffi += &format!("export function {name}_to(v{codecs}) {{\n{to}}}\nexport function {name}_from(u{codecs}) {{\n{from}}}\n");
    }
}

// The bindings of the XML, or what Gleam cannot express of it.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let root_import = doc.root_element().attribute("import").map(str::to_string);
    let mut m = model(xml, "Gleam")?;
    take_secondary(&mut m);
    super::names::settle(&mut m, &NAMING);
    break_cycles(&mut m);
    let mut g = Gen {
        m: &m,
        types: HashMap::new(),
        variants: HashMap::new(),
        callbacks: HashMap::new(),
        values: HashSet::new(),
        reserved_values: HashSet::new(),
        unit: RUNTIME.into(),
        out: String::new(),
        ffi: String::new(),
        used: RefCell::default(),
        units: BTreeMap::new(),
        home: HashMap::new(),
        classes: m.classes.iter().map(|c| (c.name.clone(), c.type_params.len())).collect(),
        parents: m.classes.iter().filter_map(|c| parent(&m, c).map(|p| (c.name.clone(), p))).collect(),
        open: RefCell::new(None),
        bound: RefCell::default(),
        skipped: m.skipped.clone(),
    };

    // Names: every type (`names::settle` made them apart), then every
    // variant of a union or enum, apart from them and each other.
    let mut type_names: HashSet<String> = OWN.iter().map(|s| s.to_string()).collect();
    let mut name_type = |n: &str, g: &mut Gen| {
        let name = upper(n);
        assert!(type_names.insert(name.clone()), "Gleam: the type `{name}` is named twice");
        g.types.insert(n.to_string(), name);
    };
    for c in &m.classes {
        name_type(&c.name, &mut g);
    }
    for c in &m.callbacks {
        name_type(&c.name, &mut g);
        g.callbacks.insert(c.name.clone(), (0..c.type_params.len()).collect());
    }
    // Gleam allows no unused type parameter: `this` of a callback has gone, and
    // what is `Dynamic` shows none. A callback may hold another: until settled.
    loop {
        let mut changed = false;
        for c in &m.callbacks {
            let used: Vec<usize> = (0..c.type_params.len()).filter(|&i| c.params.iter().map(|x| &x.1).chain([&c.ret]).any(|t| g.shows(t, &c.type_params[i].name))).collect();
            if used != g.callbacks[&c.name] {
                g.callbacks.insert(c.name.clone(), used);
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
    for u in &m.unions {
        name_type(&u.name, &mut g);
    }
    for (e, _) in &m.enums {
        name_type(e, &mut g);
    }
    for (e, _) in &m.num_enums {
        name_type(e, &mut g);
    }
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike") {
        name_type(o, &mut g);
    }
    g.types.insert("PromiseLike".into(), "Promise".into());
    // A variant by its own name where that is free, else its type's and its own.
    let mut variant_names: Vec<(String, usize, String)> = Vec::new();
    for u in m.unions.iter().filter(|u| u.decodable()) {
        for (i, p) in u.parts.iter().enumerate() {
            let own = match p {
                Ty::Num => "Number".to_string(),
                Ty::Str => "String".to_string(),
                Ty::Bool => "Boolean".to_string(),
                Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "Array".to_string(),
                Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) => g.tname(n),
                Ty::Param(n) => upper(n),
                _ => format!("Case{i}"),
            };
            variant_names.push((u.name.clone(), i, format!("{}{own}", g.tname(&u.name))));
        }
    }
    for (e, values) in &m.enums {
        for (i, v) in values.iter().enumerate() {
            variant_names.push((e.clone(), i, format!("{}{}", g.tname(e), upper(if v.is_empty() { "empty" } else { v }))));
        }
    }
    for (e, members) in &m.num_enums {
        for (i, (member, _)) in members.iter().enumerate() {
            variant_names.push((e.clone(), i, format!("{}{}", g.tname(e), upper(member))));
        }
    }
    for (owner, i, base) in variant_names {
        let mut name = base.clone();
        let mut n = 2;
        while !type_names.insert(name.clone()) {
            name = format!("{base}{n}");
            n += 1;
        }
        g.variants.insert((owner, i), name);
    }
    // What the FFI's own helpers are called: not a function's name.
    for reserved in ["promise_then", "attempt", "js_error_message", "dynamic"] {
        g.reserved_values.insert(reserved.into());
    }

    // Each area's modules: `areas/main` is `bindings/areas/main` (its
    // functions) and `bindings/types/areas_main` (its classes).
    let mut paths: HashMap<String, String> = HashMap::new();
    let mut taken: HashSet<String> = HashSet::from([RUNTIME.to_string(), TYPES.to_string()]);
    let areas: BTreeSet<&String> = m.classes.iter().map(|c| &c.module).chain(m.funcs.iter().map(|f| &f.module)).collect();
    for area in areas {
        let segments: Vec<String> = area.split('/').filter(|s| !s.is_empty()).map(lower).collect();
        let mut path = format!("bindings/{}", segments.join("/"));
        let mut n = 2;
        while !taken.insert(path.clone()) {
            path = format!("bindings/{}_{n}", segments.join("/"));
            n += 1;
        }
        paths.insert(area.clone(), path);
    }
    // Each class's module, in its area's: `bindings/areas/main/shape`.
    let mut class_paths: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        // A namespace's class by its own name: `ns.Node`'s is `namespaces/ns/node`.
        let own = m.namespaced.get(&c.name).map(|p| {
            let depth = m.namespace_modules.get(&c.name).map_or(1, |(_, d)| *d).min(p.len() - 1);
            p[depth..].iter().map(|s| snake(&upper(s))).collect::<Vec<_>>().join("/")
        });
        let mut segment = own.unwrap_or_else(|| snake(&g.tname(&c.name)));
        if KEYWORDS.contains(&segment.as_str()) {
            segment.push('_');
        }
        // Its namespace's module, a package's in the package's (`areas/pb/util`).
        let module = m.namespace_modules.get(&c.name).map_or(&c.module, |(m, _)| m);
        let base = format!("{}/{segment}", module_path(&paths, module));
        let mut path = base.clone();
        let mut n = 2;
        while !taken.insert(path.clone()) {
            path = format!("{base}_{n}");
            n += 1;
        }
        class_paths.insert(c.name.clone(), path);
    }
    for o in OWN_RUNTIME {
        g.home.insert(o.to_string(), RUNTIME.into());
    }
    for c in &m.classes {
        g.home.insert(g.tname(&c.name), TYPES.into());
        g.home.insert(format!("{}Tag", g.tname(&c.name)), TYPES.into());
    }
    let others = m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)).chain(m.callbacks.iter().map(|c| &c.name)).chain(m.unions.iter().map(|u| &u.name)).chain(&m.opaque);
    for n in others.filter(|n| !OWN.contains(&n.as_str()) && *n != "PromiseLike") {
        g.home.insert(g.tname(n), TYPES.into());
    }

    // The runtime: what every module may use.
    g.out += "/// A JavaScript promise: `promise_then` gives its result to a function.\npub type Promise(a)\n\n\
              /// A JavaScript exception, or the reason a promise was rejected.\npub type JsError\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"promise_then\")\npub fn promise_then(promise: Promise(a), then: fn(Result(a, JsError)) -> Nil) -> Nil\n\n\
              /// Runs `f`, and gives what it returned, or the exception it threw.\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"attempt\")\npub fn attempt(f: fn() -> a) -> Result(a, JsError)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"js_error_message\")\npub fn js_error_message(error: JsError) -> String\n\n\
              /// Any value, untyped: what a value of no particular type is made from.\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"dynamic\")\npub fn dynamic(value: a) -> Dynamic\n\n\
              /// How a type's values cross between Gleam and JavaScript: what a generic\n\
              /// function takes for each type parameter whose values cross (`box_value(b,\n\
              /// codec_float())`). Each type has one: `shape_codec()`, `mode_codec()`.\n\
              pub type Codec(a)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_same\")\npub fn codec_float() -> Codec(Float)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_same\")\npub fn codec_string() -> Codec(String)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_same\")\npub fn codec_bool() -> Codec(Bool)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_same\")\npub fn codec_dynamic() -> Codec(Dynamic)\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_option\")\npub fn codec_option(of: Codec(a)) -> Codec(Option(a))\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_list\")\npub fn codec_list(of: Codec(a)) -> Codec(List(a))\n\n\
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_tuple2\")\npub fn codec_tuple2(a: Codec(a), b: Codec(b)) -> Codec(#(a, b))\n\n";
    g.note("Dynamic");
    g.note("Option");
    // The modules the program reaches: not a type-only one (node's `stream/iter`).
    let used = m.runtime_packages();
    let mut packages: Vec<&String> = m.packages.values().filter(|p| used.contains(*p)).collect();
    packages.sort();
    packages.dedup();
    let mut imports = String::new();
    let mut modules = Vec::new();
    for (i, p) in packages.iter().enumerate() {
        imports += &format!("import * as P{i} from {p:?};\n");
        modules.push(format!("{p:?}: P{i}"));
    }
    if let Some(root) = root_import.as_ref().filter(|r| !packages.contains(r)) {
        imports += &format!("import * as ROOT from {root:?};\n");
        modules.push(format!("{root:?}: ROOT"));
    }
    g.ffi += &format!(
        "{imports}\n\
         // Where the API's names are: its module, or globals; with several packages, each one's.\n\
         export const PK = {{ {} }};\nexport const G = {};\n\n\
         export function promise_then(p, f) {{\n  Promise.resolve(p).then((v) => f(new Ok(v)), (e) => f(new GleamError(e)));\n}}\n\
         export function attempt(f) {{\n  try {{\n    return new Ok(f());\n  }} catch (e) {{\n    return new GleamError(e);\n  }}\n}}\n\
         export function js_error_message(e) {{\n  return String(e instanceof Error ? e.message : e?.message ?? e);\n}}\n\
         export function dynamic(v) {{\n  return v;\n}}\n\
         // Codecs: how a type parameter's values cross, each way.\n\
         export const $same = {{ to: (v) => v, from: (v) => v }};\n\
         export function codec_same() {{\n  return $same;\n}}\n\
         export function codec_option(c) {{\n  return {{ to: (v) => (v == null ? new None() : new Some(c.to(v))), from: (v) => (v instanceof Some ? c.from(v[0]) : undefined) }};\n}}\n\
         export function codec_list(c) {{\n  return {{ to: (v) => toList(Array.from(v ?? [], (x) => c.to(x))), from: (v) => v.toArray().map((x) => c.from(x)) }};\n}}\n\
         export function codec_tuple2(a, b) {{\n  return {{ to: (v) => [a.to(v[0]), b.to(v[1])], from: (v) => [a.from(v[0]), b.from(v[1])] }};\n}}\n\
         // A Gleam function made a JavaScript function, the same one each time: one\n\
         // listener added and removed is one function.\n\
         const $wrapped = new WeakMap();\n\
         export function $wrap(f, make) {{\n  if (typeof f !== \"function\") return f;\n  let w = $wrapped.get(f);\n  if (!w) $wrapped.set(f, (w = make(f)));\n  return w;\n}}\n",
        modules.join(", "),
        root_import.as_ref().map_or("globalThis".to_string(), |r| format!("PK[{r:?}]"))
    );

    // The types that may refer to each other: externals, enums, unions, callbacks.
    g.switch(TYPES);
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike") {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        let params = if arity == 0 { String::new() } else { format!("({})", (1..=arity).map(|i| format!("t{i}")).collect::<Vec<_>>().join(", ")) };
        g.out += &format!("pub type {}{params}\n\n", g.tname(o));
    }
    // String and numeric enums: custom types without fields.
    for (e, values) in &m.enums {
        let name = g.tname(e);
        let variants: Vec<String> = (0..values.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &format!("pub type {name} {{\n{}\n}}\n\n", variants.iter().map(|v| format!("  {v}")).collect::<Vec<_>>().join("\n"));
        g.ffi += &format!(
            "const {name}_values = {:?};\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  const i = {name}_values.indexOf(v);\n  if (i < 0) throw new TypeError(\"not a {name}: \" + v);\n  return new ({name}_cases()[i])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
            values,
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    for (e, members) in &m.num_enums {
        let name = g.tname(e);
        let variants: Vec<String> = (0..members.len()).map(|i| g.variants[&(e.clone(), i)].clone()).collect();
        g.out += &format!("pub type {name} {{\n{}\n}}\n\n", variants.iter().map(|v| format!("  {v}")).collect::<Vec<_>>().join("\n"));
        g.ffi += &format!(
            "const {name}_values = [{}];\nconst {name}_cases = () => [{}];\n\
             export function {name}_to(v) {{\n  return new ({name}_cases()[{name}_values.indexOf(v)])();\n}}\n\
             export function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
            members.iter().map(|(_, v)| v.clone()).collect::<Vec<_>>().join(", "),
            variants.iter().map(|v| format!("B.{v}")).collect::<Vec<_>>().join(", ")
        );
    }
    // Callbacks: Gleam functions, made JavaScript's and back.
    for c in &m.callbacks {
        let name = g.tname(&c.name);
        let used: Vec<TParam> = g.callbacks[&c.name].iter().map(|&i| c.type_params[i].clone()).collect();
        let params = Gen::type_params(&used);
        let args: Vec<String> = c.params.iter().map(|(_, t)| g.ty(t)).collect();
        let ret = g.ty(&c.ret);
        g.out += &format!("pub type {name}{params} =\n  fn({}) -> {ret}\n\n", args.join(", "));
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let into_gleam: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.to_gleam(t, a)).collect();
        let mut into_js: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.from_gleam(t, a)).collect();
        let codecs: String = used.iter().map(|p| format!(", {}", codec_var(&p.name))).collect();
        // The rest of the arguments: gathered into a list, and spread back.
        let mut gathered = names.clone();
        if c.rest {
            if let Some(last) = gathered.last_mut() {
                *last = format!("...{last}");
            }
            if let Some(last) = into_js.last_mut() {
                *last = format!("...{last}");
            }
        }
        g.ffi += &format!(
            "export function {name}_from(f{codecs}) {{\n  return $wrap(f, (f) => ({}) => {});\n}}\n\
             export function {name}_to(f{codecs}) {{\n  return typeof f === \"function\" ? ({}) => {} : f;\n}}\n",
            gathered.join(", "),
            g.from_gleam(&c.ret, &format!("f({})", into_gleam.join(", "))),
            names.join(", "),
            g.to_gleam(&c.ret, &format!("f({})", into_js.join(", "))),
        );
    }
    for u in m.unions.iter().filter(|u| u.decodable()) {
        g.union(u);
    }
    // Each type's codec, beside it. A handle crosses as it is; an enum, a
    // union or a callback converts, a generic one by its type parameters' codecs.
    let mut codecs: Vec<(String, Vec<TParam>, bool, String)> = Vec::new();
    codecs.extend(m.enums.iter().map(|e| (e.0.clone(), Vec::new(), true, TYPES.to_string())));
    codecs.extend(m.num_enums.iter().map(|e| (e.0.clone(), Vec::new(), true, TYPES.to_string())));
    codecs.extend(m.unions.iter().filter(|u| u.decodable()).map(|u| (u.name.clone(), u.type_params.clone(), true, TYPES.to_string())));
    codecs.extend(m.callbacks.iter().map(|c| (c.name.clone(), g.callbacks[&c.name].iter().map(|&i| c.type_params[i].clone()).collect(), true, TYPES.to_string())));
    for o in m.opaque.iter().filter(|o| !OWN.contains(&o.as_str()) && *o != "PromiseLike") {
        let arity = m.opaque_arity.get(o).copied().unwrap_or(0);
        codecs.push((o.clone(), (1..=arity).map(|i| TParam { name: format!("t{i}"), default: None, constraint: None }).collect(), false, TYPES.to_string()));
    }
    // Classes. A class with no class for a base is an opaque type, its last
    // parameter a phantom: `Node(a)`. One with a base is its base's type,
    // with a marker of its own for the phantom: `Element(a) = Node(ElementTag(a))`.
    // So an `Element(Nil)` is a `Node(ElementTag(Nil))`: a Node to any function
    // that takes a `Node(a)`, and nothing more than an Element.
    // A base's arguments may not name the class, nor one below it on its
    // chain (`BitmapFont extends AbstractBitmapFont<BitmapFont>`): an alias
    // in terms of itself. Those are `Dynamic`.
    let mut below: HashMap<String, HashSet<String>> = HashMap::new();
    for c in &m.classes {
        let mut at = c.name.clone();
        below.entry(at.clone()).or_default().insert(c.name.clone());
        while let Some((p, _)) = g.parents.get(&at) {
            below.entry(p.clone()).or_default().insert(c.name.clone());
            at = p.clone();
        }
    }
    for c in &m.classes {
        let name = g.tname(&c.name);
        let mut vars: Vec<String> = c.type_params.iter().map(|p| Gen::tvar(&p.name)).collect();
        vars.push(PHANTOM.into());
        let vars = vars.join(", ");
        match g.parents.get(&c.name).cloned() {
            None => g.out += &format!("pub type {name}({vars})\n\n"),
            Some((base, args)) => {
                let own = &below[&c.name];
                let args: Vec<Ty> = args.iter().map(|a| walk(a, &|t| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if own.contains(n)).then_some(Ty::Any))).collect();
                let mut parts: Vec<String> = g.closed(|| args.iter().map(|a| g.ty(a)).collect());
                while parts.len() < g.classes[&base] {
                    g.note("Dynamic");
                    parts.push("Dynamic".into());
                }
                parts.push(format!("{name}Tag({vars})"));
                g.out += &format!("pub type {name}Tag({vars})\n\npub type {name}({vars}) = {}({})\n\n", g.gt(&base), parts.join(", "));
            }
        }
        let mut params = c.type_params.clone();
        params.push(TParam { name: PHANTOM.into(), default: None, constraint: None });
        codecs.push((c.name.clone(), params, false, TYPES.to_string()));
    }
    for (n, params, converts, unit) in codecs {
        g.switch(&unit);
        let name = g.tname(&n);
        let id = g.value_name(&format!("{}_codec", snake(&name)));
        let vars = Gen::type_params(&params);
        g.note("Codec");
        if !converts {
            g.out += &format!("@external(javascript, \"{}\", \"codec_same\")\npub fn {id}() -> Codec({name}{vars})\n\n", g.runtime_ffi());
            continue;
        }
        let gleam_params: Vec<String> = params.iter().map(|p| format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name))).collect();
        let js: String = params.iter().map(|p| format!(", {}", codec_var(&p.name))).collect();
        g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}({}) -> Codec({name}{vars})\n\n", g.own_ffi(), gleam_params.join(", "));
        g.ffi += &format!(
            "export function {id}({}) {{\n  return {{ to: (v) => {name}_to(v{js}), from: (u) => {name}_from(u{js}) }};\n}}\n",
            js.trim_start_matches(", ")
        );
    }
    // A class's functions, in its own module.
    for c in &m.classes {
        g.switch(&class_paths[&c.name]);
        let own: Vec<Ty> = c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect();
        // The class's type, open (any subtype) or closed.
        let open = |g: &Gen| {
            *g.open.borrow_mut() = Some(0);
            let t = g.class_type(&c.name, &own);
            *g.open.borrow_mut() = None;
            t
        };
        for (constant, value) in &c.consts {
            let gleam = if value.starts_with('"') {
                value.clone()
            } else if value == "true" || value == "false" {
                upper(value)
            } else if let Ok(n) = value.parse::<f64>() {
                format!("{n:?}")
            } else if let Some(hex) = value.strip_prefix("0x").or(value.strip_prefix("0X")) {
                format!("{:?}", i64::from_str_radix(hex, 16).unwrap_or(0) as f64)
            } else {
                g.skip("constant of another kind");
                continue;
            };
            let id = g.value_name(constant);
            g.out += &format!("pub const {id} = {gleam}\n\n");
        }
        for f in &c.funcs {
            let f = with_self_args(f, c);
            let id = g.value_name(&function_name(&f, &f.name));
            g.function(&id, &f, Some((&c.name, &c.type_params)));
        }
        // What it iterates, or holds as a subclass of Array: a list.
        if let Some(elem) = c.iterates.as_ref().or(c.elements.as_ref()) {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            let id = g.value_name("to_list");
            let elem_ty = g.ty(&elem);
            let this = open(&g);
            let mut params = vec![format!("self: {this}")];
            let mut js_params = vec!["self".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}({}) -> List({elem_ty})\n\n", g.own_ffi(), params.join(", "));
            g.ffi += &format!("export function {id}({}) {{\n  return toList(Array.from(self, (v) => {}));\n}}\n", js_params.join(", "), g.to_gleam(&elem, "v"));
        }
        // A subclass of Array has no class to inherit Array's members from:
        // its elements by index, and how many.
        if let Some(elem) = c.elements.as_ref() {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            let elem_ty = g.ty(&elem);
            g.note("Option");
            let (at, length) = (g.value_name("get_at"), g.value_name("length"));
            let this = open(&g);
            let mut at_params = vec![format!("self: {this}"), "index: Float".to_string()];
            let mut at_js_params = vec!["self".to_string(), "i".to_string()];
            for p in &c.type_params {
                if g.crosses(&elem, &p.name) {
                    g.note("Codec");
                    at_params.push(format!("{}_codec: Codec({})", lower(&p.name), Gen::tvar(&p.name)));
                    at_js_params.push(codec_var(&p.name));
                }
            }
            g.out += &format!("@external(javascript, \"{}\", \"{at}\")\npub fn {at}({}) -> Option({elem_ty})\n\n", g.own_ffi(), at_params.join(", "));
            let this = open(&g);
            g.out += &format!("@external(javascript, \"{}\", \"{length}\")\npub fn {length}(self: {this}) -> Float\n\n", g.own_ffi());
            g.ffi += &format!(
                "export function {at}({}) {{\n  const v = self[i];\n  return v === undefined ? new None() : new Some({});\n}}\nexport function {length}(self) {{\n  return self.length;\n}}\n",
                at_js_params.join(", "),
                g.to_gleam(&elem, "v")
            );
        }
        if c.iterates_async.is_some() {
            g.skip("async iteration");
        }
        // Upcasts. Along the chain a subtype is its base already; this closes
        // the type at this class, for a list or a branch of several subtypes:
        // `element.upcast(div)` is an `Element(Nil)`.
        let id = g.value_name("upcast");
        let (this, closed) = (open(&g), g.class_type(&c.name, &own));
        g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {closed}\n\n", g.runtime_ffi());
        // A base off the chain, whose members it has taken: `html_element.as_global_event_handlers(e)`.
        for a in secondary(&m, c) {
            let Some(args) = base_args(&m, c, &a) else { continue };
            let id = g.value_name(&format!("as_{}", snake(&g.tname(&a))));
            let (this, base) = (open(&g), g.class_type(&a, &args));
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {this}) -> {base}\n\n", g.runtime_ffi());
        }
        // Downcasts, checked: only a class has a constructor to test against.
        if let Some(js) = &c.js {
            let test = format!("v instanceof {}", global(js));
            let target = g.class_type(&c.name, &[]);
            let root = chain_root(&g, &c.name);
            if root != c.name {
                let id = g.value_name("from");
                *g.open.borrow_mut() = Some(0);
                let from = g.class_type(&root, &[]);
                *g.open.borrow_mut() = None;
                g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: {from}) -> Result({target}, Nil)\n\n", g.own_ffi());
                g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
            }
            let id = g.value_name("cast");
            g.note("Dynamic");
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(value: Dynamic) -> Result({target}, Nil)\n\n", g.own_ffi());
            g.ffi += &format!("export function {id}(v) {{\n  return {test} ? new Ok(v) : new GleamError(undefined);\n}}\n");
        }
    }
    for f in &m.funcs {
        match m.func_namespaces.get(&f.import) {
            // A namespace's by its own name, in its module (a nested one's
            // in a module of it): `geometry.distance`, `deep.twice`.
            Some((path, own, module, depth)) => {
                let unit = std::iter::once(module_path(&paths, module)).chain(path[(*depth).min(path.len())..].iter().map(|s| snake(&upper(s)))).collect::<Vec<_>>().join("/");
                g.switch(&unit);
                let id = g.fresh_value(&function_name(f, own));
                g.function(&id, f, None);
            }
            None => {
                g.switch(&paths[&f.module]);
                let id = g.value_name(&function_name(f, &f.name));
                g.function(&id, f, None);
            }
        }
    }
    g.switch("");

    // A namespace's types by their own names, in its module: `ns.Node` is
    // `pub type Node(sub) = NsNode2(sub)` in `bindings/namespaces/ns`, as the
    // types themselves are in `bindings/types`.
    let headers: HashMap<String, String> = g
        .units
        .get(TYPES)
        .map(|u| {
            u.out
                .lines()
                .filter_map(|l| l.strip_prefix("pub opaque type ").or_else(|| l.strip_prefix("pub type ")))
                .map(|rest| {
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    let params = rest[name.len()..].strip_prefix('(').and_then(|r| r.split_once(')')).map(|(p, _)| format!("({p})")).unwrap_or_default();
                    (name, params)
                })
                .collect()
        })
        .unwrap_or_default();
    let mut aliases: BTreeMap<String, Vec<(String, String, String)>> = BTreeMap::new();
    for (name, path) in &m.namespaced {
        let Some((module, depth)) = m.namespace_modules.get(name) else { continue };
        let gleam = g.tname(name);
        let Some(params) = headers.get(&gleam) else { continue };
        let base = module_path(&paths, module);
        // Past what its module stands for: a namespace's own module.
        let unit = std::iter::once(base).chain(path[(*depth).min(path.len() - 1)..path.len() - 1].iter().map(|s| snake(&upper(s)))).collect::<Vec<_>>().join("/");
        aliases.entry(unit).or_default().push((upper(path.last().unwrap()), gleam, params.clone()));
    }
    for (unit, list) in aliases {
        let u = g.units.entry(unit).or_default();
        let mut seen = HashSet::new();
        // What the module imports, these aliases' types included.
        let targets: HashSet<String> = list.iter().map(|(_, t, _)| t.clone()).collect();
        for (leaf, target, params) in list {
            // Once, and not where the module names another type so, nor
            // Gleam's own (echarts' `List` would hide `List`).
            if leaf == target || OWN.contains(&leaf.as_str()) || u.used.contains(&leaf) || targets.contains(&leaf) || !seen.insert(leaf.clone()) {
                continue;
            }
            u.out += &format!("pub type {leaf}{params} =\n  {target}{params}\n\n");
            u.used.insert(target);
        }
    }

    // Each module: what it imports of the others, of Gleam's, and its JavaScript's.
    let mut files = Vec::new();
    // Not a module with nothing in it (one whose aliases all gave way).
    for (path, unit) in std::mem::take(&mut g.units).into_iter().filter(|(p, u)| !p.is_empty() && !(u.out.trim().is_empty() && u.ffi.is_empty())) {
        let mut by_module: BTreeMap<String, BTreeSet<&str>> = BTreeMap::new();
        for t in &unit.used {
            let module = match t.as_str() {
                "Dynamic" => "gleam/dynamic".to_string(),
                "Option" => "gleam/option".to_string(),
                t => match g.home.get(t) {
                    Some(home) => home.clone(),
                    None => continue,
                },
            };
            if module != path {
                by_module.entry(module).or_default().insert(t);
            }
        }
        let imports: String = by_module.iter().map(|(module, ts)| format!("import {module}.{{{}}}\n", ts.iter().map(|t| format!("type {t}")).collect::<Vec<_>>().join(", "))).collect();
        files.push((format!("{path}.gleam"), format!("// Generated. Bindings for Gleam's JavaScript target.\n{imports}\n{}", unit.out)));
        if unit.ffi.is_empty() {
            continue;
        }
        let up = "../".repeat(path.matches('/').count());
        let prelude = if up.is_empty() { "./".to_string() } else { up.clone() };
        let mut header = format!(
            "// Generated. Calls the API, and converts between JavaScript's values and Gleam's.\n\
             import {{ Some, None }} from \"{up}../gleam_stdlib/gleam/option.mjs\";\n\
             import {{ Ok, Error as GleamError, toList }} from \"{prelude}gleam.mjs\";\n"
        );
        if path != RUNTIME {
            header += &format!("import {{ $wrap, $same, PK, G }} from \"{}\";\nimport * as $T from \"{}\";\n", relative(&path, "bindings/runtime_ffi.mjs"), relative(&path, "bindings/types_ffi.mjs"));
        }
        if path == TYPES {
            header += "import * as B from \"./types.mjs\";\n";
        }
        files.push((format!("{path}_ffi.mjs"), format!("{header}{}", unit.ffi)));
    }
    Ok(Output { files, skipped: g.skipped, manifest: super::wasm::manifest(&m) })
}

// What JavaScript reserves, as a function's name.
const JS_RESERVED: &[&str] = &[
    "await", "break", "case", "catch", "class", "const", "continue", "debugger", "default", "delete", "do", "else", "enum", "export", "extends", "false",
    "finally", "for", "function", "if", "implements", "import", "in", "instanceof", "interface", "let", "new", "null", "package", "private", "protected",
    "public", "return", "static", "super", "switch", "this", "throw", "true", "try", "typeof", "var", "void", "while", "with", "yield",
    // Not names in a module's strict code.
    "arguments", "eval",
];

// The modules every library's bindings have.
const RUNTIME: &str = "bindings/runtime";
const TYPES: &str = "bindings/types";
const OWN_RUNTIME: &[&str] = &["Promise", "JsError", "Codec"];

// A class type's phantom parameter, where it is declared.
const PHANTOM: &str = "sub";

// The class a class's chain continues from, and the arguments it gives it:
// its first base that is a class of the model.
fn parent(m: &Model, c: &Class) -> Option<(String, Vec<Ty>)> {
    c.bases.iter().find_map(|b| match b {
        Ty::Obj(n) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), Vec::new())),
        Ty::Generic(n, args) if m.classes.iter().any(|k| k.name == *n) => Some((n.clone(), args.clone())),
        _ => None,
    })
}

// The top of a class's chain.
fn chain_root(g: &Gen, class: &str) -> String {
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
fn secondary(m: &Model, c: &Class) -> Vec<String> {
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
fn take_secondary(m: &mut Model) {
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

// `to` (a path from the package's root), relative to module `from`.
fn relative(from: &str, to: &str) -> String {
    let from: Vec<&str> = from.split('/').collect();
    let from = &from[..from.len() - 1];
    let to: Vec<&str> = to.split('/').collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let up = "../".repeat(from.len() - common);
    let rest = to[common..].join("/");
    if up.is_empty() { format!("./{rest}") } else { format!("{up}{rest}") }
}

// A function's name, as the model has it; a `$` method (`$add`, which
// returns a new value where `add` changes this one) is `add_new`.
fn function_name(f: &Func, name: &str) -> String {
    let dollar = match &f.call {
        Call::Method(js) | Call::Static(js) | Call::Get(js) | Call::Set(js) | Call::StaticGet(js) | Call::StaticSet(js) => js.rsplit('.').next().is_some_and(|n| n.starts_with('$')),
        _ => false,
    };
    if !dollar {
        return name.to_string();
    }
    let stem = name.trim_start_matches('_');
    match stem.strip_prefix("set_") {
        Some(rest) => format!("set_{}_new", rest.trim_start_matches('_')),
        None => format!("{}_new", if stem.is_empty() { "dollar" } else { stem }),
    }
}

// A callback holding itself, through its own types or another's: a Gleam
// alias may not, so within such a cycle a callback is `Dynamic`.
fn break_cycles(m: &mut Model) {
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
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => refs(inner, names, out),
            Ty::Tuple(es) | Ty::Union(es) => es.iter().for_each(|e| refs(e, names, out)),
            _ => {}
        }
    }
    fn cut(t: &mut Ty, cyclic: &HashSet<String>) {
        match t {
            Ty::Obj(n) | Ty::Generic(n, _) if cyclic.contains(n) => *t = Ty::Any,
            Ty::Generic(_, args) => args.iter_mut().for_each(|a| cut(a, cyclic)),
            Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => cut(inner, cyclic),
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

// The JavaScript name of the codec of type parameter `n`.
fn codec_var(n: &str) -> String {
    format!("$c_{}", n.replace(|c: char| !c.is_alphanumeric() && c != '_', "_"))
}

fn generic_args(t: &Ty) -> &[Ty] {
    match t {
        Ty::Generic(_, args) => args,
        _ => &[],
    }
}

// Whether `t` names type parameter `p` anywhere.
fn mentions(t: &Ty, p: &str) -> bool {
    match t {
        Ty::Param(n) => n == p,
        Ty::Nullable(inner, _) | Ty::ArrayOf(inner) | Ty::Listener(inner) => mentions(inner, p),
        Ty::Tuple(es) | Ty::Union(es) | Ty::Generic(_, es) => es.iter().any(|e| mentions(e, p)),
        _ => false,
    }
}

// A module's Gleam path: as given out, else as one would be (`areas/pb`
// where only a namespace of the package's has functions).
fn module_path(paths: &HashMap<String, String>, module: &str) -> String {
    paths.get(module).cloned().unwrap_or_else(|| format!("bindings/{}", module.split('/').map(lower).collect::<Vec<_>>().join("/")))
}
