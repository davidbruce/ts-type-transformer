use std::collections::{BTreeMap, HashMap, HashSet};

use super::{camel, ident, rust_function, snake, Context, KEYWORDS};
use super::super::comments::{Style, comment, text_lines};
use super::super::model::{base_args, map_types, mentions, subst, with_self_args, Call, Class, Func, Model, TParam, Ty};

pub(super) const VALUE: &str = "JsValue";

// The Rust type of a value: owned, as it comes back or as a type argument.
pub(super) fn ty(t: &Ty, cx: &Context) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "String".into(),
        Ty::Any | Ty::Union(_) => VALUE.into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArray<{}>", ty(e, cx)),
        Ty::Tuple(es) if (2..=4).contains(&es.len()) => format!("JsTuple{}<{}>", es.len(), es.iter().map(|e| ty(e, cx)).collect::<Vec<_>>().join(", ")),
        Ty::Tuple(_) => "JsArray".into(),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => cx.tn(n),
        Ty::Nullable(inner, _) => format!("{}<{}>", cx.option(), ty(inner, cx)),
        Ty::NamedUnion(n) if cx.decodable(n) => cx.tn(n),
        Ty::NamedUnion(_) => VALUE.into(),
        Ty::Key(m) => format!("JsKey<{}, V>", cx.tn(m)),
        Ty::Mapped(_) => "V".into(),
        Ty::Listener(ev) => format!("JsListener<{}>", ty(ev, cx)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, _) if cx.is_union(n) && !cx.decodable(n) => VALUE.into(),
        Ty::Generic(n, args) if !args.is_empty() => format!("{}<{}>", cx.tn(n), args.iter().map(|a| ty(a, cx)).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) => cx.tn(n),
    }
}

// A parameter: its type as written, and the expressions that lower it. What
// is lent lives until the end of the statement that makes the call.
pub(super) fn param(t: &Ty, x: &str, cx: &Context) -> (String, Vec<String>) {
    let lend = |e: &str| vec![format!("{e}.lend().h")];
    let into = |target: String| (format!("impl Into<{target}>"), vec![format!("Into::<{target}>::into({x}).lend().h")]);
    match t {
        Ty::Num => ("f64".into(), vec![x.into()]),
        Ty::NumEnum(_) => (ty(t, cx), vec![format!("{x} as i32 as f64")]),
        Ty::Bool | Ty::Enum(_) => (ty(t, cx), vec![format!("{x} as i32")]),
        Ty::Key(_) => (ty(t, cx), vec![format!("{x}.index")]),
        Ty::Str => ("&str".into(), vec![format!("{x}.as_ptr()"), format!("{x}.len()")]),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Str => ("impl Nullable<str>".into(), vec![format!("{x}.arg().map_or(core::ptr::null(), |s| s.as_ptr())"), format!("{x}.arg().map_or(0, |s| s.len())")]),
            // The value itself or None: `500.0`, `Some(500.0)`, `None`.
            Ty::Num => (format!("impl Into<{}>", ty(t, cx)), vec![format!("{x}.is_some() as i32"), format!("{x}.unwrap_or(0.0)")]),
            Ty::NumEnum(_) => (format!("impl Into<{}>", ty(t, cx)), vec![format!("{x}.is_some() as i32"), format!("{x}.map_or(0.0, |v| v as i32 as f64)")]),
            Ty::Bool | Ty::Enum(_) => (format!("impl Into<{}>", ty(t, cx)), vec![format!("{x}.map_or(-1, |v| v as i32)")]),
            // Any member, the union, or None: `options: true`, `Some(u)`, `None`.
            Ty::NamedUnion(n) | Ty::Generic(n, _) if cx.is_union(n) && cx.takes(n) => {
                let u = if cx.decodable(n) { ty(inner, cx) } else { cx.tn(n) };
                (format!("impl Optional<{u}>"), vec![format!("Optional::<{u}>::into_option({x}).lend().h")])
            }
            Ty::Param(_) => (ty(t, cx), lend(x)),
            // Any value, or None: `JsValue::from(None)` is null. So for a union
            // nothing types: `fill(0x111111 as f64)`, `fill("red")`, `fill(None)`.
            Ty::Any | Ty::Union(_) | Ty::NamedUnion(_) => into(VALUE.into()),
            Ty::Generic(n, _) if cx.is_union(n) => into(VALUE.into()),
            // The value by reference, `Some` of it, or None.
            inner => (format!("impl Nullable<{}>", ty(inner, cx)), vec![format!("{x}.arg().lend().h")]),
        },
        // Any value that is one: `name_of(&shape)` where a Named is wanted.
        Ty::Obj(n) => (format!("&impl AsRef<{}>", cx.tn(n)), vec![format!("AsRef::<{}>::as_ref({x}).lend().h", cx.tn(n))]),
        Ty::Generic(n, _) if !cx.is_union(n) => (format!("&impl AsRef<{}>", ty(t, cx)), vec![format!("AsRef::<{}>::as_ref({x}).lend().h", ty(t, cx))]),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => into(ty(t, cx)),
        Ty::NamedUnion(n) if cx.takes(n) && !cx.decodable(n) => into(cx.tn(n)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if cx.is_union(n) => into(ty(t, cx)),
        Ty::Any | Ty::Union(_) | Ty::NamedUnion(_) => into(VALUE.into()),
        // A value of a type parameter is passed as into a collection: by value.
        Ty::Param(_) => (ty(t, cx), lend(x)),
        _ => (format!("&{}", ty(t, cx)), lend(x)),
    }
}

// What a value crosses the boundary as, going in.
pub(super) fn raw(t: &Ty) -> Vec<&'static str> {
    use super::super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::super::wasm::param_abi(t) {
        None => vec![],
        One(F64) => vec!["f64"],
        One(I32) => vec!["i32"],
        One(U32 | Void) => vec!["u32"],
        Str => vec!["*const u8", "usize"],
        OptNum => vec!["i32", "f64"],
    }
}

// What a value crosses as coming back, and what a callback's arguments and result cross as.
pub(super) fn raw_ret(t: &Ty) -> &'static str {
    use super::super::wasm::AbiSlot::*;
    match super::super::wasm::ret_abi(t) {
        Void => "()",
        F64 => "f64",
        I32 => "i32",
        U32 => "u32",
    }
}

// Expression turning what came back, `r`, into a Rust value.
pub(super) fn raise(t: &Ty, r: &str, cx: &Context) -> String {
    match t {
        Ty::Void => "()".into(),
        Ty::Num => r.into(),
        Ty::NumEnum(e) => format!("{}::from_value({r} as i32)", cx.tn(e)),
        Ty::Bool => format!("{r} != 0"),
        Ty::Enum(e) => format!("{}::from_index({r})", cx.tn(e)),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("if {r} < 0 {{ None }} else {{ Some({r} != 0) }}"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("if {r} < 0 {{ None }} else {{ Some({}::from_index({r})) }}", ty(inner, cx)),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => format!("::core::option::Option::<f64>::from_handle({r}).map(|v| {}::from_value(v as i32))", ty(inner, cx)),
        t => format!("<{}>::from_handle({r})", ty(t, cx)),
    }
}

// What a callback returns, given to JavaScript.
pub(super) fn lower_callback(t: &Ty, x: &str) -> String {
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
pub(super) fn bound(p: &TParam, cx: &Context) -> String {
    match &p.constraint {
        Some(Ty::Obj(n)) if cx.is_class(n) => format!("{}: JsType + Is<{}>", p.name, cx.tn(n)),
        _ => format!("{}: JsType", p.name),
    }
}

// `<T: JsType, U: JsType>`, or nothing.
pub(super) fn generics(params: &[TParam], cx: &Context) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| bound(p, cx)).collect::<Vec<_>>().join(", ")) }
}

// `<T, U>`, or nothing.
pub(super) fn args_of(params: &[TParam]) -> String {
    if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

// A function's name: a property `x` reads as `x()` and writes as `set_x(..)`.
// A `$` method (`$add`) reads as `add_new()` rather than `_add()`.
pub(super) fn function_name_as(f: &Func, name: &str) -> String {
    let base = match &f.call {
        Call::Get(_) | Call::GetBound(_) | Call::StaticGet(_) | Call::IndexGet => name.strip_prefix("get_").unwrap_or(name),
        _ => name,
    };
    let dollar = super::super::case::has_dollar_js_name(&f.call);
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
pub(super) fn names_of<'a>(funcs: impl Iterator<Item = &'a Func>) -> HashMap<String, String> {
    funcs.map(|f| (f.name.clone(), rust_function(f, &f.name))).collect()
}

// One function as source: `vis fn name<..>(..) -> R { .. }`. `this` is the
// handle of the object it is called on, for members. A type parameter that
// has a default and cannot be inferred from the arguments is two functions:
// `name`, with the default, and `name_as::<T>`.
// Where a function is written: in an impl (`Self::new_with(..)`) or not, and
// the names beside it, which a form with its optional arguments keeps clear of.
pub(super) struct Site<'a> {
    pub assoc: bool,
    pub taken: &'a HashSet<String>,
}

pub(super) fn function(f: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>, site: &Site, cx: &Context) -> Vec<String> {
    function_raw(f, f, name, vis, imports, this, site, cx)
}

// How many trailing optional arguments a form leaves out: `World::new(&g)` is
// `World::new_with(&g, None, ..)`. None before a rest, or where one names the
// function's type parameter, which a `None` could not fix.
pub(super) fn left_out(f: &Func) -> usize {
    // A setter's value is what it sets: `set_x()` would set `undefined`.
    if f.rest || matches!(f.call, Call::Set(_) | Call::IndexSet) {
        return 0;
    }
    let k = f.params.iter().rev().take_while(|(_, t)| matches!(t, Ty::Nullable(_, a) if a.optional())).count();
    let (kept, dropped) = f.params.split_at(f.params.len() - k);
    // One it alone names is its default there (date-fns' `ResultDate = DateType`), else unknown.
    let unknown = |p: &TParam| p.default.is_none() && !kept.iter().any(|(_, t)| mentions(t, &p.name));
    if dropped.iter().any(|(_, t)| f.type_params.iter().any(|p| unknown(p) && mentions(t, &p.name))) { 0 } else { k }
}

// Its form with every argument, by `_with`, else `_full`: ProseMirror's `can_replace_with` is a method.
fn full_name(name: &str, site: &Site) -> Option<String> {
    [format!("{name}_with"), format!("{name}_full")].into_iter().find(|n| !site.taken.contains(n) && !site.taken.contains(&format!("{n}_as")))
}

// The form without its trailing optional arguments: it calls the one with them.
#[allow(clippy::too_many_arguments)]
fn short_form(f: &Func, orig: &Func, k: usize, name: &str, full: &str, vis: &str, this: Option<&str>, site: &Site, cx: &Context) -> String {
    let keep = f.params.len() - k;
    let short = Func { params: f.params[..keep].to_vec(), ..f.clone() };
    let short_orig = Func { params: orig.params[..keep].to_vec(), ..orig.clone() };
    // A type parameter only what it leaves out named is its default.
    let gone = hidden(&short);
    let (short, short_orig) = if gone.is_empty() { (short, short_orig) } else { (defaulted(&short), defaulted(&short_orig)) };
    let p = parts_with_raw(&short, Some(&short_orig), &mut BTreeMap::new(), this, cx);
    let full_generics = parts_with_raw(f, Some(orig), &mut BTreeMap::new(), this, cx).generics;
    let generics = if p.generics.is_empty() { String::new() } else { format!("<{}>", p.generics.join(", ")) };
    let ret = if f.ret == Ty::Void { String::new() } else { format!(" -> {}", p.ret) };
    let args: Vec<String> = short.params.iter().map(|(n, _)| ident(&snake(n))).chain(std::iter::repeat_n("None".to_string(), k)).collect();
    // Its type arguments stated: one that only has a default is not inferred.
    let default_of = |n: &str| {
        let mut t = Ty::Param(n.to_string());
        for _ in 0..=gone.len() {
            t = subst(&t, &gone, &[]);
        }
        ty(&t, cx)
    };
    let names: Vec<String> = full_generics.iter().map(|g| g.split(':').next().unwrap_or(g).trim().to_string()).collect();
    let turbofish = if names.is_empty() { String::new() } else { format!("::<{}>", names.iter().map(|n| if gone.iter().any(|g| g.name == *n) { default_of(n) } else { n.clone() }).collect::<Vec<_>>().join(", ")) };
    let callee = if this.is_some() { format!("self.{full}{turbofish}") } else if site.assoc { format!("Self::{full}{turbofish}") } else { format!("{full}{turbofish}") };
    format!("{}{vis}fn {name}{generics}({}){ret} {{ {callee}({}) }}", doc_of(f), p.params.join(", "), args.join(", "))
}

#[allow(clippy::too_many_arguments)]
pub(super) fn function_raw(f: &Func, orig: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>, site: &Site, cx: &Context) -> Vec<String> {
    let k = left_out(f);
    let Some(full) = (k > 0 && orig.params.len() == f.params.len()).then(|| full_name(name, site)).flatten() else {
        return forms(f, orig, name, vis, imports, this, cx);
    };
    let mut out = forms(&Func { doc: None, ..f.clone() }, orig, &full, vis, imports, this, cx);
    let (sf, so) = if hidden(f).is_empty() { (f.clone(), orig.clone()) } else { (defaulted(f), defaulted(orig)) };
    out.push(short_form(&sf, &so, k, name, &full, vis, this, site, cx));
    out
}

// Its form by `name`, and by `name_as` where a type argument is stated.
fn forms(f: &Func, orig: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>, cx: &Context) -> Vec<String> {
    if hidden(f).is_empty() {
        return vec![one_function_raw(f, Some(orig), name, vis, imports, this, cx)];
    }
    // The variant that names its type argument has the same documentation.
    let undocumented = |f: &Func| Func { doc: None, ..f.clone() };
    vec![
        one_function_raw(&defaulted(f), Some(&defaulted(orig)), name, vis, imports, this, cx),
        one_function_raw(&undocumented(f), Some(orig), &format!("{name}_as"), vis, imports, this, cx),
    ]
}

pub(super) fn one_function(f: &Func, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>, cx: &Context) -> String {
    one_function_raw(f, None, name, vis, imports, this, cx)
}

pub(super) fn one_function_raw(f: &Func, orig: Option<&Func>, name: &str, vis: &str, imports: &mut BTreeMap<String, String>, this: Option<&str>, cx: &Context) -> String {
    let p = parts_with_raw(f, orig, imports, this, cx);
    let generics = if p.generics.is_empty() { String::new() } else { format!("<{}>", p.generics.join(", ")) };
    let ret = if f.ret == Ty::Void { String::new() } else { format!(" -> {}", p.ret) };
    format!("{}{vis}fn {name}{generics}({}){ret} {{ {} }}", doc_of(f), p.params.join(", "), p.body)
}

// Its documentation, as lines before it: a member is indented by `indented`.
fn doc_of(f: &Func) -> String {
    f.doc.as_deref().map(|d| comment(d, "///", "", Style::Rust)).unwrap_or_default()
}

// A member of a trait or an impl, with its documentation, as lines.
pub(super) fn indented(item: &str) -> String {
    item.lines().map(|l| format!("    {l}\n")).collect()
}

// A function's pieces: its bounds, its parameters as written (`&self`
// first, for a member), its result and its body.
pub(super) struct Parts {
    pub generics: Vec<String>,
    pub params: Vec<String>,
    pub ret: String,
    pub body: String,
}

fn parts(f: &Func, imports: &mut BTreeMap<String, String>, this: Option<&str>, cx: &Context) -> Parts {
    parts_with_raw(f, None, imports, this, cx)
}

fn parts_with_raw(f: &Func, orig: Option<&Func>, imports: &mut BTreeMap<String, String>, this: Option<&str>, cx: &Context) -> Parts {
    let orig = orig.unwrap_or(f);
    let mut raw_params: Vec<String> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if let Some(this) = this {
        raw_params.push("this: u32".into());
        args.push(this.into());
    }
    let mut params: Vec<String> = if this.is_some() { vec!["&self".into()] } else { Vec::new() };
    // An optional number or flag is read more than once, as the `Option` it is made into.
    let mut lets = String::new();
    for (i, ((p, t), (_, orig_t))) in f.params.iter().zip(&orig.params).enumerate() {
        let p = ident(&snake(p));
        let (written, lowered) = if raw(orig_t) == raw(t) {
            param(t, &p, cx)
        } else {
            let w = if *t == Ty::Void { "()".into() } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_))) { ty(t, cx) } else { param(t, &p, cx).0 };
            (w, vec![format!("{p}.lend().h")])
        };
        if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_))) && raw(orig_t) == raw(t) {
            lets += &format!("let {p}: {} = {p}.into(); ", ty(t, cx));
        }
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
    // A type parameter no argument or result names cannot be inferred: it has no use.
    let used = |p: &TParam| p.default.is_some() || mentions(&f.ret, &p.name) || f.params.iter().any(|(_, t)| mentions(t, &p.name)) || f.type_params.iter().any(|q| q.constraint.as_ref().is_some_and(|c| mentions(c, &p.name)));
    let mut type_params: Vec<String> = f.type_params.iter().filter(|p| used(p)).map(|p| bound(p, cx)).collect();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        type_params.insert(0, "V: JsType".into());
    }
    let body = if raw_ret(&orig.ret) == raw_ret(&f.ret) {
        match &f.ret {
            Ty::Void => format!("unsafe {{ raw::{import}({}) }};", args.join(", ")),
            t => format!("let r = unsafe {{ raw::{import}({}) }}; {}", args.join(", "), raise(t, "r", cx)),
        }
    } else {
        let semi = if f.ret == Ty::Void { ";" } else { "" };
        format!("let r = unsafe {{ raw::{import}({}) }}; <{}>::from_handle(r){semi}", args.join(", "), ty(&f.ret, cx))
    };
    Parts { generics: type_params, params, ret: ty(&f.ret, cx), body: format!("{lets}{body}") }
}

// Overloads of one method that differ in one parameter, which is a key of
// one map or another, or a string: one method, generic in that parameter.
// `el.query_selector(HTMLElementTagNameMap::P)` is an HTMLParagraphElement,
// `el.query_selector("p.intro")` an Element.
// Where they differ in other parameters too, those are the trait's type
// argument, which the chosen overload fixes: `add_event_listener(HTMLElementEventMap::CLICK,
// &listener, None)` takes a JsListener<PointerEvent>, `add_event_listener("ping", on_ping, None)`
// anything that is an EventListenerOrEventListenerObject.
pub(super) struct Family {
    pub name: String,
    pub trait_name: String,
    pub at: usize,
    // The other parameters in which they differ.
    pub others: Vec<usize>,
    pub members: Vec<Func>,
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

pub(super) fn uses_key(t: &Ty) -> bool {
    match t {
        Ty::Key(_) | Ty::Mapped(_) => true,
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => uses_key(inner),
        Ty::Generic(_, args) | Ty::Tuple(args) | Ty::Union(args) => args.iter().any(uses_key),
        _ => false,
    }
}

pub(super) fn families(c: &Class, names: &HashMap<String, String>, types: &HashSet<String>, cx: &Context) -> Vec<Family> {
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
        let trait_name = format!("{}{}Arg", cx.tn(&c.name), camel(&name));
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
    pub(super) fn methods(&self, vis: &str, imports: &mut BTreeMap<String, String>, this: &str, ret_self: Option<&Ty>, documented: bool, cx: &Context) -> Vec<String> {
        let first = &self.members[0];
        let names: Vec<String> = first.params.iter().map(|(p, _)| ident(&snake(p))).collect();
        let params: Vec<String> = first
            .params
            .iter()
            .zip(&names)
            .enumerate()
            .map(|(i, ((_, t), p))| if i == self.at { format!("{p}: A_") } else if self.others.contains(&i) { format!("{p}: R{i}_") } else { format!("{p}: {}", param(t, p, cx).0) })
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
        let mut out = vec![format!("{}{vis}fn {}<A_: {}{}{generics}>(&self, {}) -> {ret_s} {{ {body_s} }}", if documented { doc_of(first) } else { String::new() }, self.name, self.trait_name, self.rest(), params.join(", "))];
        if let Some(f) = self.members.iter().find(|f| f.params[self.at].1 == Ty::Str && !hidden(f).is_empty()) {
            let mut f = f.clone();
            if let Some(sty) = ret_self {
                f.ret = sty.clone();
            }
            f.doc = None;
            out.push(one_function(&f, &format!("{}_as", self.name), vis, imports, Some(this), cx));
        }
        out
    }

    // The trait of what the parameter may be, and each overload as one of them.
    pub(super) fn dispatch(&self, imports: &mut BTreeMap<String, String>, cx: &Context) -> String {
        let same = |p: &Parts| -> String { p.params.iter().skip(1).enumerate().filter(|(i, _)| *i != self.at && !self.others.contains(i)).map(|(_, p)| format!(", {p}")).collect() };
        let first = &self.members[0];
        let p = parts(first, &mut BTreeMap::new(), Some("this"), cx);
        let rest_param = if self.others.is_empty() { String::new() } else { format!(", rest: ({},)", self.others.iter().map(|i| format!("R{i}_")).collect::<Vec<_>>().join(", ")) };
        let trait_params = if self.others.is_empty() { String::new() } else { format!("<{}>", self.others.iter().map(|i| format!("R{i}_")).collect::<Vec<_>>().join(", ")) };
        let own = self.trait_name.rsplit("::").next().unwrap_or_default();
        let mut out = format!("pub trait {own}{trait_params} {{\n    type Output;\n    #[doc(hidden)]\n    fn call(self, this: u32{rest_param}{}) -> Self::Output;\n}}\n", same(&p));
        for f in &self.members {
            let f = &defaulted(f);
            let p = parts(f, imports, Some("this"), cx);
            let key = ident(&snake(&f.params[self.at].0));
            let mut generics = p.generics.clone();
            let (mut rest_types, mut rest_names) = (Vec::new(), Vec::new());
            for &i in &self.others {
                let name = ident(&snake(&f.params[i].0));
                let (bounds, t) = nameable(&param(&f.params[i].1, &name, cx).0, i);
                generics.extend(bounds);
                rest_types.push(t);
                rest_names.push(name);
            }
            let (self_generics, self_type) = nameable(&param(&f.params[self.at].1, &key, cx).0, self.at);
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
pub(super) fn hidden(f: &Func) -> Vec<TParam> {
    f.type_params.iter().filter(|p| p.default.is_some() && !f.params.iter().any(|(_, t)| mentions(t, &p.name))).cloned().collect()
}

// The function with those parameters at their defaults.
pub(super) fn defaulted(f: &Func) -> Func {
    let hidden = hidden(f);
    // A default may name another that is hidden: `R = D` beside `D = Date`.
    let mut out = f.clone();
    let fill = |t: &Ty| (0..=hidden.len()).fold(t.clone(), |t, _| subst(&t, &hidden, &[]));
    out = map_types(&out, &fill);
    out.type_params.retain(|p| !hidden.iter().any(|h| h.name == p.name));
    // And in those left: `ResultDate = DateType` where `DateType` is `Date`.
    for p in &mut out.type_params {
        p.default = p.default.as_ref().map(&fill);
        p.constraint = p.constraint.as_ref().map(&fill);
    }
    out
}

// A subtype as its ancestor: by value with `into()`, by reference with
// `as_ref()`. Both are one handle, transparently; for a class or base of
// type parameters too (`impl<P: JsType>`: `Marker<P>` is a `Layer`).
pub(super) fn upcast_in(bounds: &str, sub: &str, base: &str, is_external: bool) -> String {
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

pub(super) fn is_subtype(m: &Model, a: &Ty, b: &Ty, depth: usize) -> bool {
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

pub(super) fn avoid_outer_type_params(orig: &Func, outer: &[TParam], k_params: &[TParam]) -> Func {
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

// `handle!` takes the documentation as attributes, for it makes the struct.
pub(super) fn handle_decl(model_name: &str, rust_name: &str, type_params: &[TParam], doc: Option<&str>, cx: &Context) -> String {
    let attrs: String = doc.map(|d| text_lines(d, Style::Rust).iter().map(|l| format!("#[doc = {:?}] ", format!(" {l}").trim_end())).collect()).unwrap_or_default();
    if type_params.is_empty() {
        return format!("handle!({attrs}{rust_name});");
    }
    let mut defaults: Vec<Option<String>> = vec![None; type_params.len()];
    for i in (0..type_params.len()).rev() {
        let Some(d) = &type_params[i].default else { break };
        if uses_key(d) || mentions_type(d, model_name) || type_params[i..].iter().any(|later| mentions(d, &later.name)) {
            break;
        }
        defaults[i] = Some(ty(d, cx));
    }
    let parts: Vec<String> = type_params
        .iter()
        .zip(defaults)
        .map(|(p, d)| match d {
            Some(d) => format!("{} = {d}", p.name),
            None => p.name.clone(),
        })
        .collect();
    format!("handle!({attrs}{rust_name}<{}>);", parts.join(", "))
}

// A buffer element as a Rust type.
pub(super) fn rust_elem(e: super::super::model::Elem) -> &'static str {
    use super::super::model::Elem::*;
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
pub(super) fn ns_module(name: &str) -> String {
    let m = ident(&snake(name));
    if matches!(m.as_str(), "raw" | "runtime" | "types" | "lib") { format!("{m}_") } else { m }
}
