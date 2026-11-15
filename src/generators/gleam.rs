// XML -> Gleam bindings for Gleam's JavaScript target: `bindings.gleam`, of
// opaque types and external functions, and `bindings_ffi.mjs`, which calls
// the API and converts between JavaScript's values and Gleam's.
//
// Gleam has functions and custom types; no methods, subtypes, overloads or
// optional arguments. So, from the shared model (`generators::wasm`): a class
// is an opaque type and its members functions named for it
// (`shape_get_size(s)`), an ancestor is reached by `shape_as_named(s)`, an
// optional argument or a nullable value is an `Option(a)`, an array a
// `List(a)`, a tuple a tuple, a union or string enum a custom type, a
// callback a Gleam function, a promise a `Promise(a)` to `promise_then`.
// Gleam's values are JavaScript's, but for these: the FFI converts them.

use std::collections::{BTreeMap, HashMap, HashSet};

use roxmltree::Document;

use super::js::{access, global, js_test};
use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, model, with_self_args};

const KEYWORDS: &[&str] = &[
    "as", "assert", "auto", "case", "const", "delegate", "derive", "echo", "else", "fn", "if", "implement", "import", "let", "macro", "opaque", "panic", "pub", "test", "todo", "type", "use",
];

// Types the bindings declare whatever the API: what a module of Gleam can
// name of JavaScript's values. Not opaque types of the API.
const OWN: &[&str] = &["Promise", "JsError", "Dynamic", "Option", "List", "Result", "Nil", "Int", "Float", "String", "Bool", "BitArray"];

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
    out: String,
    ffi: String,
    skipped: BTreeMap<String, usize>,
}

impl<'m> Gen<'m> {
    fn skip(&mut self, why: &str) {
        *self.skipped.entry(why.to_string()).or_default() += 1;
    }

    // A module-level value name not yet taken: `shape_get_size`, `area_2`.
    fn value_name(&mut self, base: &str) -> String {
        let base = lower(base);
        let mut name = base.clone();
        let mut i = 2;
        while !self.values.insert(name.clone()) {
            name = format!("{base}_{i}");
            i += 1;
        }
        name
    }

    fn tname(&self, n: &str) -> String {
        self.types.get(n).cloned().unwrap_or_else(|| upper(n))
    }

    // A generic type named without its arguments: `Polygon(Dynamic)`.
    fn bare(&self, n: &str) -> String {
        let arity = self.m.classes.iter().find(|c| c.name == n).map(|c| c.type_params.len())
            .or_else(|| self.m.opaque_arity.get(n).copied())
            .or_else(|| self.m.unions.iter().find(|u| u.name == n).map(|u| u.type_params.len()))
            .or_else(|| self.callbacks.get(n).map(Vec::len))
            .unwrap_or(0);
        let name = self.tname(n);
        if arity == 0 { name } else { format!("{name}({})", vec!["Dynamic"; arity].join(", ")) }
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
        match t {
            Ty::Void => "Nil".into(),
            Ty::Num => "Float".into(),
            Ty::Bool => "Bool".into(),
            Ty::Str => "String".into(),
            Ty::Any | Ty::Union(_) | Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => "Dynamic".into(),
            Ty::Array => "List(Dynamic)".into(),
            Ty::ArrayOf(e) => format!("List({})", self.ty(e)),
            Ty::Tuple(es) => format!("#({})", es.iter().map(|e| self.ty(e)).collect::<Vec<_>>().join(", ")),
            // A promise of what the model does not say.
            Ty::Obj(n) if n == "Promise" || n == "PromiseLike" => "Promise(Dynamic)".into(),
            Ty::Obj(n) => self.bare(n),
            Ty::Enum(n) | Ty::NumEnum(n) => self.tname(n),
            Ty::Nullable(inner, _) => format!("Option({})", self.ty(inner)),
            Ty::NamedUnion(n) if self.decodable(n) => self.bare(n),
            Ty::NamedUnion(_) => "Dynamic".into(),
            Ty::Param(n) => Self::tvar(n),
            Ty::Generic(n, _) if self.m.unions.iter().any(|u| u.name == *n) && !self.decodable(n) => "Dynamic".into(),
            // A callback takes the type arguments it uses.
            Ty::Generic(n, args) if self.callbacks.contains_key(n) => {
                let args: Vec<String> = self.callbacks[n].iter().filter_map(|&i| args.get(i)).map(|a| self.ty(a)).collect();
                if args.is_empty() { self.tname(n) } else { format!("{}({})", self.tname(n), args.join(", ")) }
            }
            Ty::Generic(n, args) if args.is_empty() => self.bare(n),
            Ty::Generic(n, args) => format!("{}({})", self.tname(n), args.iter().map(|a| self.ty(a)).collect::<Vec<_>>().join(", ")),
        }
    }

    // A JavaScript expression that is `x`, JavaScript's value, as Gleam has it.
    fn to_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            Ty::Void => format!("({x}, undefined)"),
            Ty::Nullable(inner, _) => format!("((v) => v == null ? new None() : new Some({}))({x})", self.to_gleam(inner, "v")),
            Ty::Array => format!("toList(Array.from({x} ?? []))"),
            Ty::ArrayOf(e) => format!("toList(Array.from({x} ?? [], (v) => {}))", self.to_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.to_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("{}_to({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("{}_to({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("{}_to({x})", self.tname(n)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("{}_to({x})", self.tname(n)),
            Ty::Generic(n, args) if (n == "Promise" || n == "PromiseLike") && args.len() == 1 => {
                format!("Promise.resolve({x}).then((v) => {})", self.to_gleam(&args[0], "v"))
            }
            _ => x.to_string(),
        }
    }

    // A JavaScript expression that is `x`, Gleam's value, as JavaScript has it.
    fn from_gleam(&self, t: &Ty, x: &str) -> String {
        match t {
            Ty::Nullable(inner, undefined) => format!("((v) => v instanceof Some ? {} : {})({x})", self.from_gleam(inner, "v[0]"), if *undefined { "undefined" } else { "null" }),
            Ty::Array => format!("{x}.toArray()"),
            Ty::ArrayOf(e) => format!("{x}.toArray().map((v) => {})", self.from_gleam(e, "v")),
            Ty::Tuple(es) => format!("((v) => [{}])({x})", es.iter().enumerate().map(|(i, e)| self.from_gleam(e, &format!("v[{i}]"))).collect::<Vec<_>>().join(", ")),
            Ty::Enum(n) | Ty::NumEnum(n) => format!("{}_from({x})", self.tname(n)),
            Ty::NamedUnion(n) if self.decodable(n) => format!("{}_from({x})", self.tname(n)),
            Ty::Generic(n, args) if self.decodable(n) && !args.is_empty() => format!("{}_from({x})", self.tname(n)),
            Ty::Obj(n) | Ty::Generic(n, _) if self.is_callback(n) => format!("{}_from({x})", self.tname(n)),
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
            params.push(format!("self: {}{}", self.tname(class), Self::type_params(tps)));
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
        self.out += &format!("@external(javascript, \"./bindings_ffi.mjs\", \"{name}\")\npub fn {name}({}) -> {}\n\n", params.join(", "), self.ty(&f.ret));
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
        self.ffi += &format!("function {name}_to(v) {{\n{to}}}\nfunction {name}_from(u) {{\n{from}}}\n");
    }
}

// The bindings of the XML, or what Gleam cannot express of it.
pub fn module(xml: &str) -> Result<Output, Vec<String>> {
    let doc = Document::parse(xml).map_err(|e| vec![format!("not well-formed XML: {e}")])?;
    let root_import = doc.root_element().attribute("import").map(str::to_string);
    let mut m = model(xml, "Gleam")?;
    break_cycles(&mut m);
    let mut g = Gen { m: &m, types: HashMap::new(), variants: HashMap::new(), callbacks: HashMap::new(), values: HashSet::new(), out: String::new(), ffi: String::new(), skipped: m.skipped.clone() };

    // Names: every type, and every variant of a union or enum, apart.
    let mut type_names: HashSet<String> = OWN.iter().map(|s| s.to_string()).collect();
    let mut name_type = |n: &str, g: &mut Gen| {
        let base = upper(n);
        let mut name = base.clone();
        let mut i = 2;
        while !type_names.insert(name.clone()) {
            name = format!("{base}{i}");
            i += 1;
        }
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

    g.out += "// Generated. Bindings for Gleam's JavaScript target.\nimport gleam/dynamic.{type Dynamic}\nimport gleam/option.{type Option}\n\n\
              /// A JavaScript promise: `promise_then` gives its result to a function.\npub type Promise(a)\n\n\
              /// A JavaScript exception, or the reason a promise was rejected.\npub type JsError\n\n\
              @external(javascript, \"./bindings_ffi.mjs\", \"promise_then\")\npub fn promise_then(promise: Promise(a), then: fn(Result(a, JsError)) -> Nil) -> Nil\n\n\
              /// Runs `f`, and gives what it returned, or the exception it threw.\n\
              @external(javascript, \"./bindings_ffi.mjs\", \"attempt\")\npub fn attempt(f: fn() -> a) -> Result(a, JsError)\n\n\
              @external(javascript, \"./bindings_ffi.mjs\", \"js_error_message\")\npub fn js_error_message(error: JsError) -> String\n\n\
              /// Any value, untyped: what a value of no particular type is made from.\n\
              @external(javascript, \"./bindings_ffi.mjs\", \"dynamic\")\npub fn dynamic(value: a) -> Dynamic\n\n";
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
        "// Generated. Calls the API, and converts between JavaScript's values and Gleam's.\n\
         import * as B from \"./bindings.mjs\";\n\
         import {{ Some, None }} from \"../gleam_stdlib/gleam/option.mjs\";\n\
         import {{ Ok, Error as GleamError, toList }} from \"./gleam.mjs\";\n{imports}\n\
         // Where the API's names are: its module, or globals; with several packages, each one's.\n\
         const PK = {{ {} }};\nconst G = {};\n\n\
         export function promise_then(p, f) {{\n  Promise.resolve(p).then((v) => f(new Ok(v)), (e) => f(new GleamError(e)));\n}}\n\
         export function attempt(f) {{\n  try {{\n    return new Ok(f());\n  }} catch (e) {{\n    return new GleamError(e);\n  }}\n}}\n\
         export function js_error_message(e) {{\n  return String(e instanceof Error ? e.message : e?.message ?? e);\n}}\n\
         export function dynamic(v) {{\n  return v;\n}}\n\
         // A Gleam function made a JavaScript function, the same one each time: one\n\
         // listener added and removed is one function.\n\
         const $wrapped = new WeakMap();\n\
         function $wrap(f, make) {{\n  if (typeof f !== \"function\") return f;\n  let w = $wrapped.get(f);\n  if (!w) $wrapped.set(f, (w = make(f)));\n  return w;\n}}\n\n",
        modules.join(", "),
        root_import.as_ref().map_or("globalThis".to_string(), |r| format!("PK[{r:?}]"))
    );

    // Types declared elsewhere: opaque.
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
             function {name}_to(v) {{\n  const i = {name}_values.indexOf(v);\n  if (i < 0) throw new TypeError(\"not a {name}: \" + v);\n  return new ({name}_cases()[i])();\n}}\n\
             function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
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
             function {name}_to(v) {{\n  return new ({name}_cases()[{name}_values.indexOf(v)])();\n}}\n\
             function {name}_from(v) {{\n  return {name}_values[{name}_cases().findIndex((c) => v instanceof c)];\n}}\n",
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
        g.out += &format!("pub type {name}{params} =\n  fn({}) -> {}\n\n", args.join(", "), g.ty(&c.ret));
        let names: Vec<String> = (0..c.params.len()).map(|i| format!("a{i}")).collect();
        let into_gleam: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.to_gleam(t, a)).collect();
        let into_js: Vec<String> = c.params.iter().zip(&names).map(|((_, t), a)| g.from_gleam(t, a)).collect();
        g.ffi += &format!(
            "function {name}_from(f) {{\n  return $wrap(f, (f) => ({}) => {});\n}}\n\
             function {name}_to(f) {{\n  return typeof f === \"function\" ? ({}) => {} : f;\n}}\n",
            names.join(", "),
            g.from_gleam(&c.ret, &format!("f({})", into_gleam.join(", "))),
            names.join(", "),
            g.to_gleam(&c.ret, &format!("f({})", into_js.join(", "))),
        );
    }
    for u in m.unions.iter().filter(|u| u.decodable()) {
        g.union(u);
    }
    // Classes: an opaque type, and functions named for it.
    for c in &m.classes {
        let name = g.tname(&c.name);
        let params = Gen::type_params(&c.type_params);
        g.out += &format!("pub type {name}{params}\n\n");
        let prefix = snake(&c.name);
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
            let id = g.value_name(&format!("{prefix}_{}", function_name(&f)));
            g.function(&id, &f, Some((&c.name, &c.type_params)));
        }
        // What it iterates, or holds as a subclass of Array: a list.
        if let Some(elem) = c.iterates.as_ref().or(c.elements.as_ref()) {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            let id = g.value_name(&format!("{prefix}_to_list"));
            g.out += &format!("@external(javascript, \"./bindings_ffi.mjs\", \"{id}\")\npub fn {id}(self: {name}{params}) -> List({})\n\n", g.ty(&elem));
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
            let id = g.value_name(&format!("{prefix}_as_{}", snake(a)));
            g.out += &format!("@external(javascript, \"./bindings_ffi.mjs\", \"dynamic\")\npub fn {id}(value: {name}{params}) -> {}{args}\n\n", g.tname(a));
        }
    }
    for f in &m.funcs {
        let id = g.value_name(&function_name(f));
        g.function(&id, f, None);
    }
    // No unused import: a warning, where warnings are errors.
    let out = if g.out.contains("Option(") { g.out } else { g.out.replacen("import gleam/option.{type Option}\n", "", 1) };
    Ok(Output { files: vec![("bindings.gleam".into(), out), ("bindings_ffi.mjs".into(), g.ffi)], skipped: g.skipped })
}


// A function's name, as the model has it; a `$` method (`$add`, which
// returns a new value where `add` changes this one) is `add_new`.
fn function_name(f: &Func) -> String {
    let dollar = match &f.call {
        Call::Method(js) | Call::Static(js) | Call::Get(js) | Call::Set(js) | Call::StaticGet(js) | Call::StaticSet(js) => js.rsplit('.').next().is_some_and(|n| n.starts_with('$')),
        _ => false,
    };
    if !dollar {
        return f.name.clone();
    }
    let stem = f.name.trim_start_matches('_');
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
