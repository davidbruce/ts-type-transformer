// XML -> Gleam bindings for Gleam's JavaScript target: modules of opaque
// types and external functions, each with an FFI module that calls the API
// and converts between JavaScript's values and Gleam's.
//
// Gleam modules may not import each other in a cycle. A class is an opaque
// type, which names nothing: each area's are in `bindings/types/<area>`,
// importing nothing. Enums, unions and callbacks, which may name each other,
// are in `bindings/types`; `Promise`, `JsError` and `Codec` in
// `bindings/runtime`. An area's functions, in `bindings/<area>`, import
// only types, so no module imports one that imports it.
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
use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, model, with_self_args};
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
    type_names: |candidate, _, kind, _| if kind == Kind::Opaque && (OWN.contains(&candidate) || candidate == "PromiseLike") { Vec::new() } else { vec![upper(candidate)] },
    values_of: |own, name, kind, m| {
        let prefix = snake(own);
        // Every type has a codec: `shape_codec`.
        let codec = lower(&format!("{prefix}_codec"));
        let Some(c) = m.classes.iter().find(|c| c.name == name).filter(|_| kind == Kind::Class) else { return vec![codec] };
        let mut out: Vec<String> = c.consts.iter().map(|(k, _)| lower(&format!("{prefix}_{k}"))).collect();
        out.push(codec);
        if c.iterates.is_some() || c.elements.is_some() {
            out.push(lower(&format!("{prefix}_to_list")));
        }
        out.extend(c.ancestors.iter().map(|a| lower(&format!("{prefix}_as_{}", snake(&upper(a))))));
        out
    },
    types_apart: true,
    overloads: false,
    func_name: |owner, name, f| {
        Some(match owner {
            Some(o) => lower(&format!("{}_{}", snake(o), function_name(f, name))),
            None => lower(&function_name(f, name)),
        })
    },
    funcs_flat: true,
    members: |_| Vec::new(),
};

pub struct Output {
    pub files: Vec<(String, String)>,
    pub skipped: BTreeMap<String, usize>,
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
    // Names given out: module-level values, and types and variants.
    values: HashSet<String>,
    // The module being written: its Gleam, its JavaScript, and the Gleam
    // types its Gleam names (for its imports). Those written, by path.
    unit: String,
    out: String,
    ffi: String,
    used: RefCell<BTreeSet<String>>,
    units: BTreeMap<String, Unit>,
    // The module each Gleam type is declared in.
    home: HashMap<String, String>,
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

    // A module-level value's name: `shape_get_size`. `names::settle` made
    // them apart; one given out twice is a mistake in its naming.
    fn value_name(&mut self, base: &str) -> String {
        let name = lower(base);
        assert!(self.values.insert(name.clone()), "Gleam: the value `{name}` is named twice");
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
            Ty::Param(n) => Self::tvar(n),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => dynamic(),
            // A callback takes the type arguments it uses.
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => {
                let args: Vec<String> = self.callbacks[n].iter().filter_map(|&i| args.get(i)).map(|a| self.ty(a)).collect();
                if args.is_empty() { self.gt(n) } else { format!("{}({})", self.gt(n), args.join(", ")) }
            }
            Ty::Generic(n, args) if args.is_empty() => self.bare(n),
            Ty::Generic(n, args) => format!("{}({})", self.gt(n), args.iter().map(|a| self.ty(a)).collect::<Vec<_>>().join(", ")),
        }
    }

    // A JavaScript expression that is `x`, JavaScript's value, as Gleam has it.
    fn to_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            // A type parameter's values cross by the codec the function was given.
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
            Ty::Param(n) => codec_var(n),
            t if self.to_gleam(t, "v") == "v" && self.from_gleam(t, "v") == "v" => "$same".into(),
            t => format!("{{ to: (v) => {}, from: (v) => {} }}", self.to_gleam(t, "v"), self.from_gleam(t, "v")),
        }
    }

    // Whether a type parameter's values cross where `t` is: bare, in an
    // option, list, tuple or promise, through a callback or a union. Not
    // inside a handle (`Box(t)`), which crosses as it is.
    fn crosses(&self, t: &Ty, p: &str) -> bool {
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
        let mut params = Vec::new();
        let mut js_params = Vec::new();
        if let Some((class, tps)) = owner.filter(|_| Class::has_self(f)) {
            params.push(format!("self: {}{}", self.gt(class), Self::type_params(tps)));
            js_params.push("self".to_string());
        }
        let mut taken: HashSet<String> = HashSet::from(["self".to_string()]);
        let mut args = Vec::new();
        for (i, (p, t)) in f.params.iter().enumerate() {
            let mut id = lower(p);
            while !taken.insert(id.clone()) {
                id.push('_');
            }
            let last_rest = f.rest && i == f.params.len() - 1;
            params.push(format!("{id}: {}", self.ty(t)));
            js_params.push(id.clone());
            let value = self.from_gleam(t, &id);
            args.push(if last_rest { format!("...({value} ?? [])") } else { value });
        }
        // A codec for each type parameter whose values cross: the class's, then the function's own.
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
            Call::Set(p) => format!("self{} = {list}", access(p)),
            Call::New(path) => format!("new ({})({list})", global(path)),
            Call::Static(path) => format!("{}({list})", global(path)),
            Call::StaticGet(path) => global(path),
            // A top-level variable is assigned where it is: a module's cannot be.
            Call::StaticSet(path) if !path.contains('.') && !path.contains("::") => format!("globalThis{} = {list}", access(path)),
            Call::StaticSet(path) => format!("{} = {list}", global(path)),
            Call::Create => "({})".into(),
            Call::Invoke => format!("self({list})"),
            Call::Construct => format!("new self({list})"),
            Call::IndexGet => format!("self[{first}]"),
            Call::IndexSet => format!("self[{first}] = {second}"),
        };
        let body = match &f.ret {
            Ty::Void => format!("{call};\n  return undefined;"),
            t => format!("return {};", self.to_gleam(t, &call)),
        };
        let ret = self.ty(&f.ret);
        self.out += &format!("@external(javascript, \"{}\", \"{name}\")\npub fn {name}({}) -> {ret}\n\n", self.own_ffi(), params.join(", "));
        self.ffi += &format!("export function {name}({}) {{\n  {body}\n}}\n", js_params.join(", "));
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
    super::names::settle(&mut m, &NAMING);
    break_cycles(&mut m);
    let mut g = Gen {
        m: &m,
        types: HashMap::new(),
        variants: HashMap::new(),
        callbacks: HashMap::new(),
        values: HashSet::new(),
        unit: RUNTIME.into(),
        out: String::new(),
        ffi: String::new(),
        used: RefCell::default(),
        units: BTreeMap::new(),
        home: HashMap::new(),
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
        g.values.insert(reserved.into());
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
        while !taken.insert(path.clone()) || !taken.insert(class_unit(&path)) {
            path = format!("bindings/{}_{n}", segments.join("/"));
            n += 1;
        }
        paths.insert(area.clone(), path);
    }
    for o in OWN_RUNTIME {
        g.home.insert(o.to_string(), RUNTIME.into());
    }
    for c in &m.classes {
        g.home.insert(g.tname(&c.name), class_unit(&paths[&c.module]));
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
              @external(javascript, \"./runtime_ffi.mjs\", \"codec_list\")\npub fn codec_list(of: Codec(a)) -> Codec(List(a))\n\n";
    g.note("Dynamic");
    g.note("Option");
    let mut packages: Vec<&String> = m.packages.values().collect();
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
        let into_js: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.from_gleam(t, a)).collect();
        let codecs: String = used.iter().map(|p| format!(", {}", codec_var(&p.name))).collect();
        g.ffi += &format!(
            "export function {name}_from(f{codecs}) {{\n  return $wrap(f, (f) => ({}) => {});\n}}\n\
             export function {name}_to(f{codecs}) {{\n  return typeof f === \"function\" ? ({}) => {} : f;\n}}\n",
            names.join(", "),
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
    // Classes: an opaque type each, in its area's module of types.
    for c in &m.classes {
        g.switch(&class_unit(&paths[&c.module]));
        g.out += &format!("pub type {}{}\n\n", g.tname(&c.name), Gen::type_params(&c.type_params));
        codecs.push((c.name.clone(), c.type_params.clone(), false, g.unit.clone()));
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
    // Functions named for a class, in its area's module of functions.
    for c in &m.classes {
        g.switch(&paths[&c.module]);
        // Noted where written: a class may have no functions.
        let name = g.tname(&c.name);
        let params = Gen::type_params(&c.type_params);
        let prefix = snake(&name);
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
            let id = g.value_name(&format!("{prefix}_{constant}"));
            g.out += &format!("pub const {id} = {gleam}\n\n");
        }
        for f in &c.funcs {
            let f = with_self_args(f, c);
            let id = g.value_name(&format!("{prefix}_{}", function_name(&f, &f.name)));
            g.function(&id, &f, Some((&c.name, &c.type_params)));
        }
        // What it iterates, or holds as a subclass of Array: a list.
        if let Some(elem) = c.iterates.as_ref().or(c.elements.as_ref()) {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            let id = g.value_name(&format!("{prefix}_to_list"));
            let elem_ty = g.ty(&elem);
            g.note(&name);
            g.out += &format!("@external(javascript, \"{}\", \"{id}\")\npub fn {id}(self: {name}{params}) -> List({elem_ty})\n\n", g.own_ffi());
            g.ffi += &format!("export function {id}(self) {{\n  return toList(Array.from(self, (v) => {}));\n}}\n", g.to_gleam(&elem, "v"));
        }
        if c.iterates_async.is_some() {
            g.skip("async iteration");
        }
        // An ancestor's functions take the type: `shape_as_named(s)`.
        for a in &c.ancestors {
            let Some(k) = m.classes.iter().find(|k| k.name == *a) else { continue };
            let args = if k.type_params.is_empty() {
                String::new()
            } else {
                match base_args(&m, c, a) {
                    Some(args) => format!("({})", args.iter().map(|t| g.ty(t)).collect::<Vec<_>>().join(", ")),
                    None => continue,
                }
            };
            let id = g.value_name(&format!("{prefix}_as_{}", snake(&g.tname(a))));
            let ancestor = g.gt(a);
            g.note(&name);
            g.out += &format!("@external(javascript, \"{}\", \"dynamic\")\npub fn {id}(value: {name}{params}) -> {ancestor}{args}\n\n", g.runtime_ffi());
        }
    }
    for f in &m.funcs {
        g.switch(&paths[&f.module]);
        let id = g.value_name(&function_name(f, &f.name));
        g.function(&id, f, None);
    }
    g.switch("");

    // Each module: what it imports of the others, of Gleam's, and its JavaScript's.
    let mut files = Vec::new();
    for (path, unit) in std::mem::take(&mut g.units).into_iter().filter(|(p, _)| !p.is_empty()) {
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
    Ok(Output { files, skipped: g.skipped })
}

// The modules every library's bindings have.
const RUNTIME: &str = "bindings/runtime";
const TYPES: &str = "bindings/types";
const OWN_RUNTIME: &[&str] = &["Promise", "JsError", "Codec"];

// An area's module of class types: `bindings/areas/main` -> `bindings/types/areas_main`.
fn class_unit(area: &str) -> String {
    format!("{TYPES}/{}", area.trim_start_matches("bindings/").replace('/', "_"))
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
