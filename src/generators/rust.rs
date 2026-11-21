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
use std::path::Path;

use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, mentions, subst, with_self_args};
use super::names::{Kind, Naming, identifiers};

#[derive(Clone, Debug)]
pub struct ProjectRef {
    pub crate_name: String,
    pub dep_path: String,
    /// Exported top-level types: Rust name -> `(arity, is_handle)`.
    pub types: HashMap<String, (usize, bool)>,
}

/// Loads a previously generated Rust binding crate (via `tsrs-meta.toml`)
/// so another generated crate can reference its types directly instead of
/// emitting duplicate opaque handles.
pub fn load_project_ref(crate_or_toml: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(crate_or_toml, from_dir, "rust")?;
    Ok(ProjectRef {
        crate_name: loaded.meta.package,
        dep_path: loaded.rel_dir,
        types: loaded.meta.rust_types.into_iter().collect(),
    })
}

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
    super::case::snake_case(name)
}

// `Options_mode` -> `OptionsMode`, `fill` -> `Fill`: for generated type names and variants.
fn camel(name: &str) -> String {
    let out = super::case::pascal_underscore(name);
    if out.chars().next().is_none_or(|c| c.is_ascii_digit()) {
        format!("_{out}")
    } else if KEYWORDS.contains(&out.as_str()) {
        // `"self"` as a variant is `Self_`: `Self` is Rust's.
        format!("{out}_")
    } else {
        out
    }
}

fn camel_variant(name: &str) -> String {
    super::case::screaming_to_pascal(name, camel)
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
        Ty::Generic(n, _) if !is_union(n) => (format!("&impl AsRef<{}>", ty(t)), vec![format!("AsRef::<{}>::as_ref({x}).lend().h", ty(t))]),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => into(ty(t)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n) => into(ty(t)),
        Ty::Any | Ty::Union(_) | Ty::NamedUnion(_) => into(VALUE.into()),
        // A value of a type parameter is passed as into a collection: by value.
        Ty::Param(_) => (ty(t), lend(x)),
        _ => (format!("&{}", ty(t)), lend(x)),
    }
}

// What a value crosses the boundary as, going in.
fn raw(t: &Ty) -> Vec<&'static str> {
    use super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::wasm::param_abi(t) {
        None => vec![],
        One(F64) => vec!["f64"],
        One(I32) => vec!["i32"],
        One(U32 | Void) => vec!["u32"],
        Str => vec!["*const u8", "usize"],
        OptNum => vec!["i32", "f64"],
    }
}

// What a value crosses as coming back, and what a callback's arguments and result cross as.
fn raw_ret(t: &Ty) -> &'static str {
    use super::wasm::AbiSlot::*;
    match super::wasm::ret_abi(t) {
        Void => "()",
        F64 => "f64",
        I32 => "i32",
        U32 => "u32",
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

// A function's name: a property `x` reads as `x()` and writes as `set_x(..)`.
// A `$` method (`$add`) reads as `add_new()` rather than `_add()`.
fn function_name_as(f: &Func, name: &str) -> String {
    let base = match &f.call {
        Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_) | Call::IndexGet => name.strip_prefix("get_").unwrap_or(name),
        _ => name,
    };
    let dollar = super::case::has_dollar_js_name(&f.call);
    let s = snake(base);
    if !dollar {
        return ident(&s);
    }
    let is_set = matches!(&f.call, Call::Set(_) | Call::StaticSet(_));
    let is_prop = matches!(&f.call, Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::StaticGet(_) | Call::StaticSet(_));
    let stem = if is_set { s.strip_prefix("set_").unwrap_or(&s) } else { &s }.trim_start_matches('_');
    let stem = if stem.is_empty() { "dollar" } else { stem };
    ident(&if is_prop {
        if is_set { format!("set_dollar_{stem}") } else { format!("dollar_{stem}") }
    } else if is_set {
        format!("set_{stem}_new")
    } else {
        format!("{stem}_new")
    })
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
    function_raw(f, f, name, vis, imports, this)
}

fn function_raw(f: &Func, orig: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> Vec<String> {
    if hidden(f).is_empty() {
        return vec![one_function_raw(f, Some(orig), name, vis, imports, this)];
    }
    vec![
        one_function_raw(&defaulted(f), Some(&defaulted(orig)), name, vis, imports, this),
        one_function_raw(f, Some(orig), &format!("{name}_as"), vis, imports, this),
    ]
}

fn one_function(f: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> String {
    one_function_raw(f, None, name, vis, imports, this)
}

fn one_function_raw(f: &Func, orig: Option<&Func>, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> String {
    let p = parts_with_raw(f, orig, imports, this);
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
    parts_with_raw(f, None, imports, this)
}

fn parts_with_raw(f: &Func, orig: Option<&Func>, imports: &mut BTreeMap<String, String>, this: Option<&str>) -> Parts {
    let orig = orig.unwrap_or(f);
    let mut raw_params: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(this) = this {
        raw_params.push("this: u32".into());
        args.push(this.into());
    }
    let mut params: Vec<String> = if this.is_some() { vec!["&self".into()] } else { Vec::new() };
    for (i, ((p, t), (_, orig_t))) in f.params.iter().zip(&orig.params).enumerate() {
        let p = ident(&snake(p));
        let (written, lowered) = if raw(orig_t) == raw(t) {
            param(t, &p)
        } else {
            let w = if *t == Ty::Void { "()".into() } else { param(t, &p).0 };
            (w, vec![format!("{p}.lend().h")])
        };
        params.push(format!("{p}: {written}"));
        raw_params.extend(raw(orig_t).into_iter().enumerate().map(|(j, r)| format!("a{i}_{j}: {r}")));
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
        format!("    {link}pub fn {raw_name}({}) -> {};\n", raw_params.join(", "), raw_ret(&orig.ret))
    });
    let import = raw_name;
    let mut type_params: Vec<String> = f.type_params.iter().map(bound).collect();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        type_params.insert(0, "V: JsType".into());
    }
    let body = if raw_ret(&orig.ret) == raw_ret(&f.ret) {
        match &f.ret {
            Ty::Void => format!("unsafe {{ raw::{import}({}) }};", args.join(", ")),
            t => format!("let r = unsafe {{ raw::{import}({}) }}; {}", args.join(", "), raise(t, "r")),
        }
    } else {
        let semi = if f.ret == Ty::Void { ";" } else { "" };
        format!("let r = unsafe {{ raw::{import}({}) }}; <{}>::from_handle(r){semi}", args.join(", "), ty(&f.ret))
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
    fn methods(&self, vis: &str, imports: &mut BTreeMap<String, String>, this: &str, ret_self: Option<&Ty>) -> Vec<String> {
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
        let call = format!("A_::call({}, {this}{rest}{same})", names[self.at]);
        let (ret_s, body_s) = match ret_self {
            Some(_) => (
                "Self".to_string(),
                format!("let out_ = {call}; let h_ = AsRef::<JsHandle>::as_ref(&out_).0; core::mem::forget(out_); <Self>::from_handle(h_)"),
            ),
            None => ("A_::Output".to_string(), call),
        };
        let mut out = vec![format!("{vis}fn {}<A_: {}{}{generics}>(&self, {}) -> {ret_s} {{ {body_s} }}", self.name, self.trait_name, self.rest(), params.join(", "))];
        if let Some(f) = self.members.iter().find(|f| f.params[self.at].1 == Ty::Str && !hidden(f).is_empty()) {
            let mut f = f.clone();
            if let Some(sty) = ret_self {
                f.ret = sty.clone();
            }
            out.push(one_function(&f, &format!("{}_as", self.name), vis, imports, Some(this)));
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

const RUNTIME: &str = include_str!("runtime/rust_runtime.rs");

// What the model names that the runtime declares under another name.
const RUNTIME_TYPES: &[(&str, &str)] = &[("Promise", "JsPromise"), ("PromiseLike", "JsPromise")];

// A model type's Rust name: CamelCase (a type named in snake_case would
// share its constructor's name with a function), or the runtime's.
fn rust_type(n: &str) -> String {
    // A bundler's `SeriesOption$1` is `SeriesOption1`.
    RUNTIME_TYPES.iter().find(|r| r.0 == n).map_or_else(|| camel(&ident(n)), |r| r.1.to_string())
}

pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "Rust",
    counted: true,
    generics: true,
    overloading: false,
    retain_opaque_constraints: false,
};

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
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, crate_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let crate_name = if crate_name.is_empty() { "bindings" } else { crate_name };
    let wasm_module = if crate_name == "bindings" { "env" } else { crate_name };
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
    let mut resolved_opaque: HashMap<String, (usize, String, bool)> = HashMap::new();
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.iter().any(|r| r.0 == n.as_str()) && *n != "Error" && *n != "Date" && super::model::buffer_spec(n).is_none()) {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        if arity == 0 && !m.namespaced.contains_key(name.as_str()) {
            let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
            let rn_orig = rust_type(orig);
            for (idx, r) in refs.iter().enumerate() {
                if let Some(&(0, is_handle)) = r.types.get(&rn_orig) {
                    resolved_opaque.insert(name.clone(), (idx, format!("::{}::{rn_orig}", r.crate_name), is_handle));
                    break;
                }
            }
        }
    }
    let leaves: HashSet<String> = own.values().cloned().collect();
    CX.with(|c| {
        *c.borrow_mut() = Context {
            decodable: m.unions.iter().filter(|u| u.decodable()).map(|u| u.name.clone()).collect(),
            unions: m.unions.iter().map(|u| u.name.clone()).collect(),
            classes: m.classes.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.clone()).chain(m.opaque.iter().filter(|n| !m.opaque_arity.contains_key(*n)).filter(|n| resolved_opaque.get(*n).is_none_or(|x| x.2)).cloned()).collect(),
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
                let mut id = if id.trim_matches('_').is_empty() { "Value".to_string() } else { id };
                if !seen.insert(id.clone()) {
                    let mut i = 2;
                    while !seen.insert(format!("{id}{i}")) {
                        i += 1;
                    }
                    id = format!("{id}{i}");
                }
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
             impl AsRef<{name}> for {name} {{ fn as_ref(&self) -> &Self {{ self }} }}\n\
             impl {name} {{\n    pub fn from_index(i: i32) -> Self {{ match i {{ {index}_ => panic!(\"{name}: no value {{i}}\") }} }}\n    pub fn as_str(self) -> &'static str {{ match self {{ {to}}} }}\n}}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ self.as_str().lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ let s = String::from_handle(h); match s.as_str() {{ {from}_ => panic!(\"{name}: no value {{s}}\") }} }} }}\n",
            variants.iter().map(|(v, _)| v.as_str()).collect::<Vec<_>>().join(", ")
        );
    }
    for (model_name, members) in &m.num_enums {
        let name = def_name(model_name);
        let mut seen = HashSet::new();
        let all: Vec<(String, &String)> = members
            .iter()
            .map(|(member, value)| {
                let mut id = camel_variant(&super::model::ident(member));
                if !seen.insert(id.clone()) {
                    let mut i = 2;
                    while !seen.insert(format!("{id}{i}")) {
                        i += 1;
                    }
                    id = format!("{id}{i}");
                }
                (id, value)
            })
            .collect();
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
             impl AsRef<{name}> for {name} {{ fn as_ref(&self) -> &Self {{ self }} }}\n\
             #[allow(non_upper_case_globals)]\nimpl {name} {{ {}pub fn from_value(v: i32) -> Self {{ match v {{ {from}_ => panic!(\"{name}: no value {{v}}\") }} }} }}\n\
             impl ToJs for {name} {{ fn lend(&self) -> Lent {{ (*self as i32 as f64).lend() }} }}\n\
             impl FromJs for {name} {{ fn from_handle(h: u32) -> Self {{ Self::from_value(f64::from_handle(h) as i32) }} }}\n",
            variants.iter().map(|(v, value)| format!("{v} = {value}")).collect::<Vec<_>>().join(", "),
            aliases.concat()
        );
    }
    let mut used_refs: BTreeSet<usize> = BTreeSet::new();
    let mut impl_refs: HashSet<String> = HashSet::new();
    // Types declared elsewhere: resolve against referenced crates where possible, else opaque handles.
    for name in m.opaque.iter().filter(|n| !RUNTIME_TYPES.iter().any(|r| r.0 == n.as_str())) {
        if let Some((ref_idx, qualified, is_handle)) = resolved_opaque.get(name) {
            used_refs.insert(*ref_idx);
            let ref_crate = &refs[*ref_idx].crate_name;
            let short = qualified.trim_start_matches("::");
            let local_name = def_name(name);
            let reexport = if short == format!("{ref_crate}::{local_name}") {
                format!("pub use {qualified};\n")
            } else {
                format!("pub use {qualified} as {local_name};\n")
            };
            types_file += &reexport;
            if impl_refs.insert(qualified.clone()) {
                types_file += &format!(
                    "impl crate::runtime::ToJs for {qualified} {{\n\
                     \x20   fn lend(&self) -> crate::runtime::Lent {{ let (h, owned) = ::{ref_crate}::ToJs::lend(self).into_raw(); crate::runtime::Lent::from_raw(h, owned) }}\n\
                     }}\n\
                     impl crate::runtime::FromJs for {qualified} {{\n\
                     \x20   fn from_handle(h: u32) -> Self {{ <{qualified} as ::{ref_crate}::FromJs>::from_handle(h) }}\n\
                     }}\n\
                     impl From<{qualified}> for crate::runtime::JsValue {{ fn from(x: {qualified}) -> Self {{ Self(crate::runtime::JsHandle(crate::runtime::give(&x))) }} }}\n\
                     impl From<&{qualified}> for crate::runtime::JsValue {{ fn from(x: &{qualified}) -> Self {{ Self(crate::runtime::JsHandle(crate::runtime::give(x))) }} }}\n"
                );
                if *is_handle {
                    types_file += &format!("impl crate::runtime::Is<{qualified}> for {qualified} {{}}\n");
                }
            }
            continue;
        }
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<String> = (1..=arity).map(|i| format!("T{i}")).collect();
        let raw_name = name.as_str();
        let name = def_name(name);
        let targs = if params.is_empty() { String::new() } else { format!("<{}>", params.join(", ")) };
        let phantom = if params.is_empty() { "" } else { ", PhantomData" };
        if raw_name != "Error" && raw_name != "Date" && super::model::buffer_spec(raw_name).is_none() {
            types_file += "// external\n";
        }
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
        let text = union_enum(u, m, &mut imports);
        match module_of.get(&u.name) {
            Some(path) => *namespaces.entry(path.clone()).or_default() += &text,
            None => types_file += &text,
        }
    }
    for c in &m.callbacks {
        let apply = m.funcs.iter().find(|f| {
            f.call == Call::Apply
                && f.params.first().is_some_and(|(_, t)| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if n == &c.name))
        });
        let text = callback(&c.name, &c.type_params, &c.params, &c.ret, apply, &mut imports);
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
        let (bounds, args) = (generics(&c.type_params), args_of(&c.type_params));
        let name = def_name(&c.name);
        let me = format!("{name}{args}");
        let own = &names[c.name.as_str()];
        *out += &format!("\n{}\n", handle_decl(&c.name, &name, &c.type_params));
        *out += &format!("impl{bounds} Is<{me}> for {me} {{}}\n");
        if let Some(f) = c.funcs.iter().find(|f| matches!(f.call, Call::Create(_)) && f.params.is_empty() && f.type_params.is_empty()) {
            *out += &format!("impl{bounds} ::core::default::Default for {me} {{ fn default() -> Self {{ Self::{}() }} }}\n", own[&f.name]);
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
            for item in g.methods("", &mut imports, this, None) {
                trait_body += &format!("    {item}\n");
            }
            dispatch += &g.dispatch(&mut imports);
        }
        if let Some(elem) = &c.iterates {
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false }, c).ret;
            trait_body += &format!("    fn iter(&self) -> JsIterator<{}> {{ JsIterator::over({this}) }}\n", ty(&elem));
        }
        if let Some(elem) = &c.iterates_async {
            trait_body += &format!("    fn iterate_async(&self) -> JsAsyncIterator<{}> {{ JsAsyncIterator::over({this}) }}\n", ty(elem));
        }
        *out += &format!("pub trait {name}Methods{bounds}: AsRef<JsHandle> {{\n{trait_body}}}\n");
        *out += &format!("impl{bounds} {name}Methods{args} for {me} {{}}\n{dispatch}");
        // Statics, constants, a map's keys, inherent methods (own + inherited), fluent builders, and conversions.
        let mut inherent = String::new();
        let mut inherent_taken: HashSet<String> = HashSet::new();
        for f in c.funcs.iter().filter(|f| !Class::has_self(f)) {
            let fname = &own[&f.name];
            let f_self = with_self_args(f, c);
            if !hidden(&f_self).is_empty() {
                inherent_taken.insert(format!("{fname}_as"));
            }
            inherent_taken.insert(fname.clone());
            for item in function(&f_self, fname, "pub ", &mut imports, None) {
                inherent += &format!("    {item}\n");
            }
        }
        for (constant, value) in &c.consts {
            let t = if value.starts_with('"') { "&'static str" } else if value == "true" || value == "false" { "bool" } else { "f64" };
            // A whole number, decimal or hexadecimal, as an f64.
            let value = if t == "f64" && (value.starts_with("0x") || value.starts_with("0X")) { format!("{value}i64 as f64") } else if t == "f64" && !value.contains('.') && !value.contains('e') { format!("{value}.0") } else { value.clone() };
            let cname = snake(&super::model::ident(constant)).to_uppercase();
            inherent_taken.insert(cname.clone());
            inherent += &format!("    pub const {cname}: {t} = {value};\n");
        }
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let mut seen = HashSet::new();
            for (i, (key, v)) in map.keys.iter().zip(&map.values).enumerate() {
                let constant = snake(&super::model::ident(key)).to_uppercase();
                let constant = if seen.insert(constant.clone()) { constant } else { format!("{constant}_{i}") };
                inherent_taken.insert(constant.clone());
                inherent += &format!("    pub const {constant}: JsKey<{name}, {}> = JsKey::new({i});\n", ty(v));
            }
        }
        // A subclass of Array, as the array it is.
        if let Some(elem) = &c.elements {
            inherent_taken.insert("as_array".into());
            inherent += &format!("    pub fn as_array(&self) -> JsArray<{}> {{ JsArray::from_handle(crate::runtime::give(self)) }}\n", ty(elem));
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
        let args_for = |h: &str, k: &Class| -> Vec<Ty> {
            if h == c.name {
                k.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect()
            } else {
                let b = base_args(m, c, h).unwrap_or_default();
                (0..k.type_params.len()).map(|i| b.get(i).cloned().unwrap_or(Ty::Any)).collect()
            }
        };
        let self_ty = if c.type_params.is_empty() {
            Ty::Obj(c.name.clone())
        } else {
            Ty::Generic(c.name.clone(), c.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect())
        };
        for (method, hs) in &holders {
            if inherent_taken.contains(method) {
                continue;
            }
            // The nearest holder: the type itself, then its nearest ancestor.
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let h_args = args_for(h, k);
            if let Some(g) = grouped[k.name.as_str()].iter().find(|g| g.name == *method) {
                inherent_taken.insert(method.clone());
                if g.members.iter().any(|f| f.params[g.at].1 == Ty::Str && !hidden(f).is_empty()) {
                    inherent_taken.insert(format!("{method}_as"));
                }
                let ret_self = (h != &c.name && g.members.iter().all(|f| f.returns_this)).then_some(&self_ty);
                for item in g.methods("pub ", &mut imports, this, ret_self) {
                    inherent += &format!("    {item}\n");
                }
                continue;
            }
            for f0 in k.funcs.iter().filter(|f| Class::has_self(f) && names[k.name.as_str()][&f.name] == *method) {
                let mut orig = with_self_args(f0, k);
                if !c.type_params.is_empty() && !orig.type_params.is_empty() {
                    orig = avoid_outer_type_params(&orig, &c.type_params, &k.type_params);
                }
                let mut f = map_types(&orig, &|t| subst(t, &k.type_params, &h_args));
                if f.returns_this && h != &c.name {
                    f.ret = self_ty.clone();
                }
                inherent_taken.insert(method.clone());
                let as_name = format!("{method}_as");
                if !hidden(&f).is_empty() && !inherent_taken.contains(&as_name) && !holders.contains_key(&as_name) {
                    inherent_taken.insert(as_name);
                    for item in function_raw(&f, &orig, method, "pub ", &mut imports, Some(this)) {
                        inherent += &format!("    {item}\n");
                    }
                } else {
                    let f_def = if hidden(&f).is_empty() { f } else { defaulted(&f) };
                    let o_def = if hidden(&orig).is_empty() { orig } else { defaulted(&orig) };
                    inherent += &format!("    {}\n", one_function_raw(&f_def, Some(&o_def), method, "pub ", &mut imports, Some(this)));
                }
            }
        }
        // Fluent `with_<prop>(self, v) -> Self` builders for single-arg property setters (own + inherited).
        for (method, hs) in &holders {
            let Some(stem) = method.strip_prefix("set_") else { continue };
            if stem.is_empty() {
                continue;
            }
            let with_name = ident(&format!("with_{stem}"));
            if inherent_taken.contains(&with_name) || holders.contains_key(&with_name) || TAKEN.contains(&with_name.as_str()) {
                continue;
            }
            let h = &hs[0];
            let k = class_of(h).unwrap();
            let matching: Vec<&Func> = k.funcs.iter().filter(|f| Class::has_self(f) && names[k.name.as_str()][&f.name] == *method).collect();
            if matching.len() != 1 {
                continue;
            }
            let f0 = matching[0];
            if !matches!(f0.call, Call::Set(_)) || f0.params.len() != 1 || !f0.type_params.is_empty() || f0.ret != Ty::Void || !f0.fixed.is_empty() {
                continue;
            }
            let h_args = args_for(h, k);
            let orig = with_self_args(f0, k);
            let f = map_types(&orig, &|t| subst(t, &k.type_params, &h_args));
            let orig_t = &orig.params[0].1;
            let (p_raw, t) = &f.params[0];
            let p = ident(&snake(p_raw));
            let (written, val_raw_args) = match t {
                Ty::Nullable(inner, _) => {
                    if matches!(**inner, Ty::Void | Ty::Key(_) | Ty::Mapped(_) | Ty::Nullable(..)) {
                        continue;
                    }
                    let (w, inner_args) = param(inner, &p);
                    let ra = if raw(orig_t) != raw(t) {
                        vec![format!("{p}.lend().h")]
                    } else {
                        match &**inner {
                            Ty::Num => vec!["1".into(), p.clone()],
                            Ty::NumEnum(_) => vec!["1".into(), format!("{p} as i32 as f64")],
                            Ty::Bool | Ty::Enum(_) => vec![format!("{p} as i32")],
                            _ => inner_args,
                        }
                    };
                    (w, ra)
                }
                _ => {
                    if matches!(t, Ty::Void | Ty::Key(_) | Ty::Mapped(_)) {
                        continue;
                    }
                    let (w, mut ra) = param(t, &p);
                    if raw(orig_t) != raw(t) {
                        ra = vec![format!("{p}.lend().h")];
                    }
                    (w, ra)
                }
            };
            let (import, mut raw_params, mut call_args) = match &f.shared {
                Some((shared, k_idx)) => (shared.clone(), vec!["this: u32".into(), "k: i32".into()], vec!["self.0 .0".into(), k_idx.to_string()]),
                None => (f.import.clone(), vec!["this: u32".into()], vec!["self.0 .0".into()]),
            };
            call_args.extend(val_raw_args);
            raw_params.extend(raw(orig_t).into_iter().enumerate().map(|(j, r)| format!("a0_{j}: {r}")));
            let raw_name = if KEYWORDS.contains(&import.as_str()) { format!("{import}_") } else { import.clone() };
            imports.entry(import.clone()).or_insert_with(|| {
                let link = if raw_name != import { format!("#[link_name = \"{import}\"] ") } else { String::new() };
                format!("    {link}pub fn {raw_name}({}) -> ();\n", raw_params.join(", "))
            });
            inherent_taken.insert(with_name.clone());
            inherent += &format!("    pub fn {with_name}(self, {p}: {written}) -> Self {{ unsafe {{ raw::{raw_name}({}) }}; self }}\n", call_args.join(", "));
        }
        if !inherent_taken.contains("iter")
            && let Some((k, elem)) = std::iter::once(c.name.as_str()).chain(all.iter().map(String::as_str)).find_map(|h| class_of(h).and_then(|k| k.iterates.as_ref().map(|e| (k, e))))
        {
            let h_args = args_for(&k.name, k);
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false }, k).ret;
            let elem = subst(&elem, &k.type_params, &h_args);
            inherent_taken.insert("iter".into());
            inherent += &format!("    pub fn iter(&self) -> JsIterator<{}> {{ JsIterator::over({this}) }}\n", ty(&elem));
        }
        if !inherent_taken.contains("iterate_async")
            && let Some((k, elem)) = std::iter::once(c.name.as_str()).chain(all.iter().map(String::as_str)).find_map(|h| class_of(h).and_then(|k| k.iterates_async.as_ref().map(|e| (k, e))))
        {
            let h_args = args_for(&k.name, k);
            let elem = with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone(), returns_this: false }, k).ret;
            let elem = subst(&elem, &k.type_params, &h_args);
            inherent_taken.insert("iterate_async".into());
            inherent += &format!("    pub fn iterate_async(&self) -> JsAsyncIterator<{}> {{ JsAsyncIterator::over({this}) }}\n", ty(&elem));
        }
        // Covariant upcast for generic handles: `Texture<BufferImageSource>::upcast::<TextureSource<JsValue>>()`.
        if !c.type_params.is_empty() && !inherent_taken.contains("upcast") {
            let u_names: Vec<String> = (0..c.type_params.len()).map(|i| format!("U{i}_")).collect();
            let u_bounds: Vec<String> = u_names.iter().map(|u| format!("{u}: JsType")).collect();
            let wheres: Vec<String> = c.type_params.iter().zip(&u_names).map(|(p, u)| format!("{}: Is<{u}>", p.name)).collect();
            let u_args = u_names.join(", ");
            inherent_taken.insert("upcast".into());
            inherent += &format!(
                "    pub fn upcast<{}>(&self) -> {name}<{u_args}> where {} {{ {name}(self.0.clone(), core::marker::PhantomData) }}\n",
                u_bounds.join(", "),
                wheres.join(", ")
            );
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if inherent_taken.contains("cast") { "try_cast" } else { "cast" };
            let is_imp = format!("js_is__{}", c.name);
            imports.entry(is_imp.clone()).or_insert_with(|| format!("    pub fn {is_imp}(h: u32) -> i32;\n"));
            let opt = option();
            inherent_taken.insert(cast_name.into());
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
                    *out += &upcast_in(&bounds, &me, &tn(a), resolved_opaque.contains_key(a));
                }
                continue;
            };
            let base: Vec<Ty> = base_args(m, c, a).unwrap_or_default();
            let base_args_str = if k.type_params.is_empty() { String::new() } else { format!("<{}>", (0..k.type_params.len()).map(|i| base.get(i).map(ty).unwrap_or_else(|| VALUE.into())).collect::<Vec<_>>().join(", ")) };
            *out += &format!("impl{bounds} {}Methods{base_args_str} for {me} {{}}\n", tn(a));
            // Generic or not: `Marker<P>` is a `Layer`.
            *out += &upcast_in(&bounds, &me, &format!("{}{base_args_str}", tn(a)), false);
            // Covariant default-instantiated base for concrete classes: `Rect` (`Shape<RectConfig>`) -> `Shape<ShapeConfig>`,
            // `BufferImageSource` (`TextureSource<...>`) -> `TextureSource<JsValue>`.
            if c.type_params.is_empty() && !k.type_params.is_empty() {
                let def_tys: Vec<Ty> = k.type_params.iter().map(|p| p.default.clone().unwrap_or(Ty::Any)).collect();
                let def_str = format!("<{}>", def_tys.iter().map(ty).collect::<Vec<_>>().join(", "));
                let actual: Vec<Ty> = (0..k.type_params.len()).map(|i| base.get(i).cloned().unwrap_or(Ty::Any)).collect();
                if def_str != base_args_str && actual.iter().zip(&def_tys).all(|(x, y)| is_subtype(m, x, y, 0)) {
                    *out += &upcast_in(&bounds, &me, &format!("{}{def_str}", tn(a)), false);
                }
            }
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

    let deps = if used_refs.is_empty() {
        String::new()
    } else {
        let lines: String = used_refs
            .iter()
            .map(|&idx| format!("{} = {{ path = {:?} }}\n", refs[idx].crate_name, refs[idx].dep_path))
            .collect();
        format!("\n[dependencies]\n{lines}")
    };
    let mut lib = format!(
        "// Generated. Bindings: `use {crate_name}::*;`.\n#![allow(non_camel_case_types, non_snake_case, clippy::all)]\n\nmod raw;\nmod runtime;\npub use runtime::*;\nmod types;\npub use types::*;\n"
    );
    let mut out = vec![
        ("Cargo.toml".to_string(), format!("# Generated.\n[package]\nname = \"{crate_name}\"\nversion = \"0.1.0\"\nedition = \"2024\"\n{deps}")),
        ("src/runtime.rs".to_string(), RUNTIME.to_string()),
        ("src/types.rs".to_string(), types_file),
    ];
    // A namespace's module, by its own name (`pub mod stream;`), not taken
    // into the crate's: its types are `stream::PipeOptions`. Its members'
    // traits are in scope with the crate's, under no name.
    let tops: BTreeSet<&String> = namespaces.keys().filter_map(|k| k.first()).collect();
    for top in tops {
        let mut text = PRELUDE.to_string();
        fn nest(at: &[String], namespaces: &BTreeMap<Vec<String>, String>, text: &mut String) {
            *text += namespaces.get(at).map(String::as_str).unwrap_or_default();
            let children: BTreeSet<&String> = namespaces.keys().filter(|k| k.len() > at.len() && k[..at.len()] == *at).map(|k| &k[at.len()]).collect();
            for child in children {
                let mut path = at.to_vec();
                path.push(child.clone());
                *text += &format!("\npub mod {child} {{\n{}", PRELUDE.replace("// Generated.\n#![allow(unused_imports)]\n", "#[allow(unused_imports)]\nuse crate::*;\n"));
                nest(&path, namespaces, text);
                *text += "}\n";
            }
        }
        nest(std::slice::from_ref(top), &namespaces, &mut text);
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
    out.push(("src/raw.rs".into(), format!("// Generated. The imports.\n#![allow(dead_code, unused_imports)]\nuse core::ffi::c_void;\n\n#[link(wasm_import_module = \"{wasm_module}\")]\nunsafe extern \"C\" {{\n{}}}\n", imports.into_values().collect::<String>())));
    out.push(("src/lib.rs".into(), lib));
    let mut rust_types = BTreeMap::new();
    let is_top_exported = |rn: &str| !matches!(rn, "JsValue" | "Error" | "Date") && super::model::buffer_spec(rn).is_none();
    for c in &m.classes {
        if !module_of.contains_key(&c.name) {
            let rn = def_name(&c.name);
            if is_top_exported(&rn) {
                rust_types.insert(rn, (c.type_params.len(), true));
            }
        }
    }
    for c in &m.callbacks {
        if !module_of.contains_key(&c.name) {
            let rn = def_name(&c.name);
            if is_top_exported(&rn) {
                rust_types.insert(rn, (c.type_params.len(), true));
            }
        }
    }
    for (model_name, _) in &m.enums {
        if !module_of.contains_key(model_name) {
            rust_types.entry(def_name(model_name)).or_insert((0, false));
        }
    }
    for (model_name, _) in &m.num_enums {
        if !module_of.contains_key(model_name) {
            rust_types.entry(def_name(model_name)).or_insert((0, false));
        }
    }
    for u in m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()) {
        if !module_of.contains_key(&u.name) {
            rust_types.entry(def_name(&u.name)).or_insert((0, false));
        }
    }
    let meta = super::pkg_meta::PkgMeta {
        target: "rust".into(),
        package: crate_name.to_string(),
        entry: Some("Cargo.toml".into()),
        rust_types,
        ..Default::default()
    };
    out.push((super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}

// A subtype as its ancestor: by value with `into()`, by reference with
// `as_ref()`. Both are one handle, transparently; for a class or base of
// type parameters too (`impl<P: JsType>`: `Marker<P>` is a `Layer`).
fn upcast_in(bounds: &str, sub: &str, base: &str, is_external: bool) -> String {
    let name = base.split('<').next().unwrap_or(base);
    let make = if is_external || name.starts_with("::") {
        format!("<{base} as crate::runtime::FromJs>::from_handle({{ let h = x.0 .0; core::mem::forget(x); h }})")
    } else if base.contains('<') {
        format!("{name}(x.0, core::marker::PhantomData)")
    } else {
        format!("{name}(x.0)")
    };
    format!(
        "impl{bounds} From<{sub}> for {base} {{ fn from(x: {sub}) -> Self {{ {make} }} }}\n\
         impl{bounds} AsRef<{base}> for {sub} {{ fn as_ref(&self) -> &{base} {{ unsafe {{ &*(self as *const {sub} as *const {base}) }} }} }}\n\
         impl{bounds} Is<{base}> for {sub} {{}}\n"
    )
}

fn is_subtype(m: &Model, a: &Ty, b: &Ty, depth: usize) -> bool {
    if depth > 8 {
        return false;
    }
    if a == b || matches!(b, Ty::Any) {
        return true;
    }
    let class_of = |n: &str| m.classes.iter().find(|k| k.name == n);
    let has_ancestor = |sub: &str, target: &str| -> bool {
        let mut todo = vec![sub];
        let mut seen = HashSet::new();
        while let Some(cur) = todo.pop() {
            if cur == target {
                return true;
            }
            if seen.insert(cur)
                && let Some(k) = class_of(cur)
            {
                todo.extend(k.ancestors.iter().map(String::as_str));
            }
        }
        false
    };
    match (a, b) {
        (Ty::Obj(an), Ty::Obj(bn)) => has_ancestor(an, bn),
        (Ty::Generic(an, _), Ty::Obj(bn)) => has_ancestor(an, bn),
        (Ty::Generic(an, a_args), Ty::Generic(bn, b_args)) => {
            if an == bn && a_args.len() == b_args.len() {
                return a_args.iter().zip(b_args).all(|(x, y)| is_subtype(m, x, y, depth + 1));
            }
            let Some(ca) = class_of(an) else { return false };
            let Some(cb) = class_of(bn) else { return false };
            let Some(raw) = base_args(m, ca, bn) else { return false };
            let inst: Vec<Ty> = (0..cb.type_params.len())
                .map(|i| raw.get(i).map(|t| subst(t, &ca.type_params, a_args)).unwrap_or(Ty::Any))
                .collect();
            inst.len() == b_args.len() && inst.iter().zip(b_args).all(|(x, y)| is_subtype(m, x, y, depth + 1))
        }
        (Ty::Obj(an), Ty::Generic(bn, b_args)) => {
            let Some(ca) = class_of(an) else { return false };
            let Some(cb) = class_of(bn) else { return false };
            let Some(raw) = base_args(m, ca, bn) else { return false };
            let inst: Vec<Ty> = (0..cb.type_params.len()).map(|i| raw.get(i).cloned().unwrap_or(Ty::Any)).collect();
            inst.len() == b_args.len() && inst.iter().zip(b_args).all(|(x, y)| is_subtype(m, x, y, depth + 1))
        }
        _ => false,
    }
}

fn rename_tparam(f: &Func, old: &str, new: &str) -> Func {
    let rep = &[TParam { name: old.to_string(), constraint: None, default: Some(Ty::Param(new.to_string())) }];
    let mut out = map_types(f, &|t| subst(t, rep, &[]));
    for tp in &mut out.type_params {
        if tp.name == old {
            tp.name = new.to_string();
        }
        if let Some(c) = &tp.constraint {
            tp.constraint = Some(subst(c, rep, &[]));
        }
        if let Some(d) = &tp.default {
            tp.default = Some(subst(d, rep, &[]));
        }
    }
    out
}

fn avoid_outer_type_params(orig: &Func, outer: &[TParam], k_params: &[TParam]) -> Func {
    let mut out = orig.clone();
    for i in 0..out.type_params.len() {
        let name = out.type_params[i].name.clone();
        if outer.iter().any(|o| o.name == name) {
            let mut candidate = format!("{name}_");
            while outer.iter().any(|o| o.name == candidate) || k_params.iter().any(|p| p.name == candidate) || out.type_params.iter().any(|p| p.name == candidate) {
                candidate.push('_');
            }
            out = rename_tparam(&out, &name, &candidate);
        }
    }
    out
}

fn mentions_type(t: &Ty, name: &str) -> bool {
    match t {
        Ty::Obj(n) | Ty::NamedUnion(n) | Ty::Enum(n) | Ty::NumEnum(n) => n == name,
        Ty::Generic(n, args) => n == name || args.iter().any(|a| mentions_type(a, name)),
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => mentions_type(inner, name),
        Ty::Tuple(args) | Ty::Union(args) => args.iter().any(|a| mentions_type(a, name)),
        _ => false,
    }
}

fn handle_decl(model_name: &str, rust_name: &str, type_params: &[TParam]) -> String {
    if type_params.is_empty() {
        return format!("handle!({rust_name});");
    }
    let mut defaults: Vec<Option<String>> = vec![None; type_params.len()];
    for i in (0..type_params.len()).rev() {
        let Some(d) = &type_params[i].default else { break };
        if uses_key(d) || mentions_type(d, model_name) || type_params[i..].iter().any(|later| mentions(d, &later.name)) {
            break;
        }
        defaults[i] = Some(ty(d));
    }
    let parts: Vec<String> = type_params
        .iter()
        .zip(defaults)
        .map(|(p, d)| match d {
            Some(d) => format!("{} = {d}", p.name),
            None => p.name.clone(),
        })
        .collect();
    format!("handle!({rust_name}<{}>);", parts.join(", "))
}

// A union as an enum: decoded from a handle by its tag, lent as its member,
// and made from any member: `describe(42.0)`.
fn union_enum(u: &NamedUnion, m: &Model, imports: &mut BTreeMap<String, String>) -> String {
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
    let debug_arms: String = u.parts.iter().map(|p| format!("{n}::{}(x) => f.debug_tuple({:?}).field(x).finish(), ", variant(p), variant(p))).collect();
    let debug_where = if u.type_params.is_empty() {
        String::new()
    } else {
        format!(" where {}", u.type_params.iter().map(|p| format!("{}: core::fmt::Debug", p.name)).collect::<Vec<_>>().join(", "))
    };
    let mut out = format!(
        "pub enum {n}{bounds} {{ {} }}\n\
         impl{bounds} Clone for {n}{args} {{ fn clone(&self) -> Self {{ match self {{ {clone}}} }} }}\n\
         impl{bounds} ToJs for {n}{args} {{ fn lend(&self) -> Lent {{ match self {{ {lend}}} }} }}\n\
         impl{bounds} FromJs for {n}{args} {{ fn from_handle(h: u32) -> Self {{ match unsafe {{ raw::{import}_tag(h, {kind}) }} {{ {decode}_ => unreachable!() }} }} }}\n\
         impl<{is_params}> Is<B_> for {n}{args} where {} {{}}\n\
         impl{bounds} Optional<{n}{args}> for {opt}<{n}{args}> {{ fn into_option(self) -> {opt}<{n}{args}> {{ self }} }}\n\
         impl{bounds} Optional<{n}{args}> for JsNone {{ fn into_option(self) -> {opt}<{n}{args}> {{ None }} }}\n\
         impl{bounds} core::fmt::Debug for {n}{args}{debug_where} {{ fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {{ match self {{ {debug_arms}}} }} }}\n",
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
    // Made from concrete subtypes of an arm when unambiguous and small (e.g. Konva's `Rect` -> `GroupOrShape`).
    if u.type_params.is_empty() {
        let exact: HashSet<String> = u.parts.iter().map(ty).collect();
        let mut sub_impls: Vec<(String, String, String)> = Vec::new();
        for c in &m.classes {
            if !c.type_params.is_empty() {
                continue;
            }
            let c_ty = Ty::Obj(c.name.clone());
            let c_rust = ty(&c_ty);
            if exact.contains(&c_rust) {
                continue;
            }
            let matches: Vec<&Ty> = u.parts.iter().filter(|p| matches!(p, Ty::Obj(_) | Ty::Generic(..)) && is_subtype(m, &c_ty, p, 0)).collect();
            if matches.len() == 1 {
                let p = matches[0];
                sub_impls.push((c_rust, variant(p), ty(p)));
            }
        }
        if sub_impls.len() <= 40 {
            for (sub, v, target) in sub_impls {
                out += &format!(
                    "impl From<{sub}> for {n} {{ fn from(x: {sub}) -> Self {{ let h = x.0 .0; core::mem::forget(x); {n}::{v}(<{target}>::from_handle(h)) }} }}\n\
                     impl From<&{sub}> for {n} {{ fn from(x: &{sub}) -> Self {{ {n}::{v}(<{target}>::from_handle(crate::runtime::give(x))) }} }}\n"
                );
            }
        }
    }
    out
}

// A callback type, made from a closure: `CounterVisitF::new(|c: Counter| ..)`,
// or for the length of a call, `CounterVisitF::scoped(|c| .., |f| counter.visit(f))`.
fn callback(name: &str, type_params: &[TParam], params: &[(String, Ty)], ret: &Ty, apply: Option<&Func>, imports: &mut BTreeMap<String, String>) -> String {
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
        format!("extern \"C\" fn call<{}>(ctx: *mut core::ffi::c_void{raw_args}){raw_result} {{ {} }}", generic.join(", "), lower_callback(ret, &format!("(&mut *{f})({})", call_args.join(", "))))
    };
    let turbofish: Vec<String> = tps.iter().cloned().chain(std::iter::once("F".to_string())).collect();
    imports.insert(format!("{import}_from"), format!("    pub fn {import}_from(f: usize, ctx: *mut c_void) -> u32;\n"));
    let handle_params = if tps.is_empty() { String::new() } else { format!("<{}>", tps.join(", ")) };
    let is_self = if tps.is_empty() { format!("impl Is<{name}> for {name} {{}}\n") } else { String::new() };
    let call_method = if let Some(apply) = apply.filter(|a| !a.params.is_empty()) {
        let method_f = Func {
            type_params: Vec::new(),
            module: apply.module.clone(),
            import: apply.import.clone(),
            name: "call".into(),
            call: Call::Method("call".into()),
            params: apply.params[1..].to_vec(),
            rest: apply.rest,
            fixed: apply.fixed.clone(),
            shared: apply.shared.clone(),
            ret: apply.ret.clone(),
            returns_this: false,
        };
        format!("\n    {}", one_function(&method_f, "call", "pub ", imports, Some("self.0 .0")))
    } else {
        String::new()
    };
    format!(
        "handle!({name}{handle_params});\n{is_self}impl{bounds} JsCallback for {name}{args} {{}}\n\
         impl{bounds} {name}{args} {{\n    \
         pub fn new<F: {fn_bound} + 'static>(f: F) -> Self {{\n        {}\n        \
         Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, crate::runtime::closure(f)) }})\n    }}\n    \
         // For the length of `body`: its closure may borrow.\n    \
         pub fn scoped<F: {fn_bound}, R_>(f: F, body: impl FnOnce(&Self) -> R_) -> R_ {{\n        {}\n        \
         let f = core::cell::RefCell::new(f);\n        \
         let made = Self::from_handle(unsafe {{ raw::{import}_from(call::<{2}> as *const () as usize, &f as *const core::cell::RefCell<F> as *mut core::ffi::c_void) }});\n        \
         let _forget = crate::runtime::Forget(made.0 .0);\n        body(&made)\n    }}{call_method}\n}}\n",
        trampoline("unsafe { crate::runtime::closure_fn::<F>(ctx) }", " + 'static"),
        trampoline("unsafe { &*(ctx as *const core::cell::RefCell<F>) }.borrow_mut()", ""),
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
