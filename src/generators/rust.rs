// XML -> a Rust crate of bindings for wasm32-unknown-unknown.
//
// Every JavaScript object is a value holding a counted handle: `Clone`
// retains it and `Drop` releases it. Inheritance is traits: each type's own
// members are default methods of `XMethods`, implemented for the type and
// each of its subtypes, so `div.set_attribute(..)` works on an
// HTMLDivElement. Where two of a type's traits share a name, the type has an
// inherent method of that name, which Rust prefers.
//
// Names are Rust's: methods and functions in snake_case, a property `x` as
// `x()` and `set_x(..)`, generated type names and enum variants in
// CamelCase, a map's keys as constants of the map
// (`HTMLElementEventMap::CLICK`). Arguments are borrowed: an object is
// `&impl AsRef<T>`, so any subtype goes in; an array or a union is
// `impl Into<..>`, so `["a", "b"]` or `42.0` do; a value of a type parameter
// goes by value. Nullables are `Option`,
// unions enums, callbacks closures, promises futures.
//
// The crate: `Cargo.toml`, `src/lib.rs`, `src/runtime.rs`, `src/types.rs`
// (enums, unions, callbacks), a file per module, and `src/raw.rs` (the
// imports).

use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, subst, with_self_args};
use super::names::{Kind, Naming, identifiers};

const KEYWORDS: &[&str] = &[
    "as", "break", "const", "continue", "else", "enum", "extern", "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub",
    "ref", "return", "static", "struct", "trait", "true", "type", "unsafe", "use", "where", "while", "async", "await", "dyn", "abstract", "become", "box",
    "do", "final", "macro", "override", "priv", "typeof", "unsized", "virtual", "yield", "try", "gen", "self", "Self", "super", "crate", "_",
];

// Method names a handle type already has, from the traits every one implements.
const TAKEN: &[&str] = &["clone", "as_ref", "eq", "ne", "into", "lend", "release", "iter", "iterate_async", "as_array"];

// A value's name: a keyword gets a trailing underscore.
fn ident(name: &str) -> String {
    let s = super::model::ident(name);
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// `getElementById` -> `get_element_by_id`, `innerHTML` -> `inner_html`.
fn snake(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::new();
    for (i, &c) in chars.iter().enumerate() {
        if c.is_uppercase() {
            let prev = i.checked_sub(1).map(|j| chars[j]);
            let next = chars.get(i + 1);
            // `rotate2D` is `rotate2d`; `toBase64Url`, `to_base64_url`.
            let after_word = prev.is_some_and(|p| p.is_lowercase() || (p.is_ascii_digit() && next.is_some_and(|n| n.is_lowercase())));
            let ends_acronym = prev.is_some_and(|p| p.is_uppercase()) && next.is_some_and(|n| n.is_lowercase());
            if (after_word || ends_acronym) && !out.is_empty() && !out.ends_with('_') {
                out.push('_');
            }
            out.extend(c.to_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

// `Options_mode` -> `OptionsMode`, `fill` -> `Fill`: for generated type names and variants.
fn camel(name: &str) -> String {
    let out: String = name
        .split('_')
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut cs = s.chars();
            cs.next().map(|c| c.to_uppercase().collect::<String>() + cs.as_str()).unwrap_or_default()
        })
        .collect();
    if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        format!("_{out}")
    } else if KEYWORDS.contains(&out.as_str()) {
        // `"self"` as a variant is `Self_`: `Self` is Rust's.
        format!("{out}_")
    } else {
        out
    }
}

#[derive(Default)]
struct Context {
    // Unions values can be decoded to, and all unions.
    decodable: BTreeSet<String>,
    unions: BTreeSet<String>,
    // Types declared without type parameters: `Is` is implemented for them.
    classes: BTreeSet<String>,
    // The Rust name of each type the model names: a namespace's as its
    // path, `stream::PipeOptions`.
    types: HashMap<String, String>,
    // A namespace's types' own names (`PipeOptions`), by the model's; and
    // all of them, which a top-level type's name may be too.
    own: HashMap<String, String>,
    leaves: HashSet<String>,
}

thread_local! {
    // The model being generated.
    static CX: RefCell<Context> = RefCell::new(Context::default());
}

fn decodable(n: &str) -> bool {
    CX.with(|c| c.borrow().decodable.contains(n))
}

fn is_union(n: &str) -> bool {
    CX.with(|c| c.borrow().unions.contains(n))
}

fn is_class(n: &str) -> bool {
    CX.with(|c| c.borrow().classes.contains(n))
}

// `Option`, in full where the API declares a type of that name.
fn option() -> &'static str {
    if CX.with(|c| c.borrow().classes.contains("Option") || c.borrow().types.contains_key("Option")) { "::core::option::Option" } else { "Option" }
}

// A type's name in Rust, where it is named: a namespace's by its path
// (`stream::PipeOptions`); a top-level one a namespace's type is also
// named, from the crate (`crate::PipeOptions`), as that would hide it there.
fn tn(n: &str) -> String {
    CX.with(|c| {
        let c = c.borrow();
        let t = c.types.get(n).cloned().unwrap_or_else(|| n.to_string());
        if !c.own.contains_key(n) && c.leaves.contains(&t) { format!("crate::{t}") } else { t }
    })
}

// A type's name where it is defined: a namespace's own (`PipeOptions`).
fn def_name(n: &str) -> String {
    CX.with(|c| {
        let c = c.borrow();
        c.own.get(n).cloned().or_else(|| c.types.get(n).cloned()).unwrap_or_else(|| n.to_string())
    })
}

const VALUE: &str = "JsValue";

// The Rust type of a value: owned, as it comes back or as a type argument.
fn ty(t: &Ty) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "String".into(),
        Ty::Any | Ty::Union(_) => VALUE.into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArray<{}>", ty(e)),
        Ty::Tuple(es) if (2..=4).contains(&es.len()) => format!("JsTuple{}<{}>", es.len(), es.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Tuple(_) => "JsArray".into(),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => tn(n),
        Ty::Nullable(inner, _) => format!("{}<{}>", option(), ty(inner)),
        Ty::NamedUnion(n) if decodable(n) => tn(n),
        Ty::NamedUnion(_) => VALUE.into(),
        Ty::Key(m) => format!("JsKey<{}, V>", tn(m)),
        Ty::Mapped(_) => "V".into(),
        Ty::Listener(ev) => format!("JsListener<{}>", ty(ev)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, _) if is_union(n) && !decodable(n) => VALUE.into(),
        Ty::Generic(n, args) if !args.is_empty() => format!("{}<{}>", tn(n), args.iter().map(ty).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) => tn(n),
    }
}

// A parameter: its type as written, and the expressions that lower it. What
// is lent lives until the end of the statement that makes the call.
fn param(t: &Ty, x: &str) -> (String, Vec<String>) {
    let lend = |e: &str| vec![format!("{e}.lend().h")];
    let into = |target: String| (format!("impl Into<{target}>"), vec![format!("Into::<{target}>::into({x}).lend().h")]);
    let option = |inner: String| format!("{}<{inner}>", option());
    match t {
        Ty::Num => ("f64".into(), vec![x.into()]),
        Ty::NumEnum(_) => (ty(t), vec![format!("{x} as i32 as f64")]),
        Ty::Bool | Ty::Enum(_) => (ty(t), vec![format!("{x} as i32")]),
        Ty::Key(_) => (ty(t), vec![format!("{x}.index")]),
        Ty::Str => ("&str".into(), vec![format!("{x}.as_ptr()"), format!("{x}.len()")]),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Str => (option("&str".into()), vec![format!("{x}.map_or(core::ptr::null(), |s| s.as_ptr())"), format!("{x}.map_or(0, |s| s.len())")]),
            Ty::Num => (ty(t), vec![format!("{x}.is_some() as i32"), format!("{x}.unwrap_or(0.0)")]),
            Ty::NumEnum(_) => (ty(t), vec![format!("{x}.is_some() as i32"), format!("{x}.map_or(0.0, |v| v as i32 as f64)")]),
            Ty::Bool | Ty::Enum(_) => (ty(t), vec![format!("{x}.map_or(-1, |v| v as i32)")]),
            // Any member, the union, or None: `options: true`, `Some(u)`, `None`.
            Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n) && decodable(n) => (format!("impl Optional<{}>", ty(inner)), vec![format!("Optional::<{}>::into_option({x}).lend().h", ty(inner))]),
            Ty::Param(_) => (ty(t), lend(x)),
            inner => (option(format!("&{}", ty(inner))), lend(x)),
        },
        // Any value that is one: `name_of(&shape)` where a Named is wanted.
        Ty::Obj(n) => (format!("&impl AsRef<{}>", tn(n)), vec![format!("AsRef::<{}>::as_ref({x}).lend().h", tn(n))]),
        Ty::Array | Ty::ArrayOf(_) => into(ty(t)),
        Ty::Tuple(es) if !(2..=4).contains(&es.len()) => into(ty(t)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n) => into(ty(t)),
        Ty::Any | Ty::Union(_) | Ty::NamedUnion(_) => into(VALUE.into()),
        // A value of a type parameter is passed as into a collection: by value.
        Ty::Param(_) => (ty(t), lend(x)),
        _ => (format!("&{}", ty(t)), lend(x)),
    }
}

// What a value crosses the boundary as, going in.
fn raw(t: &Ty) -> Vec<&'static str> {
    match t {
        Ty::Void => vec![],
        Ty::Num | Ty::NumEnum(_) => vec!["f64"],
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => vec!["i32"],
        Ty::Str => vec!["*const u8", "usize"],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec!["*const u8", "usize"],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => vec!["i32", "f64"],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => vec!["i32"],
        _ => vec!["u32"],
    }
}

// What a value crosses as coming back, and what a callback's arguments and result cross as.
fn raw_ret(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "()",
        Ty::Num | Ty::NumEnum(_) => "f64",
        Ty::Bool | Ty::Enum(_) => "i32",
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "i32",
        _ => "u32",
    }
}

// Expression turning what came back, `r`, into a Rust value.
fn raise(t: &Ty, r: &str) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => r.into(),
        Ty::NumEnum(e) => format!("{}::from_value({r} as i32)", tn(e)),
        Ty::Bool => format!("{r} != 0"),
        Ty::Enum(e) => format!("{}::from_index({r})", tn(e)),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("if {r} < 0 {{ None }} else {{ Some({r} != 0) }}"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("if {r} < 0 {{ None }} else {{ Some({}::from_index({r})) }}", ty(inner)),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => format!("::core::option::Option::<f64>::from_handle({r}).map(|v| {}::from_value(v as i32))", ty(inner)),
        t => format!("<{}>::from_handle({r})", ty(t)),
    }
}

// What a callback returns, given to JavaScript.
fn lower_callback(t: &Ty, x: &str) -> String {
    match t {
        Ty::Void => format!("{x};"),
        Ty::Num => x.into(),
        Ty::NumEnum(_) => format!("{x} as i32 as f64"),
        Ty::Bool | Ty::Enum(_) => format!("{x} as i32"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => format!("{x}.map_or(-1, |v| v as i32)"),
        _ => format!("crate::runtime::give(&{x})"),
    }
}

// A type parameter with its bounds: `T: JsType`, and for `T extends Node`,
// `T: JsType + Is<Node>`.
fn bound(p: &TParam) -> String {
    match &p.constraint {
        Some(Ty::Obj(n)) if is_class(n) => format!("{}: JsType + Is<{}>", p.name, tn(n)),
        _ => format!("{}: JsType", p.name),
    }
}

// `<T: JsType, U: JsType>`, or nothing.
fn generics(params: &[TParam]) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(bound).collect::<Vec<_>>().join(", ")) }
}

// `<T, U>`, or nothing.
fn args_of(params: &[TParam]) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

fn mentions(t: &Ty, param: &str) -> bool {
    match t {
        Ty::Param(n) => n == param,
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => mentions(inner, param),
        Ty::Generic(_, args) | Ty::Tuple(args) | Ty::Union(args) => args.iter().any(|a| mentions(a, param)),
        _ => false,
    }
}

// A function's name: a property `x` reads as `x()` and writes as `set_x(..)`.
// A `$` method (`$add`) reads as `add_new()` rather than `_add()`.
fn function_name_as(f: &Func, name: &str) -> String {
    let base = match &f.call {
        Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_) | Call::IndexGet => name.strip_prefix("get_").unwrap_or(name),
        _ => name,
    };
    let dollar = match &f.call {
        Call::Method(js) | Call::Static(js) | Call::Get(js) | Call::GetBound(js) | Call::Set(js) | Call::StaticGet(js) | Call::StaticSet(js) => {
            js.rsplit('.').next().is_some_and(|n| n.starts_with('$'))
        }
        _ => false,
    };
    let s = snake(base);
    if !dollar {
        return ident(&s);
    }
    let is_set = matches!(&f.call, Call::Set(_) | Call::StaticSet(_));
    let stem = if is_set { s.strip_prefix("set_").unwrap_or(&s) } else { &s }.trim_start_matches('_');
    let stem = if stem.is_empty() { "dollar" } else { stem };
    ident(&if is_set { format!("set_{stem}_new") } else { format!("{stem}_new") })
}

// Each function's Rust name, by the model's: `names::settle` made them apart.
fn names_of<'a>(funcs: impl Iterator<Item = &'a Func>) -> HashMap<String, String> {
    funcs.map(|f| (f.name.clone(), rust_function(f, &f.name))).collect()
}

// One function as source: `vis fn name<..>(..) -> R { .. }`. `this` is the
// handle of the object it is called on, for members. A type parameter that
// has a default and cannot be inferred from the arguments is two functions:
// `name`, with the default, and `name_as::<T>`.
fn function(f: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> Vec<String> {
    if hidden(f).is_empty() {
        return vec![one_function(f, name, vis, imports, this)];
    }
    vec![one_function(&defaulted(f), name, vis, imports, this), one_function(f, &format!("{name}_as"), vis, imports, this)]
}

fn one_function(f: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> String {
    let p = parts(f, imports, this);
    let generics = if p.generics.is_empty() { String::new() } else { format!("<{}>", p.generics.join(", ")) };
    let ret = if f.ret == Ty::Void { String::new() } else { format!(" -> {}", p.ret) };
    format!("{vis}fn {name}{generics}({}){ret} {{ {} }}", p.params.join(", "), p.body)
}

// A function's pieces: its bounds, its parameters as written (`&self`
// first, for a member), its result and its body.
struct Parts {
    generics: Vec<String>,
    params: Vec<String>,
    ret: String,
    body: String,
}

fn parts(f: &Func, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> Parts {
    let mut raw_params: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(this) = this {
        raw_params.push("this: u32".into());
        args.push(this.into());
    }
    let mut params: Vec<String> = if this.is_some() { vec!["&self".into()] } else { Vec::new() };
    for (i, (p, t)) in f.params.iter().enumerate() {
        let p = ident(&snake(p));
        let (written, lowered) = param(t, &p);
        params.push(format!("{p}: {written}"));
        raw_params.extend(raw(t).into_iter().enumerate().map(|(j, r)| format!("a{i}_{j}: {r}")));
        args.extend(lowered);
    }
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(this.is_some());
            raw_params.insert(at, "k: i32".into());
            args.insert(at, k.to_string());
            shared.clone()
        }
        None => f.import.clone(),
    };
    // An import named as a keyword (`use`) is linked by its name, and called by another.
    let raw_name = if KEYWORDS.contains(&import.as_str()) { format!("{import}_") } else { import.clone() };
    imports.entry(import.clone()).or_insert_with(|| {
        let link = if raw_name != import { format!("#[link_name = \"{import}\"] ") } else { String::new() };
        format!("    {link}pub fn {raw_name}({}) -> {};\n", raw_params.join(", "), raw_ret(&f.ret))
    });
    let import = raw_name;
    let mut type_params: Vec<String> = f.type_params.iter().map(bound).collect();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        type_params.insert(0, "V: JsType".into());
    }
    let body = match &f.ret {
        Ty::Void => format!("unsafe {{ raw::{import}({}) }};", args.join(", ")),
        t => format!("let r = unsafe {{ raw::{import}({}) }}; {}", args.join(", "), raise(t, "r")),
    };
    Parts { generics: type_params, params, ret: ty(&f.ret), body }
}

// Overloads of one method that differ in one parameter, which is a key of
// one map or another, or a string: one method, generic in that parameter.
// `el.query_selector(HTMLElementTagNameMap::P)` is an HTMLParagraphElement,
// `el.query_selector("p.intro")` an Element.
// Where they differ in other parameters too, those are the trait's type
// argument, which the chosen overload fixes: `add_event_listener(HTMLElementEventMap::CLICK,
// &listener, None)` takes a JsListener<PointerEvent>, `add_event_listener("ping", on_ping, None)`
// anything that is an EventListenerOrEventListenerObject.
struct Family {
    name: String,
    trait_name: String,
    at: usize,
    // The other parameters in which they differ.
    others: Vec<usize>,
    members: Vec<Func>,
}

// A parameter's type as an impl can name it: `impl Into<X>` becomes a type
// parameter, and a reference has the impl's lifetime.
fn nameable(written: &str, i: usize) -> (Vec<String>, String) {
    let g = format!("P{i}_");
    if let Some(bound) = written.strip_prefix("&impl ") {
        return (vec![format!("{g}: {bound}")], format!("&'a {g}"));
    }
    if let Some(bound) = written.strip_prefix("impl ") {
        return (vec![format!("{g}: {bound}")], g);
    }
    (Vec::new(), written.replace('&', "&'a "))
}

fn uses_key(t: &Ty) -> bool {
    match t {
        Ty::Key(_) | Ty::Mapped(_) => true,
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => uses_key(inner),
        Ty::Generic(_, args) | Ty::Tuple(args) | Ty::Union(args) => args.iter().any(uses_key),
        _ => false,
    }
}

fn families(c: &Class, names: &HashMap<String, String>, types: &HashSet<String>) -> Vec<Family> {
    if !c.type_params.is_empty() {
        return Vec::new();
    }
    let mut groups: Vec<(String, Vec<&Func>)> = Vec::new();
    for f in &c.funcs {
        let Call::Method(js) = &f.call else { continue };
        let key = format!("{js} {:?}", f.fixed);
        match groups.iter_mut().find(|g| g.0 == key) {
            Some(g) => g.1.push(f),
            None => groups.push((key, vec![f])),
        }
    }
    let mut out = Vec::new();
    for (_, fs) in groups.into_iter().filter(|g| g.1.len() > 1) {
        let first = fs[0];
        if fs.iter().any(|f| f.rest || f.params.len() != first.params.len()) {
            continue;
        }
        let differing: Vec<usize> = (0..first.params.len()).filter(|&i| fs.iter().any(|f| f.params[i].1 != first.params[i].1)).collect();
        // The first that tells them apart: a key of one map or another, or a string.
        let tells = |at: usize| {
            let mut seen = HashSet::new();
            fs.iter().all(|f| matches!(f.params[at].1, Ty::Key(_) | Ty::Str) && seen.insert(format!("{:?}", f.params[at].1)))
        };
        let Some(&at) = differing.iter().find(|&&i| tells(i)) else { continue };
        let others: Vec<usize> = differing.iter().copied().filter(|&i| i != at).collect();
        // The rest are written once, for every overload.
        let plain = fs.iter().all(|f| f.params.iter().enumerate().all(|(i, (_, t))| differing.contains(&i) || !(uses_key(t) || f.type_params.iter().any(|p| mentions(t, &p.name)))));
        let name = names[&first.name].clone();
        let trait_name = format!("{}{}Arg", tn(&c.name), camel(&name));
        if plain && !types.contains(&trait_name) {
            out.push(Family { name, trait_name, at, others, members: fs.into_iter().map(|f| with_self_args(f, c)).collect() });
        }
    }
    out
}

impl Family {
    // `<R1_, R2_>` for the other differing parameters, or nothing.
    fn rest(&self) -> String {
        if self.others.is_empty() { String::new() } else { format!("<{}>", self.others.iter().map(|i| format!("R{i}_")).collect::<Vec<_>>().join(", ")) }
    }

    // The method: `fn query_selector<A_: ..Arg>(&self, selectors: A_) -> A_::Output`,
    // and for an overload with a type argument to state, `query_selector_as::<T>`.
    fn methods(&self, vis: &str, imports: &mut BTreeMap<String, String>, this: &str) -> Vec<String> {
        let first = &self.members[0];
        let names: Vec<String> = first.params.iter().map(|(p, _)| ident(&snake(p))).collect();
        let params: Vec<String> = first
            .params
            .iter()
            .zip(&names)
            .enumerate()
            .map(|(i, ((_, t), p))| if i == self.at { format!("{p}: A_") } else if self.others.contains(&i) { format!("{p}: R{i}_") } else { format!("{p}: {}", param(t, p).0) })
            .collect();
        let rest = if self.others.is_empty() { String::new() } else { format!(", ({},)", self.others.iter().map(|&i| names[i].as_str()).collect::<Vec<_>>().join(", ")) };
        let same: String = names.iter().enumerate().filter(|(i, _)| *i != self.at && !self.others.contains(i)).map(|(_, p)| format!(", {p}")).collect();
        let generics: String = self.others.iter().map(|i| format!(", R{i}_")).collect();
        let mut out = vec![format!("{vis}fn {}<A_: {}{}{generics}>(&self, {}) -> A_::Output {{ A_::call({}, {this}{rest}{same}) }}", self.name, self.trait_name, self.rest(), params.join(", "), names[self.at])];
        if let Some(f) = self.members.iter().find(|f| f.params[self.at].1 == Ty::Str && !hidden(f).is_empty()) {
            out.push(one_function(f, &format!("{}_as", self.name), vis, imports, Some(this)));
        }
        out
    }

    // The trait of what the parameter may be, and each overload as one of them.
    fn dispatch(&self, imports: &mut BTreeMap<String, String>) -> String {
        let same = |p: &Parts| -> String { p.params.iter().skip(1).enumerate().filter(|(i, _)| *i != self.at && !self.others.contains(i)).map(|(_, p)| format!(", {p}")).collect() };
        let first = &self.members[0];
        let p = parts(first, &mut BTreeMap::new(), Some("this"));
        let rest_param = if self.others.is_empty() { String::new() } else { format!(", rest: ({},)", self.others.iter().map(|i| format!("R{i}_")).collect::<Vec<_>>().join(", ")) };
        let trait_params = if self.others.is_empty() { String::new() } else { format!("<{}>", self.others.iter().map(|i| format!("R{i}_")).collect::<Vec<_>>().join(", ")) };
        let own = self.trait_name.rsplit("::").next().unwrap_or_default();
        let mut out = format!("pub trait {own}{trait_params} {{\n    type Output;\n    #[doc(hidden)]\n    fn call(self, this: u32{rest_param}{}) -> Self::Output;\n}}\n", same(&p));
        for f in &self.members {
            let f = &defaulted(f);
            let p = parts(f, imports, Some("this"));
            let key = ident(&snake(&f.params[self.at].0));
            let mut generics = p.generics.clone();
            let (mut rest_types, mut rest_names) = (Vec::new(), Vec::new());
            for &i in &self.others {
                let name = ident(&snake(&f.params[i].0));
                let (bounds, t) = nameable(&param(&f.params[i].1, &name).0, i);
                generics.extend(bounds);
                rest_types.push(t);
                rest_names.push(name);
            }
            let (self_generics, self_type) = nameable(&param(&f.params[self.at].1, &key).0, self.at);
            generics.extend(self_generics);
            let header = format!("{self_type} {}", rest_types.join(" "));
            if header.contains("&'a") {
                generics.insert(0, "'a".into());
            }
            let generics = if generics.is_empty() { String::new() } else { format!("<{}>", generics.join(", ")) };
            let (trait_args, rest_param, unpack) = if self.others.is_empty() {
                (String::new(), String::new(), String::new())
            } else {
                (format!("<{}>", rest_types.join(", ")), format!(", rest: ({},)", rest_types.join(", ")), format!("let ({},) = rest; ", rest_names.join(", ")))
            };
            out += &format!(
                "impl{generics} {}{trait_args} for {self_type} {{\n    type Output = {};\n    fn call(self, this: u32{rest_param}{}) -> Self::Output {{ let {key} = self; {unpack}{} }}\n}}\n",
                self.trait_name,
                p.ret,
                same(&p),
                p.body
            );
        }
        out
    }
}

// Type parameters with a default that the arguments cannot infer.
fn hidden(f: &Func) -> Vec<TParam> {
    f.type_params.iter().filter(|p| p.default.is_some() && !f.params.iter().any(|(_, t)| mentions(t, &p.name))).cloned().collect()
}

// The function with those parameters at their defaults.
fn defaulted(f: &Func) -> Func {
    let hidden = hidden(f);
    let mut out = map_types(f, &|t| subst(t, &hidden, &[]));
    out.type_params.retain(|p| !hidden.iter().any(|h| h.name == p.name));
    out
}

const RUNTIME: &str = r#"// Generated. The runtime: handles, conversions, arrays, callbacks, promises.
use core::ffi::c_void;
use core::marker::PhantomData;
use std::any::Any;
use std::cell::RefCell;
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

#[allow(dead_code)]
pub(crate) mod rt {
    use core::ffi::c_void;
    #[link(wasm_import_module = "env")]
    unsafe extern "C" {
        pub fn js_string_len(h: u32) -> u32;
        pub fn js_string_read(h: u32, p: *mut u8);
        pub fn js_string_take(h: u32, p: *mut u8, cap: usize) -> u32;
        pub fn js_from_number(x: f64) -> u32;
        pub fn js_from_bool(x: i32) -> u32;
        pub fn js_from_string(p: *const u8, n: usize) -> u32;
        pub fn js_to_number(h: u32) -> f64;
        pub fn js_to_bool(h: u32) -> i32;
        pub fn js_equals(a: u32, b: u32) -> i32;
        pub fn js_array_new() -> u32;
        pub fn js_array_length(h: u32) -> u32;
        pub fn js_array_get(h: u32, i: u32) -> u32;
        pub fn js_array_push(h: u32, v: u32);
        pub fn js_listener_from(f: usize, ctx: *mut c_void) -> u32;
        pub fn js_callback_release(h: u32) -> u32;
        pub fn js_retain(h: u32);
        pub fn js_release(h: u32);
        pub fn js_live() -> u32;
        pub fn js_take_error() -> u32;
        pub fn js_error_message(h: u32) -> u32;
        pub fn js_error_name(h: u32) -> u32;
        pub fn js_error_stack(h: u32) -> u32;
        pub fn js_error_new(p: *const u8, n: usize) -> u32;
        pub fn js_date_new(ms: f64) -> u32;
        pub fn js_date_now() -> f64;
        pub fn js_date_time(h: u32) -> f64;
        pub fn js_date_to_iso_string(h: u32) -> u32;
        pub fn js_bytes_len(h: u32) -> u32;
        pub fn js_bytes_read(h: u32, off: usize, dst: *mut u8, len: usize) -> u32;
        pub fn js_bytes_write(h: u32, off: usize, src: *const u8, len: usize) -> u32;
        pub fn js_bytes_new(kind: u32, src: *const u8, byte_len: usize) -> u32;
        pub fn js_bytes_buffer(h: u32) -> u32;
        pub fn js_bytes_offset(h: u32) -> u32;
        pub fn js_promise_then(p: u32, f: usize, ctx: *mut c_void);
        pub fn js_iterate(h: u32) -> u32;
        pub fn js_iterator_next(it: u32) -> u32;
        pub fn js_async_iterate(h: u32) -> u32;
        pub fn js_async_iterator_next(it: u32) -> u32;
    }
}

// A JavaScript value, counted: `Clone` retains it, `Drop` releases it. Zero is null.
pub struct JsHandle(pub u32);
impl Clone for JsHandle {
    fn clone(&self) -> Self {
        if self.0 != 0 {
            unsafe { rt::js_retain(self.0) }
        }
        JsHandle(self.0)
    }
}
impl Drop for JsHandle {
    fn drop(&mut self) {
        if self.0 != 0 {
            unsafe { rt::js_release(self.0) }
        }
    }
}

// A value lent to JavaScript for one call: an object's own handle, or a new
// one for a number, flag or string, released after the call.
pub struct Lent {
    pub h: u32,
    own: bool,
}
impl Lent {
    pub fn borrow(h: u32) -> Self {
        Lent { h, own: false }
    }
    pub fn owned(h: u32) -> Self {
        Lent { h, own: true }
    }
}
impl Drop for Lent {
    fn drop(&mut self) {
        if self.own && self.h != 0 {
            unsafe { rt::js_release(self.h) }
        }
    }
}

// A value that crosses as a handle, going in and coming back. `from_handle`
// takes the handle over. KIND tells a bare type parameter apart in a union.
pub trait ToJs {
    fn lend(&self) -> Lent;
}
pub trait FromJs: Sized {
    const KIND: i32 = 0;
    fn from_handle(h: u32) -> Self;
}
pub trait JsType: FromJs + ToJs + Clone + 'static {}
impl<T: FromJs + ToJs + Clone + 'static> JsType for T {}

// What an optional union argument may be: any member, as the union is made
// from it, or an Option of the union (each union implements it for that).
pub trait Optional<U> {
    fn into_option(self) -> Option<U>;
}
impl<U, T: Into<U>> Optional<U> for T {
    fn into_option(self) -> Option<U> {
        Some(self.into())
    }
}

// `T: Is<B>` says a T is a B: itself, a subtype of it, a union of such, or
// any value. It is what a type parameter's `extends` becomes.
pub trait Is<B> {}
impl<B> Is<B> for JsValue {}

// A handle JavaScript keeps: what a callback returns.
#[doc(hidden)]
pub fn give<T: ToJs + ?Sized>(v: &T) -> u32 {
    let l = v.lend();
    let h = l.h;
    if !l.own && h != 0 {
        unsafe { rt::js_retain(h) }
    }
    core::mem::forget(l);
    h
}

fn read_string(h: u32) -> String {
    let n = unsafe { rt::js_string_len(h) } as usize;
    let mut v = vec![0u8; n];
    unsafe { rt::js_string_read(h, v.as_mut_ptr()) };
    String::from_utf8(v).unwrap_or_default()
}

impl<T: ToJs + ?Sized> ToJs for &T {
    fn lend(&self) -> Lent {
        (**self).lend()
    }
}
impl ToJs for f64 {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_number(*self) })
    }
}
impl FromJs for f64 {
    const KIND: i32 = 1;
    fn from_handle(h: u32) -> Self {
        let x = unsafe { rt::js_to_number(h) };
        unsafe { rt::js_release(h) };
        x
    }
}
impl ToJs for bool {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_bool(*self as i32) })
    }
}
impl FromJs for bool {
    const KIND: i32 = 3;
    fn from_handle(h: u32) -> Self {
        let x = unsafe { rt::js_to_bool(h) } != 0;
        unsafe { rt::js_release(h) };
        x
    }
}
impl ToJs for str {
    fn lend(&self) -> Lent {
        Lent::owned(unsafe { rt::js_from_string(self.as_ptr(), self.len()) })
    }
}
impl ToJs for String {
    fn lend(&self) -> Lent {
        self.as_str().lend()
    }
}
impl FromJs for String {
    const KIND: i32 = 2;
    // In one call, when it fits a small buffer.
    fn from_handle(h: u32) -> Self {
        let mut buf = [0u8; 256];
        let n = unsafe { rt::js_string_take(h, buf.as_mut_ptr(), buf.len()) } as usize;
        if n <= buf.len() {
            return String::from_utf8(buf[..n].to_vec()).unwrap_or_default();
        }
        let s = read_string(h);
        unsafe { rt::js_release(h) };
        s
    }
}
impl ToJs for () {
    fn lend(&self) -> Lent {
        Lent::borrow(0)
    }
}
impl FromJs for () {
    fn from_handle(h: u32) -> Self {
        if h != 0 {
            unsafe { rt::js_release(h) }
        }
    }
}
impl<T: ToJs> ToJs for Option<T> {
    fn lend(&self) -> Lent {
        match self {
            Some(x) => x.lend(),
            None => Lent::borrow(0),
        }
    }
}
impl<T: FromJs> FromJs for Option<T> {
    fn from_handle(h: u32) -> Self {
        if h == 0 { None } else { Some(T::from_handle(h)) }
    }
}

// Declares a handle type, generic or not. `==` is whether two handles are
// one JavaScript value.
macro_rules! handle {
    ($name:ident $(<$($p:ident),*>)?) => {
        #[repr(transparent)]
        pub struct $name $(<$($p),*>)? (pub $crate::runtime::JsHandle $(, pub core::marker::PhantomData<fn() -> ($($p,)*)>)?);
        impl $(<$($p),*>)? AsRef<$name $(<$($p),*>)?> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &Self {
                self
            }
        }
        impl $(<$($p),*>)? Clone for $name $(<$($p),*>)? {
            fn clone(&self) -> Self {
                $name(self.0.clone() $(, core::marker::PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
        impl $(<$($p),*>)? AsRef<$crate::runtime::JsHandle> for $name $(<$($p),*>)? {
            fn as_ref(&self) -> &$crate::runtime::JsHandle {
                &self.0
            }
        }
        impl $(<$($p),*>)? PartialEq for $name $(<$($p),*>)? {
            fn eq(&self, o: &Self) -> bool {
                $crate::runtime::js::same(&self.0, &o.0)
            }
        }
        impl $(<$($p),*>)? $crate::runtime::ToJs for $name $(<$($p),*>)? {
            fn lend(&self) -> $crate::runtime::Lent {
                $crate::runtime::Lent::borrow(self.0 .0)
            }
        }
        impl $(<$($p),*>)? $crate::runtime::FromJs for $name $(<$($p),*>)? {
            fn from_handle(h: u32) -> Self {
                $name($crate::runtime::JsHandle(h) $(, core::marker::PhantomData::<fn() -> ($($p,)*)>)?)
            }
        }
    };
}
pub(crate) use handle;

// A JavaScript value of no particular type, made from a number, a flag, a
// string, or any object: `f(42.0)`, `f("text")`, `f(&element)`.
handle!(JsValue);
impl JsValue {
    pub fn as_f64(&self) -> f64 {
        unsafe { rt::js_to_number(self.0 .0) }
    }
    pub fn as_bool(&self) -> bool {
        unsafe { rt::js_to_bool(self.0 .0) != 0 }
    }
    pub fn as_string(&self) -> String {
        read_string(self.0 .0)
    }
    pub fn is_null(&self) -> bool {
        self.0 .0 == 0
    }
    // `null`, for an argument of any type given nothing.
    pub fn null() -> Self {
        JsValue(JsHandle(0))
    }
}
impl From<f64> for JsValue {
    fn from(x: f64) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_number(x) }))
    }
}
impl From<bool> for JsValue {
    fn from(x: bool) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_bool(x as i32) }))
    }
}
impl From<&str> for JsValue {
    fn from(s: &str) -> Self {
        JsValue(JsHandle(unsafe { rt::js_from_string(s.as_ptr(), s.len()) }))
    }
}
impl From<String> for JsValue {
    fn from(s: String) -> Self {
        s.as_str().into()
    }
}
impl<T: AsRef<JsHandle>> From<&T> for JsValue {
    fn from(x: &T) -> Self {
        JsValue(x.as_ref().clone())
    }
}

// A JavaScript array of T; `JsArray` of any values. Made from an array, a
// slice, a Vec or an iterator of what T is made from: `["a", "b"]`.
pub struct JsArray<T = JsValue>(pub JsHandle, PhantomData<fn() -> T>);
impl<T> Clone for JsArray<T> {
    fn clone(&self) -> Self {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T> ToJs for JsArray<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for JsArray<T> {
    fn from_handle(h: u32) -> Self {
        JsArray(JsHandle(h), PhantomData)
    }
}
impl<T: JsType> JsArray<T> {
    pub fn new() -> Self {
        JsArray(JsHandle(unsafe { rt::js_array_new() }), PhantomData)
    }
    pub fn len(&self) -> usize {
        unsafe { rt::js_array_length(self.0 .0) as usize }
    }
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
    pub fn get(&self, i: usize) -> T {
        T::from_handle(unsafe { rt::js_array_get(self.0 .0, i as u32) })
    }
    pub fn push(&self, v: impl Into<T>) {
        let v: T = v.into();
        unsafe { rt::js_array_push(self.0 .0, v.lend().h) }
    }
    // Each element in turn.
    pub fn iter(&self) -> impl Iterator<Item = T> + '_ {
        (0..self.len()).map(|i| self.get(i))
    }
    // The same array, untyped.
    pub fn any(&self) -> JsArray {
        JsArray(self.0.clone(), PhantomData)
    }
}
impl<T: JsType> Default for JsArray<T> {
    fn default() -> Self {
        Self::new()
    }
}
impl<T: JsType, A: Into<T>> FromIterator<A> for JsArray<T> {
    fn from_iter<I: IntoIterator<Item = A>>(items: I) -> Self {
        let a = JsArray::new();
        for x in items {
            a.push(x);
        }
        a
    }
}
impl<T: JsType, A: Into<T>, const N: usize> From<[A; N]> for JsArray<T> {
    fn from(items: [A; N]) -> Self {
        items.into_iter().collect()
    }
}
impl<T: JsType, A: Into<T>> From<Vec<A>> for JsArray<T> {
    fn from(items: Vec<A>) -> Self {
        items.into_iter().collect()
    }
}
impl<T: JsType, A: Into<T> + Clone> From<&[A]> for JsArray<T> {
    fn from(items: &[A]) -> Self {
        items.iter().cloned().collect()
    }
}
impl<T> From<&JsArray<T>> for JsArray<T> {
    fn from(a: &JsArray<T>) -> Self {
        a.clone()
    }
}

// Arrays of fixed length and element types: `[number, string]` is `JsTuple2<f64, String>`.
macro_rules! tuple {
    ($name:ident; $($t:ident $get:ident $i:literal),*) => {
        pub struct $name<$($t),*>(pub JsHandle, PhantomData<fn() -> ($($t,)*)>);
        impl<$($t),*> Clone for $name<$($t),*> {
            fn clone(&self) -> Self {
                $name(self.0.clone(), PhantomData)
            }
        }
        impl<$($t),*> ToJs for $name<$($t),*> {
            fn lend(&self) -> Lent {
                Lent::borrow(self.0 .0)
            }
        }
        impl<$($t),*> FromJs for $name<$($t),*> {
            fn from_handle(h: u32) -> Self {
                $name(JsHandle(h), PhantomData)
            }
        }
        impl<$($t: JsType),*> $name<$($t),*> {
            pub fn new($($get: impl Into<$t>),*) -> Self {
                let a = unsafe { rt::js_array_new() };
                $(unsafe { rt::js_array_push(a, Into::<$t>::into($get).lend().h) };)*
                $name(JsHandle(a), PhantomData)
            }
            $(pub fn $get(&self) -> $t {
                $t::from_handle(unsafe { rt::js_array_get(self.0 .0, $i) })
            })*
        }
    };
}
tuple!(JsTuple2; A get0 0, B get1 1);
tuple!(JsTuple3; A get0 0, B get1 1, C get2 2);
tuple!(JsTuple4; A get0 0, B get1 1, C get2 2, D get3 3);

// A key of a map whose value has type V: `HTMLElementEventMap::CLICK`.
pub struct JsKey<M, V> {
    pub index: i32,
    _p: PhantomData<fn() -> (M, V)>,
}
impl<M, V> JsKey<M, V> {
    pub const fn new(index: i32) -> Self {
        JsKey { index, _p: PhantomData }
    }
}
impl<M, V> Clone for JsKey<M, V> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<M, V> Copy for JsKey<M, V> {}

// A closure kept for JavaScript to call, until released.
pub(crate) fn closure<F: 'static>(f: F) -> *mut c_void {
    Box::into_raw(Box::new(Box::new(f) as Box<dyn Any>)) as *mut c_void
}
pub(crate) unsafe fn closure_fn<'a, F: 'static>(ctx: *mut c_void) -> &'a mut F {
    unsafe { (*(ctx as *mut Box<dyn Any>)).downcast_mut::<F>().unwrap() }
}

// Forgets a callback when dropped: JavaScript calling it afterwards is an
// error there, never a call into a closure that is gone.
pub(crate) struct Forget(pub u32);
impl Drop for Forget {
    fn drop(&mut self) {
        unsafe { rt::js_callback_release(self.0) };
    }
}

// A JavaScript function made from a closure. One made with `new` lives until
// `release`, which frees the closure; one made with `scoped` lives for the
// length of a call, and its closure may borrow.
pub trait JsCallback: AsRef<JsHandle> + Sized {
    fn release(self) {
        let ctx = unsafe { rt::js_callback_release(AsRef::<JsHandle>::as_ref(&self).0) };
        if ctx != 0 {
            drop(unsafe { Box::from_raw(ctx as usize as *mut Box<dyn Any>) });
        }
    }
}

// A callback taking one object of type E: `JsListener::new(|e: PointerEvent| ..)`.
pub struct JsListener<E>(pub JsHandle, PhantomData<fn() -> E>);
impl<E> Clone for JsListener<E> {
    fn clone(&self) -> Self {
        JsListener(self.0.clone(), PhantomData)
    }
}
impl<E> AsRef<JsHandle> for JsListener<E> {
    fn as_ref(&self) -> &JsHandle {
        &self.0
    }
}
impl<E> ToJs for JsListener<E> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<E> FromJs for JsListener<E> {
    fn from_handle(h: u32) -> Self {
        JsListener(JsHandle(h), PhantomData)
    }
}
impl<E> JsCallback for JsListener<E> {}
impl<E: JsType> JsListener<E> {
    pub fn new<F: FnMut(E) + 'static>(f: F) -> Self {
        extern "C" fn call<E: JsType, F: FnMut(E) + 'static>(ctx: *mut c_void, ev: u32) {
            (unsafe { closure_fn::<F>(ctx) })(E::from_handle(ev))
        }
        JsListener(JsHandle(unsafe { rt::js_listener_from(call::<E, F> as *const () as usize, closure(f)) }), PhantomData)
    }
    // A listener for the length of `body`: its closure may borrow.
    pub fn scoped<F: FnMut(E), R>(mut f: F, body: impl FnOnce(&Self) -> R) -> R {
        extern "C" fn call<E: JsType, F: FnMut(E)>(ctx: *mut c_void, ev: u32) {
            (unsafe { &mut *(ctx as *mut F) })(E::from_handle(ev))
        }
        let l = JsListener(JsHandle(unsafe { rt::js_listener_from(call::<E, F> as *const () as usize, &mut f as *mut F as *mut c_void) }), PhantomData);
        let _forget = Forget(l.0 .0);
        body(&l)
    }
}

// A JavaScript exception, or the reason a promise was rejected.
handle!(JsError);
impl JsError {
    pub fn new(msg: &str) -> Self {
        JsError(JsHandle(unsafe { rt::js_error_new(msg.as_ptr(), msg.len()) }))
    }
    pub fn name(&self) -> String {
        String::from_handle(unsafe { rt::js_error_name(self.0 .0) })
    }
    pub fn message(&self) -> String {
        String::from_handle(unsafe { rt::js_error_message(self.0 .0) })
    }
    pub fn stack(&self) -> String {
        String::from_handle(unsafe { rt::js_error_stack(self.0 .0) })
    }
}
impl core::fmt::Debug for JsError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "JsError({}: {:?})", self.name(), self.message())
    }
}

// What is not a type's: errors, handles, tasks.
pub mod js {
    use super::*;

    pub fn date_now() -> f64 {
        unsafe { rt::js_date_now() }
    }

    // The exception a call threw, taken. A call that throws returns a zero
    // value, and an exception not taken by the time control returns to
    // JavaScript is reported there: see `attempt`.
    pub fn take_error() -> Option<JsError> {
        let e = unsafe { rt::js_take_error() };
        if e == 0 { None } else { Some(JsError(JsHandle(e))) }
    }

    // Runs f and gives what it returned, or the exception a call in it threw:
    // `let found = js::attempt(|| el.query_selector(text))?;`.
    pub fn attempt<T>(f: impl FnOnce() -> T) -> Result<T, JsError> {
        let x = f();
        match take_error() {
            Some(e) => Err(e),
            None => Ok(x),
        }
    }

    // How many references to JavaScript values wasm holds.
    pub fn live_handles() -> u32 {
        unsafe { rt::js_live() }
    }

    // Whether two handles are one JavaScript value: what `==` means for every handle type.
    pub fn same(a: &JsHandle, b: &JsHandle) -> bool {
        a.0 == b.0 || unsafe { rt::js_equals(a.0, b.0) != 0 }
    }

    // Runs a future to its end, across the promises it awaits:
    // `js::spawn(async move { let r = fetch(url, None).await; .. })`.
    pub fn spawn(f: impl Future<Output = ()> + 'static) {
        poll(&Rc::new(Task { future: RefCell::new(Some(Box::pin(f))) }));
    }

    struct Task {
        future: RefCell<Option<Pin<Box<dyn Future<Output = ()>>>>>,
    }

    fn poll(task: &Rc<Task>) {
        let waker = waker(task.clone());
        // A wake during its own poll would find the future borrowed: a promise never does that.
        let Ok(mut slot) = task.future.try_borrow_mut() else { return };
        if let Some(f) = slot.as_mut() {
            if f.as_mut().poll(&mut Context::from_waker(&waker)).is_ready() {
                *slot = None;
            }
        }
    }

    // One thread, so a waker is a counted pointer to its task.
    fn waker(task: Rc<Task>) -> Waker {
        unsafe fn clone(p: *const ()) -> RawWaker {
            unsafe { Rc::increment_strong_count(p as *const Task) };
            RawWaker::new(p, &VTABLE)
        }
        unsafe fn wake(p: *const ()) {
            poll(&unsafe { Rc::from_raw(p as *const Task) });
        }
        unsafe fn wake_by_ref(p: *const ()) {
            let task = unsafe { Rc::from_raw(p as *const Task) };
            poll(&task);
            core::mem::forget(task);
        }
        unsafe fn forget(p: *const ()) {
            drop(unsafe { Rc::from_raw(p as *const Task) });
        }
        static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, wake, wake_by_ref, forget);
        unsafe { Waker::from_raw(RawWaker::new(Rc::into_raw(task) as *const (), &VTABLE)) }
    }
}

// A JavaScript promise. `then` calls a closure with its result once it
// settles; in an async block, `.await` waits for it.
pub struct JsPromise<T = ()>(pub JsHandle, PhantomData<fn() -> T>);
impl<T> Clone for JsPromise<T> {
    fn clone(&self) -> Self {
        JsPromise(self.0.clone(), PhantomData)
    }
}
impl<T> ToJs for JsPromise<T> {
    fn lend(&self) -> Lent {
        Lent::borrow(self.0 .0)
    }
}
impl<T> FromJs for JsPromise<T> {
    fn from_handle(h: u32) -> Self {
        JsPromise(JsHandle(h), PhantomData)
    }
}
impl<T: JsType> JsPromise<T> {
    pub fn then<F: FnOnce(Result<T, JsError>) + 'static>(&self, f: F) {
        extern "C" fn settle<T: JsType, F: FnOnce(Result<T, JsError>) + 'static>(ctx: *mut c_void, ok: i32, v: u32) {
            let mut b = unsafe { Box::from_raw(ctx as *mut Box<dyn Any>) };
            let f = b.downcast_mut::<Option<F>>().unwrap().take().unwrap();
            f(if ok != 0 { Ok(T::from_handle(v)) } else { Err(JsError(JsHandle(v))) })
        }
        unsafe { rt::js_promise_then(self.0 .0, settle::<T, F> as *const () as usize, closure(Some(f))) }
    }
}

// What awaiting a promise polls: its result, once the promise has settled.
pub struct JsFuture<T> {
    state: Rc<RefCell<(Option<Result<T, JsError>>, Option<Waker>)>>,
}
impl<T> Future for JsFuture<T> {
    type Output = Result<T, JsError>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut state = self.state.borrow_mut();
        match state.0.take() {
            Some(r) => Poll::Ready(r),
            None => {
                state.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}
impl<T: JsType> IntoFuture for JsPromise<T> {
    type Output = Result<T, JsError>;
    type IntoFuture = JsFuture<T>;
    fn into_future(self) -> JsFuture<T> {
        let state: Rc<RefCell<(Option<Result<T, JsError>>, Option<Waker>)>> = Rc::new(RefCell::new((None, None)));
        let settled = state.clone();
        self.then(move |r| {
            let waker = {
                let mut state = settled.borrow_mut();
                state.0 = Some(r);
                state.1.take()
            };
            if let Some(w) = waker {
                w.wake();
            }
        });
        JsFuture { state }
    }
}

// Walks a JavaScript iterator: `for n in list.iter()`. Each value is the loop's.
pub struct JsIterator<T>(JsHandle, PhantomData<fn() -> T>);
impl<T: JsType> JsIterator<T> {
    pub fn over(h: u32) -> Self {
        JsIterator(JsHandle(unsafe { rt::js_iterate(h) }), PhantomData)
    }
}
impl<T: JsType> Iterator for JsIterator<T> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        let v = unsafe { rt::js_iterator_next(self.0 .0) };
        if v == u32::MAX { None } else { Some(T::from_handle(v)) }
    }
}

// A JavaScript async iterator: `next()` is a promise of the next value, or of None at the end.
pub struct JsAsyncIterator<T>(JsHandle, PhantomData<fn() -> T>);
impl<T: JsType> JsAsyncIterator<T> {
    pub fn over(h: u32) -> Self {
        JsAsyncIterator(JsHandle(unsafe { rt::js_async_iterate(h) }), PhantomData)
    }
    pub fn next(&self) -> JsPromise<Option<T>> {
        JsPromise::from_handle(unsafe { rt::js_async_iterator_next(self.0 .0) })
    }
}
"#;

// What the model names that the runtime declares under another name.
const RUNTIME_TYPES: &[(&str, &str)] = &[("Promise", "JsPromise"), ("PromiseLike", "JsPromise")];

// A model type's Rust name: CamelCase (a type named in snake_case would
// share its constructor's name with a function), or the runtime's.
fn rust_type(n: &str) -> String {
    // A bundler's `SeriesOption$1` is `SeriesOption1`.
    RUNTIME_TYPES.iter().find(|r| r.0 == n).map_or_else(|| camel(&ident(n)), |r| r.1.to_string())
}

// A tuple struct is also a function of its name: types and functions share
// a namespace. A type's functions are in its `impl`, beside what every
// handle has (`clone`, `lend`); a class's inherited ones in `XMethods`.
pub const NAMING: Naming = Naming {
    // The runtime's own types; std's (`Box`) a type of the crate may shadow.
    reserved: || {
        let words = identifiers(RUNTIME);
        let mut declared: Vec<String> = Vec::new();
        // At the top of the file, not an associated `type Item` in an `impl`.
        for line in RUNTIME.lines().filter(|l| !l.starts_with(char::is_whitespace)) {
            let mut it = line.split(|c: char| !(c.is_alphanumeric() || c == '_')).filter(|w| !w.is_empty());
            while let Some(w) = it.next() {
                if matches!(w, "struct" | "trait" | "enum" | "type")
                    && let Some(name) = it.next()
                {
                    declared.push(name.to_string());
                }
            }
        }
        KEYWORDS.iter().map(|k| k.to_string()).chain(declared.into_iter().filter(|d| words.contains(d))).collect()
    },
    type_names: |candidate, _, kind, _| {
        if RUNTIME_TYPES.iter().any(|r| r.0 == candidate) {
            return Vec::new();
        }
        let t = rust_type(candidate);
        if kind == Kind::Class { vec![t.clone(), format!("{t}Methods")] } else { vec![t] }
    },
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: false,
    func_name: |_, name, f| Some(rust_function(f, name)),
    funcs_flat: false,
    nests: true,
    members: |_, _| TAKEN.iter().map(|t| t.to_string()).collect(),
};

// A function's Rust name, were its model name `name`: what every handle has
// already (`clone`) is `js_clone`.
fn rust_function(f: &Func, name: &str) -> String {
    let base = function_name_as(f, name);
    if TAKEN.contains(&base.as_str()) { format!("js_{base}") } else { base }
}

// What every file of the crate starts with.
const PRELUDE: &str = "// Generated.\n#![allow(unused_imports)]\nuse crate::raw;\nuse crate::runtime::handle;\nuse crate::*;\nuse core::marker::PhantomData;\n\n";

// A union's variant: `Number`, `String`, `Element`, `Array`, or a type parameter.
fn variant(t: &Ty) -> String {
    match t {
        Ty::Num => "Number".into(),
        Ty::Str => "String".into(),
        Ty::Bool => "Boolean".into(),
        // A namespace's type by its own name: `ExportValue::Global`.
        Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) => def_name(n),
        Ty::Param(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "Array".into(),
        t => panic!("Rust generator: a union member it cannot hold: {t:?}"),
    }
}

// A module's file: `areas/core` is `src/areas_core.rs`.
fn module_file(module: &str) -> String {
    let name: String = snake(&module.replace('/', "_")).chars().map(|c| if c.is_alphanumeric() || c == '_' { c } else { '_' }).collect();
    if name.is_empty() { "main".into() } else { name }
}

// The crate: see the top of this file.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    // Each type's Rust name: `names::settle` made them apart.
    let declared: Vec<&String> = m.enums.iter().map(|e| &e.0).chain(m.num_enums.iter().map(|e| &e.0)).chain(m.classes.iter().map(|c| &c.name)).chain(m.callbacks.iter().map(|c| &c.name)).chain(m.unions.iter().map(|u| &u.name)).chain(&m.opaque).collect();
    let mut types: HashMap<String, String> = RUNTIME_TYPES.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
    // A namespace's type is in a module of it: `Stream.PipeOptions` is
    // `stream::PipeOptions`. Its own name is apart in that module.
    let mut own: HashMap<String, String> = HashMap::new();
    let mut module_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut taken_in: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for n in declared.iter().filter(|n| m.namespaced.contains_key(n.as_str())) {
        let path = &m.namespaced[n.as_str()];
        let segments: Vec<String> = path[..path.len() - 1].iter().map(|s| ns_module(s)).collect();
        let base = rust_type(path.last().unwrap());
        let taken = taken_in.entry(segments.clone()).or_default();
        let mut leaf = base.clone();
        let mut i = 2;
        while !taken.insert(leaf.clone()) {
            leaf = format!("{base}{i}");
            i += 1;
        }
        types.insert(n.to_string(), format!("{}::{leaf}", segments.join("::")));
        own.insert(n.to_string(), leaf);
        module_of.insert(n.to_string(), segments);
    }
    for n in &declared {
        types.entry(n.to_string()).or_insert_with(|| rust_type(n));
    }
    let leaves: HashSet<String> = own.values().cloned().collect();
    CX.with(|c| {
        *c.borrow_mut() = Context {
            decodable: m.unions.iter().filter(|u| u.decodable()).map(|u| u.name.clone()).collect(),
            unions: m.unions.iter().map(|u| u.name.clone()).collect(),
            classes: m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.clone()).chain(m.opaque.iter().filter(|n| !m.opaque_arity.contains_key(*n)).cloned()).collect(),
            types,
            own,
            leaves,
        }
    });
    // What a namespace's module defines, by its path.
    let mut namespaces: BTreeMap<Vec<String>, String> = BTreeMap::new();
    let mut imports: BTreeMap<String, String> = BTreeMap::new();
    let mut types_file = String::from(PRELUDE);

    for (model_name, values) in &m.enums {
        let name = def_name(model_name);
        let mut seen = HashSet::new();
        let variants: Vec<(String, String)> = values
            .iter()
            .map(|v| {
                let id = camel(&super::model::ident(if v.is_empty() { "empty" } else { v }));
                // Nothing of it is a name (`"\\"`, `"/"`).
                let id = if id.trim_matches('_').is_empty() { "Value".to_string() } else { id };
                let id = if seen.insert(id.clone()) { id } else { format!("{id}{}", seen.len()) };
                (id, v.clone())
            })
            .collect();
        let index: String = variants.iter().enumerate().map(|(i, (v, _))| format!("{i} => {name}::{v}, ")).collect();
        let from: String = variants.iter().map(|(v, s)| format!("{s:?} => {name}::{v}, ")).collect();
        let to: String = variants.iter().map(|(v, s)| format!("{name}::{v} => {s:?}, ")).collect();
        let out = match module_of.get(model_name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => &mut types_file,
        };
        *out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             impl {name} {{\n    pub fn from_index(i: i32) -> Self {{ match i {{ {index}_ => panic!(\"{name}: no value {{i}}\") }} }}\n    pub fn as_str(self) -> &'static str {{ match self {{ {to}}} }}\n}}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ self.as_str().lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ let s = String::from_handle(h); match s.as_str() {{ {from}_ => panic!(\"{name}: no value {{s}}\") }} }} }}\n",
            variants.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>().join(", ")
        );
    }
    for (model_name, members) in &m.num_enums {
        let name = def_name(model_name);
        let all: Vec<(String, &String)> = members.iter().map(|(member, value)| (camel(&super::model::ident(member)), value)).collect();
        // A value once a variant; a member of a value given already
        // (`ROTATE = 0` beside `LEFT = 0`) is a constant of that variant.
        let mut variants: Vec<(String, &String)> = Vec::new();
        let mut aliases: Vec<String> = Vec::new();
        for (v, value) in all {
            match variants.iter().find(|(_, w)| *w == value) {
                Some((first, _)) => aliases.push(format!("pub const {v}: {name} = {name}::{first}; ")),
                None => variants.push((v, value)),
            }
        }
        let from: String = variants.iter().map(|(v, value)| format!("{value} => {name}::{v}, ")).collect();
        let out = match module_of.get(model_name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => &mut types_file,
        };
        *out += &format!(
            "#[derive(Clone, Copy, PartialEq, Eq, Debug)]\n#[repr(i32)]\npub enum {name} {{ {} }}\n\
             #[allow(non_upper_case_globals)]\nimpl {name} {{ {}pub fn from_value(v: i32) -> Self {{ match v {{ {from}_ => panic!(\"{name}: no value {{v}}\") }} }} }}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ (*self as i32 as f64).lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ Self::from_value(f64::from_handle(h) as i32) }} }}\n",
            variants.iter().map(|(v, value)| format!("{v} = {value}")).collect::<Vec<_>>().join(", "),
            aliases.concat()
        );
    }
    // Types declared elsewhere: handles, generic or not.
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.iter().any(|r| r.0 == n.as_str())) {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<String> = (1..=arity).map(|i| format!("T{i}")).collect();
        let raw_name = name.as_str();
        let name = def_name(name);
        let targs = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        let phantom = if params.is_empty() { "" } else { ", PhantomData" };
        types_file += &format!("handle!({name}{targs});\n");
        if params.is_empty() {
            types_file += &format!("impl Is<{name}> for {name} {{}}\n");
        }
        if raw_name == "Error" && params.is_empty() {
            types_file += "impl Error {\n\
                           pub fn new(msg: &str) -> Self { Self(JsHandle(unsafe { crate::runtime::rt::js_error_new(msg.as_ptr(), msg.len()) })) }\n\
                           pub fn name(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_name(self.0 .0) }) }\n\
                           pub fn message(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_message(self.0 .0) }) }\n\
                           pub fn stack(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_error_stack(self.0 .0) }) }\n\
                           }\n\
                           impl From<JsError> for Error { fn from(e: JsError) -> Self { Self(e.0) } }\n\
                           impl From<Error> for JsError { fn from(e: Error) -> Self { Self(e.0) } }\n";
        } else if raw_name == "Date" && params.is_empty() {
            types_file += "impl Date {\n\
                           pub fn from_time(ms: f64) -> Self { Self(JsHandle(unsafe { crate::runtime::rt::js_date_new(ms) })) }\n\
                           pub fn now() -> f64 { unsafe { crate::runtime::rt::js_date_now() } }\n\
                           pub fn time(&self) -> f64 { unsafe { crate::runtime::rt::js_date_time(self.0 .0) } }\n\
                           pub fn to_iso_string(&self) -> ::std::string::String { <::std::string::String>::from_handle(unsafe { crate::runtime::rt::js_date_to_iso_string(self.0 .0) }) }\n\
                           }\n";
        } else if let Some(s) = super::model::buffer_spec(raw_name) {
            types_file += &format!(
                "impl{targs} {name}{targs} {{\n\
                 pub fn byte_len(&self) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_len(self.0 .0) as usize }} }}\n\
                 pub fn read_bytes(&self, offset: usize, dst: &mut [u8]) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_read(self.0 .0, offset, dst.as_mut_ptr(), dst.len()) as usize }} }}\n\
                 pub fn write_bytes(&self, offset: usize, src: &[u8]) -> usize {{ unsafe {{ crate::runtime::rt::js_bytes_write(self.0 .0, offset, src.as_ptr(), src.len()) as usize }} }}\n"
            );
            if s.has_buffer() {
                types_file += "pub fn byte_offset(&self) -> usize { unsafe { crate::runtime::rt::js_bytes_offset(self.0 .0) as usize } }\n\
                               pub fn buffer(&self) -> JsValue { JsValue(JsHandle(unsafe { crate::runtime::rt::js_bytes_buffer(self.0 .0) })) }\n";
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = rust_elem(s.elem);
                let eb = s.elem.bytes();
                types_file += &format!(
                    "pub fn new(len: usize) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, core::ptr::null(), len.saturating_mul({eb})) }}){phantom}) }}\n\
                     pub fn from_slice(data: &[{elem}]) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, data.as_ptr() as *const u8, data.len().saturating_mul({eb})) }}){phantom}) }}\n\
                     pub fn len(&self) -> usize {{ self.byte_len() / {eb} }}\n\
                     pub fn is_empty(&self) -> bool {{ self.len() == 0 }}\n\
                     pub fn read(&self, elem_offset: usize, dst: &mut [{elem}]) -> usize {{ let Some(off) = elem_offset.checked_mul({eb}) else {{ return 0; }}; unsafe {{ crate::runtime::rt::js_bytes_read(self.0 .0, off, dst.as_mut_ptr() as *mut u8, dst.len().saturating_mul({eb})) as usize / {eb} }} }}\n\
                     pub fn write(&self, elem_offset: usize, src: &[{elem}]) -> usize {{ let Some(off) = elem_offset.checked_mul({eb}) else {{ return 0; }}; unsafe {{ crate::runtime::rt::js_bytes_write(self.0 .0, off, src.as_ptr() as *const u8, src.len().saturating_mul({eb})) as usize / {eb} }} }}\n\
                     pub fn to_vec(&self) -> ::std::vec::Vec<{elem}> {{ let mut out = vec![Default::default(); self.len()]; self.read(0, &mut out); out }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    types_file += &format!(
                        "pub fn new(byte_len: usize) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, core::ptr::null(), byte_len) }}){phantom}) }}\n\
                         pub fn from_bytes(data: &[u8]) -> Self {{ Self(JsHandle(unsafe {{ crate::runtime::rt::js_bytes_new({kind}, data.as_ptr(), data.len()) }}){phantom}) }}\n"
                    );
                }
                types_file += "pub fn read(&self, offset: usize, dst: &mut [u8]) -> usize { self.read_bytes(offset, dst) }\n\
                               pub fn write(&self, offset: usize, src: &[u8]) -> usize { self.write_bytes(offset, src) }\n\
                               pub fn to_vec(&self) -> ::std::vec::Vec<u8> { let mut out = vec![0u8; self.byte_len()]; self.read_bytes(0, &mut out); out }\n";
            }
            types_file += "}\n";
        }
    }
    // Unions that can be decoded: enums.
    for u in m.unions.iter().filter(|u| u.decodable()) {
        let text = union_enum(u, &mut imports);
        match module_of.get(&u.name) {
            Some(path) => *namespaces.entry(path.clone()).or_default() += &text,
            None => types_file += &text,
        }
    }
    for c in &m.callbacks {
        let text = callback(&c.name, &c.type_params, &c.params, &c.ret, &mut imports);
        match module_of.get(&c.name) {
            Some(path) => *namespaces.entry(path.clone()).or_default() += &text,
            None => types_file += &text,
        }
    }

    // Each class: a struct, a trait of its members, and that trait for each subtype.
    let class_of = |n: &str| m.classes.iter().find(|k| k.name == n);
    let ancestors = |name: &str| -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        let mut todo = vec![name.to_string()];
        while let Some(c) = todo.pop() {
            for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
                if !out.contains(a) {
                    out.push(a.clone());
                    todo.insert(0, a.clone());
                }
            }
        }
        out
    };
    // The Rust name of each class's functions, by the model's.
    let names: HashMap<&str, HashMap<String, String>> = m.classes.iter().map(|c| (c.name.as_str(), names_of(c.funcs.iter()))).collect();
    let type_names: HashSet<String> = declared.iter().map(|n| tn(n)).collect();
    let grouped: HashMap<&str, Vec<Family>> = m.classes.iter().map(|c| (c.name.as_str(), families(c, &names[c.name.as_str()], &type_names))).collect();
    // The family a member is one overload of.
    let family_of = |c: &Class, f: &Func| grouped[c.name.as_str()].iter().find(|g| g.members.iter().any(|o| o.name == f.name));
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| family_of(c, f).map_or_else(|| names[c.name.as_str()][&f.name].clone(), |g| g.name.clone())).collect() };
    let this = "AsRef::<JsHandle>::as_ref(self).0";
    let mut modules: BTreeMap<String, String> = BTreeMap::new();
    for c in &m.classes {
        let out = match module_of.get(&c.name) {
            Some(path) => namespaces.entry(path.clone()).or_default(),
            None => modules.entry(module_file(&c.module)).or_insert_with(|| PRELUDE.to_string()),
        };
        let params: Vec<String> = c.type_params.iter().map(|p| p.name.clone()).collect();
        let (bounds, args) = (generics(&c.type_params), args_of(&c.type_params));
        let name = def_name(&c.name);
        let me = format!("{name}{args}");
        let own = &names[c.name.as_str()];
        *out += &format!("\nhandle!({name}{});\n", if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) });
        if params.is_empty() {
            *out += &format!("impl Is<{name}> for {name} {{}}\n");
        }
        // The trait of its own members.
        let mut trait_body = String::new();
        for f in c.funcs.iter().filter(|f| Class::has_self(f) && family_of(c, f).is_none()) {
            for item in function(&with_self_args(f, c), &own[&f.name], "", &mut imports, Some(this)) {
                trait_body += &format!("    {item}\n");
            }
        }
        let mut dispatch = String::new();
        for g in &grouped[c.name.as_str()] {
            for item in g.methods("", &mut imports, this) {
                trait_body += &format!("    {item}\n");
            }
            dispatch += &g.dispatch(&mut imports);
        }
        if let Some(elem) = &c.iterates {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret;
            trait_body += &format!("    fn iter(&self) -> JsIterator<{}> {{ JsIterator::over({this}) }}\n", ty(&elem));
        }
        if let Some(elem) = &c.iterates_async {
            trait_body += &format!("    fn iterate_async(&self) -> JsAsyncIterator<{}> {{ JsAsyncIterator::over({this}) }}\n", ty(elem));
        }
        *out += &format!("pub trait {name}Methods{bounds}: AsRef<JsHandle> {{\n{trait_body}}}\n");
        *out += &format!("impl{bounds} {name}Methods{args} for {me} {{}}\n{dispatch}");
        // Statics, constants, a map's keys, and where two traits share a name, the one this type means.
        let mut inherent = String::new();
        for f in c.funcs.iter().filter(|f| !Class::has_self(f)) {
            for item in function(&with_self_args(f, c), &own[&f.name], "pub ", &mut imports, None) {
                inherent += &format!("    {item}\n");
            }
        }
        for (constant, value) in &c.consts {
            let t = if value.starts_with('"') { "&'static str" } else if value == "true" || value == "false" { "bool" } else { "f64" };
            // A whole number, decimal or hexadecimal, as an f64.
            let value = if t == "f64" && (value.starts_with("0x") || value.starts_with("0X")) { format!("{value}i64 as f64") } else if t == "f64" && !value.contains('.') && !value.contains('e') { format!("{value}.0") } else { value.clone() };
            inherent += &format!("    pub const {}: {t} = {value};\n", snake(&super::model::ident(constant)).to_uppercase());
        }
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let mut seen = HashSet::new();
            for (i, (key, v)) in map.keys.iter().zip(&map.values).enumerate() {
                let constant = snake(&super::model::ident(key)).to_uppercase();
                let constant = if seen.insert(constant.clone()) { constant } else { format!("{constant}_{i}") };
                inherent += &format!("    pub const {constant}: JsKey<{name}, {}> = JsKey::new({i});\n", ty(v));
            }
        }
        let all = ancestors(&c.name);
        let mut holders: BTreeMap<String, Vec<String>> = BTreeMap::new();
        for h in std::iter::once(c.name.clone()).chain(all.iter().cloned()) {
            if let Some(k) = class_of(&h) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(h.clone());
                }
            }
        }
        for (method, hs) in holders.iter().filter(|(_, hs)| hs.len() > 1) {
            // The nearest holder: the type itself, then its nearest ancestor.
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let args = if *h == c.name { k.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect() } else { base_args(m, c, h).unwrap_or_default() };
            if let Some(g) = grouped[k.name.as_str()].iter().find(|g| g.name == *method) {
                for item in g.methods("pub ", &mut imports, this) {
                    inherent += &format!("    {item}\n");
                }
                continue;
            }
            for f in k.funcs.iter().filter(|f| Class::has_self(f) && names[k.name.as_str()][&f.name] == *method) {
                let f = map_types(&with_self_args(f, k), &|t| subst(t, &k.type_params, &args));
                for item in function(&f, method, "pub ", &mut imports, Some(this)) {
                    inherent += &format!("    {item}\n");
                }
            }
        }
        // A subclass of Array, as the array it is.
        if let Some(elem) = &c.elements {
            inherent += &format!("    pub fn as_array(&self) -> JsArray<{}> {{ JsArray::from_handle(crate::runtime::give(self)) }}\n", ty(elem));
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if c.funcs.iter().any(|f| own[&f.name] == "cast") || holders.get("cast").is_some_and(|hs| hs.len() > 1) { "try_cast" } else { "cast" };
            let is_imp = format!("js_is__{}", c.name);
            imports.entry(is_imp.clone()).or_insert_with(|| format!("    pub fn {is_imp}(h: u32) -> i32;\n"));
            let opt = option();
            inherent += &format!(
                "    pub fn {cast_name}(v: &impl AsRef<JsHandle>) -> {opt}<Self> {{ let h = v.as_ref().0; if h != 0 && unsafe {{ raw::{is_imp}(h) }} != 0 {{ let c = v.as_ref().clone(); core::mem::forget(c); Some(<Self>::from_handle(h)) }} else {{ None }} }}\n"
            );
        }
        if !inherent.is_empty() {
            *out += &format!("impl{bounds} {me} {{\n{inherent}}}\n");
        }
        // The traits of its ancestors, and conversions to them.
        for a in &all {
            let Some(k) = class_of(a) else {
                if !m.opaque_arity.contains_key(a) && !RUNTIME_TYPES.iter().any(|r| r.0 == a.as_str()) {
                    *out += &upcast_in(&bounds, &me, &tn(a));
                }
                continue;
            };
            let base: Vec<Ty> = base_args(m, c, a).unwrap_or_default();
            let base_args = if k.type_params.is_empty() { String::new() } else { format!("<{}>", (0..k.type_params.len()).map(|i| base.get(i).map(ty).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", ")) };
            *out += &format!("impl{bounds} {}Methods{base_args} for {me} {{}}\n", tn(a));
            // Generic or not: `Marker<P>` is a `Layer`.
            *out += &upcast_in(&bounds, &me, &format!("{}{base_args}", tn(a)));
        }
    }
    let function_names = names_of(m.funcs.iter());
    // A namespace's function is in its module, by its own name:
    // `stream::finished`, `intl::date_time_format`.
    let mut taken_in: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for f in &m.funcs {
        let (out, name) = match m.func_namespaces.get(&f.import) {
            Some((path, own, _, _)) => {
                let segments: Vec<String> = path.iter().map(|s| ns_module(s)).collect();
                let base = rust_function(f, own);
                let taken = taken_in.entry(segments.clone()).or_default();
                let mut name = base.clone();
                let mut i = 2;
                while !taken.insert(name.clone()) {
                    name = format!("{base}_{i}");
                    i += 1;
                }
                (namespaces.entry(segments).or_default(), name)
            }
            None => (modules.entry(module_file(&f.module)).or_insert_with(|| PRELUDE.to_string()), function_names[&f.name].clone()),
        };
        for item in function(f, &name, "pub ", &mut imports, None) {
            *out += &format!("{item}\n");
        }
    }

    let mut lib = String::from(
        "// Generated. Bindings: `use bindings::*;`.\n#![allow(non_camel_case_types, non_snake_case, clippy::all)]\n\nmod raw;\nmod runtime;\npub use runtime::*;\nmod types;\npub use types::*;\n",
    );
    let mut out = vec![("Cargo.toml".to_string(), "# Generated.\n[package]\nname = \"bindings\"\nversion = \"0.1.0\"\nedition = \"2024\"\n".to_string()), ("src/runtime.rs".to_string(), RUNTIME.to_string()), ("src/types.rs".to_string(), types_file)];
    // A namespace's module, by its own name (`pub mod stream;`), not taken
    // into the crate's: its types are `stream::PipeOptions`. Its members'
    // traits are in scope with the crate's, under no name.
    let tops: BTreeSet<&String> = namespaces.keys().filter_map(|k| k.first()).collect();
    for top in tops {
        let mut text = PRELUDE.to_string();
        fn nest(at: &[String], namespaces: &BTreeMap<Vec<String>, String>, depth: usize, text: &mut String) {
            *text += namespaces.get(at).map(String::as_str).unwrap_or_default();
            let children: BTreeSet<&String> = namespaces.keys().filter(|k| k.len() > at.len() && k[..at.len()] == *at).map(|k| &k[at.len()]).collect();
            for child in children {
                let mut path = at.to_vec();
                path.push(child.clone());
                *text += &format!("\npub mod {child} {{\n{}", PRELUDE.replace("// Generated.\n#![allow(unused_imports)]\n", "#[allow(unused_imports)]\nuse crate::*;\n"));
                nest(&path, namespaces, depth + 1, text);
                *text += "}\n";
            }
        }
        nest(std::slice::from_ref(top), &namespaces, 0, &mut text);
        lib += &format!("pub mod {top};\n");
        out.push((format!("src/{top}.rs"), text));
    }
    for c in m.classes.iter().filter(|c| module_of.contains_key(&c.name)) {
        lib += &format!("#[allow(unused_imports)]\npub use {}Methods as _;\n", tn(&c.name));
    }
    for (module, text) in modules {
        // A module named as one of the crate's own files gets another name.
        let module = if matches!(module.as_str(), "raw" | "runtime" | "types" | "lib") { format!("{module}_") } else { module };
        lib += &format!("mod {module};\npub use {module}::*;\n");
        out.push((format!("src/{module}.rs"), text));
    }
    out.push(("src/raw.rs".into(), format!("// Generated. The imports.\n#![allow(dead_code, unused_imports)]\nuse core::ffi::c_void;\n\n#[link(wasm_import_module = \"env\")]\nunsafe extern \"C\" {{\n{}}}\n", imports.into_values().collect::<String>())));
    out.push(("src/lib.rs".into(), lib));
    Ok(out)
}

// A subtype as its ancestor: by value with `into()`, by reference with
// `as_ref()`. Both are one handle, transparently; for a class or base of
// type parameters too (`impl<P: JsType>`: `Marker<P>` is a `Layer`).
fn upcast_in(bounds: &str, sub: &str, base: &str) -> String {
    let name = base.split('<').next().unwrap_or(base);
    let make = if base.contains('<') { format!("{name}(x.0, core::marker::PhantomData)") } else { format!("{name}(x.0)") };
    format!(
        "impl{bounds} From<{sub}> for {base} {{ fn from(x: {sub}) -> Self {{ {make} }} }}\n\
         impl{bounds} AsRef<{base}> for {sub} {{ fn as_ref(&self) -> &{base} {{ unsafe {{ &*(self as *const {sub} as *const {base}) }} }} }}\n\
         impl{bounds} Is<{base}> for {sub} {{}}\n"
    )
}

// A union as an enum: decoded from a handle by its tag, lent as its member,
// and made from any member: `describe(42.0)`.
fn union_enum(u: &NamedUnion, imports: &mut BTreeMap<String, String>) -> String {
    let n = def_name(&u.name);
    let opt = option();
    let import = &u.name;
    let (bounds, args) = (generics(&u.type_params), args_of(&u.type_params));
    // Each member's variant: by its type's own name, by its whole path
    // where two would be one (`stream::Readable` beside a `Readable`).
    let names: Vec<String> = u.parts.iter().map(variant).collect();
    let names: Vec<String> = u
        .parts
        .iter()
        .zip(&names)
        .map(|(p, v)| match p {
            Ty::Obj(n) | Ty::Generic(n, _) | Ty::Enum(n) if names.iter().filter(|w| *w == v).count() > 1 => tn(n).trim_start_matches("crate::").split("::").map(camel).collect(),
            _ => v.clone(),
        })
        .collect();
    let variant = |p: &Ty| names[u.parts.iter().position(|q| q == p).unwrap()].clone();
    let variants: Vec<String> = u.parts.iter().map(|p| format!("{}({})", variant(p), ty(p))).collect();
    // A bare type parameter is told apart by what its argument is.
    let kind = u.parts.iter().find_map(|p| if let Ty::Param(t) = p { Some(format!("{t}::KIND")) } else { None }).unwrap_or("0".into());
    let decode: String = u.parts.iter().enumerate().map(|(i, p)| format!("{i} => {n}::{}(<{}>::from_handle(h)), ", variant(p), ty(p))).collect();
    let lend: String = u.parts.iter().map(|p| format!("{n}::{}(x) => x.lend(), ", variant(p))).collect();
    let clone: String = u.parts.iter().map(|p| format!("{n}::{0}(x) => {n}::{0}(Clone::clone(x)), ", variant(p))).collect();
    imports.insert(format!("{import}_tag"), format!("    pub fn {import}_tag(h: u32, k: i32) -> i32;\n"));
    // A union is what each of its members is.
    let is_bounds: Vec<String> = u.parts.iter().map(|p| format!("{}: Is<B_>", ty(p))).collect();
    let is_params = std::iter::once("B_".to_string()).chain(u.type_params.iter().map(bound)).collect::<Vec<_>>().join(", ");
    let mut out = format!(
        "pub enum {n}{bounds} {{ {} }}\n\
         impl{bounds} Clone for {n}{args} {{ fn clone(&self) -> Self {{ match self {{ {clone}}} }} }}\n\
         impl{bounds} ToJs for {n}{args} {{ fn lend(&self) -> Lent {{ match self {{ {lend}}} }} }}\n\
         impl{bounds} FromJs for {n}{args} {{ fn from_handle(h: u32) -> Self {{ match unsafe {{ raw::{import}_tag(h, {kind}) }} {{ {decode}_ => unreachable!() }} }} }}\n\
         impl<{is_params}> Is<B_> for {n}{args} where {} {{}}\n\
         impl{bounds} Optional<{n}{args}> for {opt}<{n}{args}> {{ fn into_option(self) -> {opt}<{n}{args}> {{ self }} }}\n",
        variants.join(", "),
        is_bounds.join(", ")
    );
    // Made from a member. A bare type parameter cannot be: it could be the union itself.
    for p in u.parts.iter().filter(|p| !matches!(p, Ty::Param(_))) {
        let (v, t) = (variant(p), ty(p));
        out += &format!("impl{bounds} From<{t}> for {n}{args} {{ fn from(x: {t}) -> Self {{ {n}::{v}(x) }} }}\n");
        match p {
            Ty::Str => out += &format!("impl{bounds} From<&str> for {n}{args} {{ fn from(x: &str) -> Self {{ {n}::{v}(x.to_string()) }} }}\n"),
            Ty::Num | Ty::Bool | Ty::Enum(_) => {}
            _ => out += &format!("impl{bounds} From<&{t}> for {n}{args} {{ fn from(x: &{t}) -> Self {{ {n}::{v}(Clone::clone(x)) }} }}\n"),
        }
    }
    out
}

// A callback type, made from a closure: `CounterVisitF::new(|c: Counter| ..)`,
// or for the length of a call, `CounterVisitF::scoped(|c| .., |f| counter.visit(f))`.
fn callback(name: &str, type_params: &[TParam], params: &[(String, Ty)], ret: &Ty, imports: &mut BTreeMap<String, String>) -> String {
    let import = name;
    let name = def_name(name);
    let (bounds, args) = (generics(type_params), args_of(type_params));
    let tps: Vec<String> = type_params.iter().map(|p| p.name.clone()).collect();
    let arg_types: Vec<String> = params.iter().map(|(_, t)| ty(t)).collect();
    let fn_bound = if *ret == Ty::Void { format!("FnMut({})", arg_types.join(", ")) } else { format!("FnMut({}) -> {}", arg_types.join(", "), ty(ret)) };
    let raw_args: String = params.iter().enumerate().map(|(i, (_, t))| format!(", a{i}: {}", raw_ret(t))).collect();
    let call_args: Vec<String> = params.iter().enumerate().map(|(i, (_, t))| raise(t, &format!("a{i}"))).collect();
    let raw_result = if *ret == Ty::Void { String::new() } else { format!(" -> {}", raw_ret(ret)) };
    // The same trampoline twice: for a closure that is kept, and for one that is borrowed.
    let trampoline = |f: &str, lifetime: &str| {
        let mut generic: Vec<String> = type_params.iter().map(bound).collect();
        generic.push(format!("F: {fn_bound}{lifetime}"));
        format!("extern \"C\" fn call<{}>(ctx: *mut core::ffi::c_void{raw_args}){raw_result} {{ {} }}", generic.join(", "), lower_callback(ret, &format!("({f})({})", call_args.join(", "))))
    };
    let turbofish: Vec<String> = tps.iter().cloned().chain(std::iter::once("F".to_string())).collect();
    imports.insert(format!("{import}_from"), format!("    pub fn {import}_from(f: usize, ctx: *mut c_void) -> u32;\n"));
    let handle_params = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
    format!(
        "handle!({name}{handle_params});\nimpl{bounds} JsCallback for {name}{args} {{}}\n\
         impl{bounds} {name}{args} {{\n    \
         pub fn new<F: {fn_bound} + 'static>(f: F) -> Self {{\n        {}\n        \
         Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, crate::runtime::closure(f)) }})\n    }}\n    \
         // For the length of `body`: its closure may borrow.\n    \
         pub fn scoped<F: {fn_bound}, R_>(mut f: F, body: impl FnOnce(&Self) -> R_) -> R_ {{\n        {}\n        \
         let made = Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, &mut f as *mut F as *mut core::ffi::c_void) }});\n        \
         let _forget = crate::runtime::Forget(made.0 .0);\n        body(&made)\n    }}\n}}\n",
        trampoline("unsafe { crate::runtime::closure_fn::<F>(ctx) }", " + 'static"),
        trampoline("unsafe { &mut *(ctx as *mut F) }", ""),
        turbofish.join(", ")
    )
}

// A buffer element as a Rust type.
fn rust_elem(e: super::model::Elem) -> &'static str {
    use super::model::Elem::*;
    match e {
        I8 => "i8",
        U8 => "u8",
        I16 => "i16",
        U16 => "u16",
        I32 => "i32",
        U32 => "u32",
        I64 => "i64",
        U64 => "u64",
        F32 => "f32",
        F64 => "f64",
    }
}

// A namespace's module name: `Stream` is `stream`, `NodeJS` `node_js`; not a
// file of the crate's own.
fn ns_module(name: &str) -> String {
    let m = ident(&snake(name));
    if matches!(m.as_str(), "raw" | "runtime" | "types" | "lib") { format!("{m}_") } else { m }
}
