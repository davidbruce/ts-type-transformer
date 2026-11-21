// XML -> C++ bindings for a wasm32 module, built with the WASI SDK.
//
// Every JavaScript object is a value holding a handle, and every type
// inherits, virtually, from `ts::Handle`: inheritance and mixins are real C++
// inheritance, sharing one handle, and an HTMLDivElement is an Element.
// Strings go in as `std::string_view` and come back as `std::string`;
// nullables are `std::optional`, unions `std::variant`, arrays `ts::Array<T>`,
// callbacks lambdas, generic types class templates, and keyed methods
// templates over typed keys:
// `el.addEventListener(HTMLElementEventMapKeys::click, [](PointerEvent e) {...})`.
//
// Files:
// - `X.decl.hpp`: the struct, declaring its members. Includes only its
//   ancestors' structs, so a chain of them never meets a half-read header.
// - `X.hpp`: what a program includes. The struct, then the headers of every
//   type its members name, then the definitions of templates.
// - `<module>.cpp`: every other definition, compiled once and linked.
// - `types/X.hpp`: a callback or union.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::Path;

use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, subst, with_self_args};
use super::names::{Kind, Naming};

/// A reference to a previously generated C++ binding directory.
pub struct ProjectRef {
    pub pkg_name: String,
    pub umbrella_include: String,
    pub types: HashSet<String>,
}

fn is_builtin_cpp_type(name: &str) -> bool {
    matches!(name, "Error" | "Date" | "Promise" | "PromiseLike") || super::model::buffer_spec(name).is_some()
}

/// Loads a previously generated C++ binding directory (via `tsrs-meta.toml`)
/// so another generated C++ package can `#include` its umbrella header and
/// emit `using Type = ::ref_pkg::Type;` aliases instead of opaque stubs.
pub fn load_project_ref(dir_or_header: &Path, from_dir: &Path) -> Result<ProjectRef, String> {
    let loaded = super::pkg_meta::load(dir_or_header, from_dir, "cpp")?;
    let entry = loaded.meta.entry.as_deref().unwrap_or("bindings.hpp");
    let pkg_name = loaded.meta.package;
    let umbrella_include = format!("{}/{entry}", loaded.rel_dir);
    let types = loaded.meta.types.into_iter().collect();
    Ok(ProjectRef { pkg_name, umbrella_include, types })
}

fn import_attr(wasm_module: &str, name: &str) -> String {
    if wasm_module == "env" {
        format!("TS_IMPORT({name})")
    } else {
        format!("TS_IMPORT_MOD(\"{wasm_module}\", {name})")
    }
}

fn raw_fn(raw_prefix: &str, name: &str) -> String {
    format!("ts_raw_{raw_prefix}{name}")
}

fn wrap_ns(ns: Option<&str>, body: &str) -> String {
    match ns {
        Some(pkg) if !body.trim().is_empty() => format!("namespace {pkg} {{\n{body}}}  // namespace {pkg}\n"),
        _ => body.to_string(),
    }
}

const KEYWORDS: &[&str] = &[
        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor", "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
        "char32_t", "class", "compl", "concept", "const", "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await", "co_return",
        "co_yield", "decltype", "default", "delete", "do", "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
        "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator",
        "or", "or_eq", "private", "protected", "public", "register", "reinterpret_cast", "requires", "return", "short", "signed", "sizeof", "static",
        "static_assert", "static_cast", "struct", "switch", "template", "this", "thread_local", "throw", "true", "try", "typedef", "typeid",
        "typename", "union", "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq", "errno", "assert", "NULL", "stdin", "stdout", "stderr",
    // The handle every type holds.
    "h",
];

fn ident(name: &str) -> String {
    let s = super::model::ident(name);
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// Types, their helpers (`decode_Id`, `Mode_keys`) and free functions are in
// the global namespace; a type's functions in its own, where they overload.
pub const MODEL_OPTIONS: super::model::ModelOptions<'static> = super::model::ModelOptions {
    target: "C++",
    counted: true,
    generics: true,
    overloading: true,
    retain_opaque_constraints: false,
};

pub const NAMING: Naming = Naming {
    reserved: || KEYWORDS.iter().map(|k| k.to_string()).chain(["ts", "std"].map(String::from)).collect(),
    type_names: |candidate, name, kind, m| {
        let mut ids = vec![candidate.to_string()];
        match kind {
            Kind::Union => ids.push(format!("decode_{candidate}")),
            Kind::Class if m.maps.iter().any(|k| k.name == name) => ids.push(format!("{candidate}Keys")),
            _ => {}
        }
        ids
    },
    values_of: |_, _, _, _| Vec::new(),
    types_apart: false,
    overloads: true,
    // A getter or setter is its property's name, a constructor `create`.
    func_name: |owner, name, f| match &f.call {
        Call::Get(_) | Call::GetBound(_) | Call::Set(_) | Call::IndexGet | Call::IndexSet | Call::New(_) | Call::Create(_) => None,
        Call::StaticGet(_) | Call::StaticSet(_) if f.module.is_empty() => None,
        _ => Some(cpp_name_as(f, name, if owner.is_some() { "create" } else { "" })),
    },
    funcs_flat: false,
    nests: false,
    members: |c, _| vec![c.name.clone(), "h".into()],
};

// C++ type of a value going in. `V` is the value a typed key selects.
fn ty_in(t: &Ty) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "std::string_view".into(),
        Ty::Any => "ts::Value".into(),
        Ty::Array => "ts::Array<>".into(),
        Ty::ArrayOf(e) => format!("ts::Array<{}>", ty_arg_for("", e)),
        Ty::Tuple(es) => format!("ts::Tuple<{}>", es.iter().map(|e| ty_arg_for("", e)).collect::<Vec<_>>().join(", ")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => tn(n),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_)) => format!("std::optional<{}>", ty_in(inner).trim_start_matches("const ").trim_end_matches('&')),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_in(inner)),
        Ty::NamedUnion(n) => format!("const {}&", tn(n)),
        Ty::Union(_) => "ts::Value".into(),
        Ty::Key(m) => format!("ts::Key<{m}, V>"),
        Ty::Mapped(_) => "V".into(),
        // Not deduced from the lambda: the key says what the listener takes.
        Ty::Listener(ev) => format!("std::type_identity_t<ts::Listener<{}>>", ty_in(ev)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, args) => format!("{}<{}>", tn(n), args.iter().map(|a| ty_arg_for(n, a)).collect::<Vec<_>>().join(", ")),
    }
}

fn is_adapted_param(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> bool {
    match t {
        Ty::NamedUnion(_) => true,
        Ty::Generic(n, _) => is_union(n, unions),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(_) | Ty::Tuple(_) => true,
            Ty::Generic(n, _) => is_union(n, unions) || callbacks.contains(n.as_str()),
            Ty::Obj(n) => callbacks.contains(n.as_str()),
            _ => false,
        },
        _ => false,
    }
}

fn contains_param(t: &Ty) -> bool {
    match t {
        Ty::Param(_) => true,
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => contains_param(inner),
        Ty::Tuple(es) | Ty::Union(es) => es.iter().any(contains_param),
        Ty::Generic(_, args) => args.iter().any(contains_param),
        _ => false,
    }
}

// C++ type of a function/method parameter, using input adapters (`ts::In`/`ts::Opt`)
// where ergonomic implicit conversions are needed.
fn ty_param(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> String {
    let wrap = |inner: String| {
        if contains_param(t) {
            format!("const std::type_identity_t<{inner}>&")
        } else {
            format!("const {inner}&")
        }
    };
    match t {
        Ty::NamedUnion(n) => wrap(format!("ts::In<{}>", tn(n))),
        Ty::Generic(n, _) if is_union(n, unions) => wrap(format!("ts::In<{}>", ty_in(t))),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => wrap(format!("ts::Opt<{}>", tn(n))),
            Ty::Generic(n, _) if is_union(n, unions) || callbacks.contains(n.as_str()) => wrap(format!("ts::Opt<{}>", ty_in(inner))),
            Ty::Obj(n) if callbacks.contains(n.as_str()) => wrap(format!("ts::Opt<{}>", ty_in(inner))),
            Ty::Tuple(_) => wrap(format!("ts::Opt<{}>", ty_in(inner))),
            _ => ty_in(t),
        },
        _ => ty_in(t),
    }
}

fn union_subsumed(sub_t: &Ty, sup_t: &Ty, unions: &[NamedUnion]) -> bool {
    match (sub_t, sup_t) {
        (Ty::NamedUnion(a), Ty::NamedUnion(b)) if a != b => {
            let Some(ua) = unions.iter().find(|u| u.name == *a && u.type_params.is_empty()) else { return false; };
            let Some(ub) = unions.iter().find(|u| u.name == *b && u.type_params.is_empty()) else { return false; };
            (ua.parts.len() < ub.parts.len() || (ua.parts.len() == ub.parts.len() && a > b))
                && ua.parts.iter().all(|p| ub.parts.contains(p))
        }
        (Ty::Nullable(a, _), Ty::Nullable(b, _)) => union_subsumed(a, b, unions),
        _ => false,
    }
}

fn func_subsumed_by(
    sub: &Func,
    sup: &Func,
    sub_name: &str,
    sup_name: &str,
    unions: &[NamedUnion],
    callbacks: &HashSet<&str>,
) -> bool {
    if sub_name != sup_name
        || Class::has_self(sub) != Class::has_self(sup)
        || sub.params.len() != sup.params.len()
        || !sub.type_params.is_empty()
        || !sup.type_params.is_empty()
        || sub.fixed != sup.fixed
        || sub.call != sup.call
        || sub.rest != sup.rest
        || ty_out(&sub.ret, unions) != ty_out(&sup.ret, unions)
    {
        return false;
    }
    let mut strict = false;
    for ((_, st), (_, pt)) in sub.params.iter().zip(&sup.params) {
        if ty_param(st, unions, callbacks) == ty_param(pt, unions, callbacks) {
            continue;
        }
        if union_subsumed(st, pt, unions) {
            strict = true;
        } else {
            return false;
        }
    }
    strict
}

fn lower_param(t: &Ty, x: &str, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> Vec<String> {
    if is_adapted_param(t, unions, callbacks) {
        vec![format!("{x}.h")]
    } else {
        lower(t, x)
    }
}

fn is_deducible(param: &str, t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> bool {
    match t {
        Ty::Param(n) => n == param,
        Ty::ArrayOf(e) => is_deducible(param, e, unions, callbacks),
        Ty::Tuple(es) => es.iter().any(|e| is_deducible(param, e, unions, callbacks)),
        Ty::Nullable(inner, _) => {
            !is_adapted_param(t, unions, callbacks) && is_deducible(param, inner, unions, callbacks)
        }
        Ty::Generic(n, args) if !is_union(n, unions) && !callbacks.contains(n.as_str()) => {
            args.iter().any(|a| is_deducible(param, a, unions, callbacks))
        }
        _ => false,
    }
}

fn head_func(params: &[TParam], defaults: bool) -> String {
    head_with_extra(params, defaults, &[], &[])
}

fn head_with_extra(params: &[TParam], defaults: bool, extra_params: &[&str], extra_requires: &[String]) -> String {
    if params.is_empty() && extra_params.is_empty() {
        return String::new();
    }
    let mut ps: Vec<String> = params
        .iter()
        .map(|p| match &p.default {
            Some(d) if defaults => format!("class {} = {}", p.name, ty_arg_for("", d)),
            _ => format!("class {}", p.name),
        })
        .collect();
    ps.extend(extra_params.iter().map(|s| (*s).to_string()));
    let mut requires: Vec<String> = params.iter().filter_map(|p| constraint(&p.name, p.constraint.as_ref()?)).collect();
    requires.extend(extra_requires.iter().cloned());
    let requires = if requires.is_empty() { String::new() } else { format!(" requires ({})", requires.join(" && ")) };
    format!("template <{}>{requires}\n", ps.join(", "))
}

thread_local! {
    // The unions of the model being generated that values can be decoded to.
    static DECODABLE: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
    // Types a class's member is named as (`easingFunc easingFunc()`): within
    // the class the member hides the type, so it is written `::easingFunc`.
    static QUALIFIED: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
    static NS_PREFIX: std::cell::RefCell<String> = const { std::cell::RefCell::new(String::new()) };
    // Types imported from `--ref` project headers (may be handles, enums, or variants).
    static RESOLVED_REF: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

fn is_resolved_ref(n: &str) -> bool {
    RESOLVED_REF.with(|r| r.borrow().contains(n))
}

// A declared type's name where a type is written: global where a member hides it.
fn tn(n: &str) -> String {
    if QUALIFIED.with(|q| q.borrow().contains(n)) {
        NS_PREFIX.with(|p| format!("::{}{n}", p.borrow()))
    } else {
        n.to_string()
    }
}

// A type as a template argument: a value type, not a view.
fn ty_arg(t: &Ty) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_arg(inner)),
        // A union a handle can be decoded to; any other stays a value.
        Ty::NamedUnion(n) if DECODABLE.with(|d| d.borrow().contains(n)) => tn(n),
        Ty::NamedUnion(_) | Ty::Union(_) => "ts::Value".into(),
        t => ty_in(t),
    }
}

fn ty_arg_for(owner: &str, t: &Ty) -> String {
    if *t == Ty::Void && !matches!(owner, "Promise" | "PromiseLike") {
        "ts::Value".into()
    } else {
        ty_arg(t)
    }
}

// C++ type of a value coming back.
fn ty_out(t: &Ty, unions: &[NamedUnion]) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_out(inner, unions)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n, unions) && !decodable(n, unions) => "ts::Value".into(),
        Ty::NamedUnion(n) => tn(n),
        t => ty_in(t),
    }
}

fn is_union(n: &str, unions: &[NamedUnion]) -> bool {
    unions.iter().any(|u| u.name == n)
}

fn decodable(n: &str, unions: &[NamedUnion]) -> bool {
    unions.iter().any(|u| u.name == n && u.decodable())
}

// `template <class E = Element> requires ts::extends<E, Element>`, or nothing.
// Defaults are given only where a template is first declared, and only if
// every later parameter has one too.
fn head(params: &[TParam], defaults: bool) -> String {
    if params.is_empty() {
        return String::new();
    }
    let trailing = params.iter().skip_while(|p| p.default.is_none()).all(|p| p.default.is_some());
    let ps: Vec<String> = params
        .iter()
        .map(|p| match &p.default {
            Some(d) if defaults && trailing => format!("class {} = {}", p.name, ty_arg_for("", d)),
            _ => format!("class {}", p.name),
        })
        .collect();
    let requires: Vec<String> = params.iter().filter_map(|p| constraint(&p.name, p.constraint.as_ref()?)).collect();
    // In parentheses: a type written `::T` after it would join it.
    let requires = if requires.is_empty() { String::new() } else { format!(" requires ({})", requires.join(" && ")) };
    format!("template <{}>{requires}\n", ps.join(", "))
}

// A type's constructors: none, or from a handle.
// With its assignments: the handle, a virtual base by several paths, is
// assigned once (an implicit move assignment may move it once per path, the
// second from what the first emptied).
fn constructors(name: &str) -> String {
    format!(
        "    {name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n\
         \x20   {name}(const {name}&) = default;\n    {name}({name}&&) = default;\n\
         \x20   {name}& operator=(const {name}& o) {{ ts::Handle::operator=(static_cast<const ts::Handle&>(o)); return *this; }}\n\
         \x20   {name}& operator=({name}&& o) noexcept {{ ts::Handle::operator=(std::move(static_cast<ts::Handle&>(o))); return *this; }}\n"
    )
}

// A type parameter's constraint, where C++ can state it.
fn constraint(name: &str, c: &Ty) -> Option<String> {
    match c {
        Ty::Obj(_) | Ty::Generic(..) | Ty::Param(_) | Ty::Str | Ty::Num | Ty::Bool | Ty::ArrayOf(_) | Ty::Array => Some(format!("ts::extends<{name}, {}>", ty_arg(c))),
        _ => None,
    }
}

// The wasm types a value crosses as.
fn raw(t: &Ty) -> Vec<&'static str> {
    use super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::wasm::param_abi(t) {
        None => vec![],
        One(F64) => vec!["double"],
        One(I32) => vec!["int32_t"],
        One(U32 | Void) => vec!["uint32_t"],
        Str => vec!["const char*", "size_t"],
        OptNum => vec!["int32_t", "double"],
    }
}

fn raw_ret(t: &Ty) -> &'static str {
    use super::wasm::AbiSlot::*;
    match super::wasm::ret_abi(t) {
        Void => "void",
        F64 => "double",
        I32 => "int32_t",
        U32 => "uint32_t",
    }
}

// Expressions turning a C++ value into what crosses the boundary.
fn lower(t: &Ty, x: &str) -> Vec<String> {
    match t {
        Ty::Num => vec![x.into()],
        Ty::NumEnum(_) => vec![format!("double({x})")],
        Ty::Bool => vec![format!("({x} ? 1 : 0)")],
        Ty::Enum(_) => vec![format!("int32_t({x})")],
        Ty::Key(_) => vec![format!("{x}.index")],
        Ty::Str => vec![format!("{x}.data()"), format!("{x}.size()")],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec![format!("({x} ? {x}->data() : nullptr)"), format!("({x} ? {x}->size() : 0)")],
        Ty::Nullable(inner, _) if **inner == Ty::Num => vec![format!("({x} ? 1 : 0)"), format!("({x} ? *{x} : 0.0)")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => vec![format!("({x} ? 1 : 0)"), format!("({x} ? double(*{x}) : 0.0)")],
        Ty::Nullable(inner, _) if **inner == Ty::Bool => vec![format!("({x} ? (*{x} ? 1 : 0) : -1)")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => vec![format!("({x} ? int32_t(*{x}) : -1)")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Param(_) | Ty::Generic(..) | Ty::NamedUnion(_)) => vec![format!("ts::Arg({x}).h")],
        Ty::Nullable(inner, _) if matches!(&**inner, Ty::Obj(n) if is_resolved_ref(n)) => vec![format!("ts::Arg({x}).h")],
        Ty::Nullable(..) => vec![format!("({x} ? {x}->h : 0)")],
        Ty::Param(_) | Ty::Generic(..) | Ty::NamedUnion(_) => vec![format!("ts::Arg({x}).h")],
        Ty::Obj(n) if is_resolved_ref(n) => vec![format!("ts::Arg({x}).h")],
        _ => vec![format!("{x}.h")],
    }
}

// Expression turning what came back, `r`, into a C++ value.
fn raise(t: &Ty, r: &str, unions: &[NamedUnion]) -> String {
    match t {
        Ty::Num => r.into(),
        Ty::NumEnum(e) => format!("{e}(int32_t({r}))"),
        Ty::Bool => format!("({r} != 0)"),
        Ty::Enum(e) => format!("{e}({r})"),
        Ty::Str => format!("ts::from_handle<std::string>({r})"),
        Ty::Nullable(inner, _) if **inner == Ty::Num => format!("ts::from_handle<std::optional<double>>({r})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => {
            format!("({r} ? {}({}(int32_t(ts::from_handle<double>({r})))) : std::nullopt)", ty_out(t, unions), ty_in(inner))
        }
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("({r} < 0 ? std::nullopt : std::optional<bool>({r} != 0))"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("({r} < 0 ? std::nullopt : {}({}({r})))", ty_out(t, unions), ty_in(inner)),
        Ty::Nullable(inner, _) => format!("({r} ? {}({}) : std::nullopt)", ty_out(t, unions), raise(inner, r, unions)),
        Ty::NamedUnion(n) if decodable(n, unions) => format!("decode_{n}({r})"),
        Ty::Generic(n, args) if decodable(n, unions) => format!("decode_{n}<{}>({r})", args.iter().map(|a| ty_arg_for(n, a)).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) if is_union(n, unions) => format!("ts::Value(ts::handle, {r})"),
        Ty::Obj(n) if is_resolved_ref(n) => format!("ts::from_handle<{}>({r})", tn(n)),
        Ty::Obj(n) => format!("{}(ts::handle, {r})", tn(n)),
        Ty::Mapped(_) => format!("V(ts::handle, {r})"),
        Ty::Param(n) => format!("ts::from_handle<{n}>({r})"),
        Ty::Generic(..) | Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => format!("{}(ts::handle, {r})", ty_in(t)),
        _ => format!("ts::Value(ts::handle, {r})"),
    }
}

#[derive(PartialEq, Clone, Copy)]
enum Place {
    // A member of the class: `h` is the handle of the object it is called on.
    Member,
    // A static member of the class.
    Static,
    // A free function.
    Free,
}

// The C++ name of a function, from what it calls. `ctor` names constructors.
fn cpp_name(f: &Func, ctor: &str) -> String {
    cpp_name_as(f, &f.name, ctor)
}

// As `cpp_name`, were the function's model name `name`.
fn cpp_name_as(f: &Func, name: &str, ctor: &str) -> String {
    let base = match &f.call {
        Call::Get(p) | Call::GetBound(p) | Call::Set(p) => ident(p),
        Call::StaticGet(p) | Call::StaticSet(p) if f.module.is_empty() => ident(p.rsplit('.').next().unwrap_or(p)),
        Call::IndexGet => "at".into(),
        Call::IndexSet => "set_at".into(),
        Call::New(_) | Call::Create(_) => ctor.into(),
        _ => ident(name),
    };
    // Overloads that differ only in a fixed argument are named after it.
    if f.fixed.is_empty() {
        base
    } else {
        let values: Vec<String> = f.fixed.iter().map(|(_, v)| super::value_name(v)).collect();
        format!("{base}_{}", values.join("_"))
    }
}

// One function, in its parts.
struct Function {
    // The wasm import it calls.
    import: String,
    // Its declaration: in the class, or in the module's header.
    decl: String,
    // Its definition.
    def: String,
    // A template, defined in a header; else defined once, in the module's `.cpp`.
    templated: bool,
    // What C++ tells overloads apart by: name and parameter types.
    key: String,
    // The types it names.
    refs: BTreeSet<String>,
}

// `owner` is the class as written in a definition (`Box<T>`), with its template
// head (`template <class T>`).
#[allow(clippy::too_many_arguments)]
fn build_function(
    f: &Func,
    name: &str,
    owner: &str,
    owner_head: &str,
    place: Place,
    unions: &[NamedUnion],
    callbacks: &HashSet<&str>,
    wasm_module: &str,
    raw_prefix: &str,
    params_slice: &[(String, Ty)],
    rest_pack: Option<&str>,
    omit_rest: bool,
) -> Function {
    let mut raw_params: Vec<&str> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if Class::has_self(f) {
        raw_params.push("uint32_t");
        // `this->`: in a class template, the handle is in a dependent base.
        args.push("this->h".into());
    }
    let mut params = Vec::new();
    let mut refs = BTreeSet::new();
    for (pname, t) in params_slice {
        // `w_`; `h`, reserved, is already `h_`: two underscores are the implementation's.
        let base = ident(pname);
        let pname = if base.ends_with('_') { base } else { format!("{base}_") };
        params.push(format!("{} {pname}", ty_param(t, unions, callbacks)));
        raw_params.extend(raw(t));
        args.extend(lower_param(t, &pname, unions, callbacks));
        named_types(t, &mut refs);
    }
    if omit_rest {
        raw_params.push("uint32_t");
        args.push("0".into());
        if let Some((_, last_t)) = f.params.last() {
            named_types(last_t, &mut refs);
        }
    } else if rest_pack.is_some() {
        raw_params.push("uint32_t");
        args.push("rest_arr__.h".into());
        if let Some((_, last_t)) = f.params.last() {
            named_types(last_t, &mut refs);
        }
    }
    named_types(&f.ret, &mut refs);
    // A shared import takes the index of this function's fixed arguments after `self`.
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(Class::has_self(f));
            raw_params.insert(at, "int32_t");
            args.insert(at, k.to_string());
            shared.clone()
        }
        None => f.import.clone(),
    };
    let raw_symbol = raw_fn(raw_prefix, &import);
    let import_decl = format!("extern \"C\" {} {} {raw_symbol}({});\n", import_attr(wasm_module, &import), raw_ret(&f.ret), raw_params.join(", "));
    let call = format!("{raw_symbol}({})", args.join(", "));
    let fluent = place == Place::Member && matches!(f.call, Call::Set(_) | Call::IndexSet) && f.ret == Ty::Void;
    let body_call = if fluent {
        format!("{call}; return *this;")
    } else {
        match &f.ret {
            Ty::Void => format!("{call};"),
            t => format!("auto r = {call}; return {};", raise(t, "r", unions)),
        }
    };
    let body = if let Some(elem_ty) = rest_pack {
        format!("auto rest_arr__ = ts::Array<{elem_ty}>::of(std::forward<R0__>(r0__), std::forward<Rest__>(rest__)...); {body_call}")
    } else {
        body_call
    };

    let has_key = params_slice.iter().any(|p| matches!(p.1, Ty::Key(_)));
    let mut type_params = f.type_params.clone();
    if has_key {
        type_params.insert(0, TParam { name: "V".into(), default: None, constraint: None });
    }
    for (i, p) in type_params.iter_mut().enumerate() {
        if !(has_key && i == 0)
            && p.default.is_none()
            && p.constraint.as_ref().and_then(|c| constraint(&p.name, c)).is_none()
            && !params_slice.iter().any(|(_, t)| is_deducible(&p.name, t, unions, callbacks))
        {
            p.default = Some(Ty::Any);
        }
    }

    if rest_pack.is_some() {
        params.push("R0__&& r0__".into());
        params.push("Rest__&&... rest__".into());
    }
    let extra_params: [&str; 2] = ["class R0__", "class... Rest__"];
    let extra_requires = if let Some(elem_ty) = rest_pack {
        vec![
            format!("!(sizeof...(Rest__) == 0 && std::is_base_of_v<ts::Array<{elem_ty}>, std::decay_t<R0__>>)"),
            format!("ts::rest_accepts<{elem_ty}, R0__>"),
            format!("(ts::rest_accepts<{elem_ty}, Rest__> && ...)"),
        ]
    } else {
        Vec::new()
    };

    // Defaults where the function is first declared: in the class, or at a free function.
    let (first, again) = if rest_pack.is_some() {
        (
            head_with_extra(&type_params, true, &extra_params, &extra_requires),
            head_with_extra(&type_params, false, &extra_params, &extra_requires),
        )
    } else {
        (head_func(&type_params, true), head_func(&type_params, false))
    };
    let templated = !owner_head.is_empty() || !type_params.is_empty() || rest_pack.is_some();
    let inline = if templated { "inline " } else { "" };
    let ret = if fluent { format!("const {owner}&") } else { ty_out(&f.ret, unions) };
    let params_str = params.join(", ");
    let (decl, def) = match place {
        Place::Member => (
            format!("    {}{ret} {name}({params_str}) const;\n", first.replace('\n', " ")),
            format!("{owner_head}{again}{inline}{ret} {owner}::{name}({params_str}) const {{ {body} }}\n"),
        ),
        Place::Static => (
            format!("    {}static {ret} {name}({params_str});\n", first.replace('\n', " ")),
            format!("{owner_head}{again}{inline}{ret} {owner}::{name}({params_str}) {{ {body} }}\n"),
        ),
        Place::Free if templated => (String::new(), format!("{first}inline {ret} {name}({params_str}) {{ {body} }}\n")),
        Place::Free => (
            format!("{ret} {name}({params_str});\n"),
            format!("{ret} {name}({params_str}) {{ {body} }}\n"),
        ),
    };
    // With its template head: an overload constrained and one not are two,
    // the more constrained chosen where both fit (`merge<T extends Dictionary>`).
    // Its template parameters by place: C++ tells none apart by name.
    let again_key = head_func(&type_params, false);
    let key = if let Some(elem_ty) = rest_pack {
        format!(
            "{}{}{name}({})|rest<{elem_ty}>",
            again_key.replace('\n', " "),
            if place == Place::Static { "static " } else { "" },
            params_slice.iter().map(|p| ty_param(&p.1, unions, callbacks)).collect::<Vec<_>>().join(",")
        )
    } else {
        format!(
            "{}{}{name}({})",
            again_key.replace('\n', " "),
            if place == Place::Static { "static " } else { "" },
            params_slice.iter().map(|p| ty_param(&p.1, unions, callbacks)).collect::<Vec<_>>().join(",")
        )
    };
    let key = rename_words(&key, &type_params.iter().enumerate().map(|(i, p)| (p.name.clone(), format!("${i}"))).collect());
    Function { import: import_decl, decl, def, templated, key, refs }
}

#[allow(clippy::too_many_arguments)]
fn function_variants(
    f: &Func,
    name: String,
    zero_name: Option<String>,
    owner: &str,
    owner_head: &str,
    place: Place,
    unions: &[NamedUnion],
    callbacks: &HashSet<&str>,
    wasm_module: &str,
    raw_prefix: &str,
) -> Vec<Function> {
    let mut out = vec![build_function(f, &name, owner, owner_head, place, unions, callbacks, wasm_module, raw_prefix, &f.params, None, false)];
    if f.rest
        && let Some((_, last_t)) = f.params.last()
        && matches!(last_t, Ty::Array | Ty::ArrayOf(_))
    {
        let elem_ty = match last_t {
            Ty::Array => "ts::Value".to_string(),
            Ty::ArrayOf(e) => ty_arg_for("", e),
            _ => unreachable!(),
        };
        let fixed = &f.params[..f.params.len() - 1];
        let zname = zero_name.as_deref().unwrap_or(&name);
        out.push(build_function(f, zname, owner, owner_head, place, unions, callbacks, wasm_module, raw_prefix, fixed, None, true));
        out.push(build_function(f, &name, owner, owner_head, place, unions, callbacks, wasm_module, raw_prefix, fixed, Some(&elem_ty), false));
    }
    out
}

// The declared types a type names: classes, callbacks, unions and maps.
fn named_types(t: &Ty, out: &mut BTreeSet<String>) {
    match t {
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => {
            out.insert(n.clone());
        }
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) => named_types(inner, out),
        Ty::Tuple(es) => es.iter().for_each(|e| named_types(e, out)),
        Ty::Generic(n, args) => {
            out.insert(n.clone());
            args.iter().for_each(|a| named_types(a, out));
        }
        _ => {}
    }
}

fn param_types(params: &[TParam], out: &mut BTreeSet<String>) {
    for p in params {
        p.default.iter().chain(&p.constraint).for_each(|t| named_types(t, out));
    }
}

const RUNTIME: &str = include_str!("runtime/cpp_runtime.hpp");

// The bindings as files: see the top of this file.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    files_named(m, "bindings", &[])
}

pub fn files_named(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<Vec<(String, String)>, Vec<String>> {
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let ns: Option<&str> = (pkg_name != "bindings").then_some(pkg_name);
    let ns_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}::") };
    let inc_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}/") };
    NS_PREFIX.with(|p| *p.borrow_mut() = ns_prefix.clone());

    let unions = &m.unions;
    DECODABLE.with(|d| *d.borrow_mut() = unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect());
    let resolved_opaque: HashMap<String, (usize, String)> = m
        .opaque
        .iter()
        .filter(|n| !is_builtin_cpp_type(n))
        .filter(|n| m.opaque_arity.get(*n).copied().unwrap_or(0) == 0)
        .filter_map(|n| {
            let orig = m.opaque_names.get(n).map(String::as_str).unwrap_or(n.as_str());
            refs.iter().enumerate().find(|(_, r)| r.types.contains(orig)).map(|(i, _)| ((*n).clone(), (i, orig.to_string())))
        })
        .collect();
    RESOLVED_REF.with(|r| *r.borrow_mut() = resolved_opaque.keys().cloned().collect());
    // A constructor is named as its class, and hides nothing.
    let members: BTreeSet<String> = m.classes.iter().flat_map(|c| c.funcs.iter().filter(|f| !matches!(f.call, Call::New(_) | Call::Create(_))).map(|f| cpp_name(f, &c.name))).collect();
    QUALIFIED.with(|q| *q.borrow_mut() = members.into_iter().filter(|n| m.classes.iter().any(|c| c.name == *n) || m.callbacks.iter().any(|c| c.name == *n) || m.unions.iter().any(|u| u.name == *n) || m.enums.iter().any(|e| e.0 == *n) || m.num_enums.iter().any(|e| e.0 == *n)).collect());
    let mut taken = HashMap::new();
    // Where each declared type is defined, without the extension.
    let mut home: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        home.insert(c.name.clone(), super::model::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
    }
    for name in m.callbacks.iter().map(|c| &c.name).chain(m.unions.iter().map(|u| &u.name)) {
        home.insert(name.clone(), super::model::file_name(&format!("types/{name}"), &mut taken));
    }
    // `#include` from one generated file to another, by its path from the
    // bindings' root (compiled with `-I` on it): relative to the including
    // file, a long chain of includes makes paths too long for the system,
    // as the compiler joins them without resolving `..`.
    let include = |_from: &str, to: &str| format!("#include \"{inc_prefix}{to}\"\n");
    // The headers of the given types.
    // What a union holds, by name: a value of it needs each of them complete.
    let union_parts: HashMap<&str, BTreeSet<String>> = unions
        .iter()
        .map(|u| {
            let mut parts = BTreeSet::new();
            u.parts.iter().for_each(|p| named_types(p, &mut parts));
            (u.name.as_str(), parts)
        })
        .collect();
    let classes: BTreeSet<&str> = m.classes.iter().map(|c| c.name.as_str()).collect();
    // First the structs of the classes used, and of those the unions used
    // hold: declarations include only declarations, so each is complete
    // before any definition, however the full headers include each other.
    // Then the full headers.
    let includes = |from: &str, types: &BTreeSet<String>| -> String {
        let mut all = types.clone();
        let mut todo: Vec<String> = types.iter().cloned().collect();
        while let Some(t) = todo.pop() {
            for p in union_parts.get(t.as_str()).into_iter().flatten() {
                if all.insert(p.clone()) {
                    todo.push(p.clone());
                }
            }
        }
        let decls = all.iter().filter(|t| classes.contains(t.as_str())).filter_map(|t| home.get(t)).map(|h| format!("{h}.decl.hpp"));
        let full = types.iter().filter_map(|t| home.get(t)).map(|h| format!("{h}.hpp"));
        decls.chain(full).filter(|h| h != from).map(|h| include(from, &h)).collect()
    };

    // A namespace's types, also by their own names in it.
    let (namespaces, in_structs) = namespace_aliases(m, &ns_prefix);
    // A namespace's functions where it is a class's: its statics, by their own names.
    let containers = namespace_containers(m);
    let levels = level_names(m);
    let mut class_statics: HashMap<String, Vec<Func>> = HashMap::new();
    let mut class_statics_of: HashSet<String> = HashSet::new();
    for f in &m.funcs {
        if let Some((path, own, _, _)) = m.func_namespaces.get(&f.import)
            && path.len() == 1
            && let Some(Container::Class(c)) = containers.get(&path[0])
        {
            class_statics.entry(c.clone()).or_default().push(Func { name: own.clone(), ..(*f).clone() });
            class_statics_of.insert(f.import.clone());
        }
    }
    let types = declarations(m, pkg_name, refs, &resolved_opaque) + &wrap_ns(ns, &namespaces);
    let mut out = vec![("ts_runtime.hpp".to_string(), RUNTIME.to_string()), ("ts_types.hpp".to_string(), types)];

    // A union: its alternatives, and how to tell which one came back.
    for u in unions {
        let path = format!("{}.hpp", home[&u.name]);
        let mut parts = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut parts));
        parts.remove(&u.name);
        let tag_raw = raw_fn(&raw_prefix, &format!("{}_tag", u.name));
        let tag_attr = import_attr(wasm_module, &format!("{}_tag", u.name));
        let mut text = format!("// Generated.\n#pragma once\n{}{}", include(&path, "ts_types.hpp"), includes(&path, &parts));
        text += &format!("extern \"C\" {tag_attr} int32_t {tag_raw}(uint32_t, int32_t);\n");
        let mut decode_ns = String::new();
        let mut decoder_ext = String::new();
        if u.decodable() {
            let name = union_type(u);
            let cases: Vec<String> = u
                .parts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let value = match p {
                        Ty::Num => "ts::from_handle<double>(h)".to_string(),
                        Ty::Bool => "ts::from_handle<bool>(h)".to_string(),
                        Ty::Enum(e) => format!("ts::from_handle<{e}>(h)"),
                        p => raise(p, "h", unions),
                    };
                    format!("        case {i}: return {name}(std::in_place_index<{i}>, {value});\n")
                })
                .collect();
            // A bare type parameter is told apart by its argument; with another
            // member no test can recognise, only a number, string or flag can be.
            let param = u.parts.iter().find_map(|p| if let Ty::Param(t) = p { Some(t.clone()) } else { None });
            let kind = param.as_ref().map_or("0".to_string(), |t| format!("ts::kind_of<{t}>"));
            let check = match &param {
                Some(t) if u.tests.iter().zip(&u.parts).any(|(test, p)| test.is_none() && !matches!(p, Ty::Param(_))) => {
                    format!("    static_assert(ts::kind_of<{t}> != 0, \"{t} must be a number, string or flag to be told apart\");\n")
                }
                _ => String::new(),
            };
            decode_ns += &format!(
                "{}inline {name} decode_{}(uint32_t h) {{\n{check}    switch ({tag_raw}(h, {kind})) {{\n{2}    }}\n    __builtin_trap();\n}}\n",
                head(&u.type_params, false),
                u.name,
                cases.concat()
            );
            if decoder_owner(u, unions) {
                let guard = decoder_guard_id(u, m, pkg_name, &resolved_opaque, refs);
                decoder_ext += &format!(
                    "#ifndef TS_DECODER_DEF_{guard}\n#define TS_DECODER_DEF_{guard}\n\
                     inline ::{ns_prefix}{0} ts::decoder<::{ns_prefix}{0}>::from(uint32_t h) {{ return ::{ns_prefix}decode_{0}(h); }}\n\
                     #endif\n",
                    u.name
                );
            }
        }
        text += &wrap_ns(ns, &decode_ns);
        text += &decoder_ext;
        out.push((path, text));
    }
    // Per module: headers, and the definitions compiled once.
    let mut modules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut library: BTreeMap<String, (BTreeSet<String>, String)> = BTreeMap::new();
    let callbacks: HashSet<&str> = m.callbacks.iter().map(|c| c.name.as_str()).collect();

    // A callback, made from a lambda. Its arguments are made only once the
    // lambda's type is known, so the types they name need not be complete yet.
    for c in &m.callbacks {
        let path = format!("{}.hpp", home[&c.name]);
        let mut used = BTreeSet::new();
        c.params.iter().for_each(|p| named_types(&p.1, &mut used));
        named_types(&c.ret, &mut used);
        used.remove(&c.name);
        let raw_args: String = c.params.iter().enumerate().map(|(i, (_, t))| format!(", {} a{i}", raw_ret(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| raise_callback(t, &format!("a{i}"), unions)).collect();
        let call = format!("ts::closure_fn<F>(ctx)({})", args.join(", "));
        let result = match &c.ret {
            Ty::Void => format!("{call};"),
            t => format!("return {};", lower_callback(t, &call)),
        };
        let cb_args: Vec<String> = c.params.iter().map(|(_, t)| format!("ts::dep<F, {}>", ty_out(t, unions))).collect();
        let arg_list = if cb_args.is_empty() { String::new() } else { format!(", {}", cb_args.join(", ")) };
        let req = if c.ret == Ty::Void {
            format!("ts::callable_candidate<F> && std::invocable<F&{arg_list}>")
        } else {
            format!("ts::callable_candidate<F> && std::invocable<F&{arg_list}> && !std::is_void_v<std::invoke_result_t<F&{arg_list}>>")
        };
        let (name, ret) = (&c.name, raw_ret(&c.ret));
        let tpl = head(&c.type_params, false);
        let owner = if c.type_params.is_empty() {
            c.name.clone()
        } else {
            format!("{}<{}>", c.name, c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", "))
        };
        let cb_from = format!("{name}_from");
        let cb_raw = raw_fn(&raw_prefix, &cb_from);
        let cb_attr = import_attr(wasm_module, &cb_from);
        let mut cb_imports = BTreeSet::new();
        cb_imports.insert(format!("extern \"C\" {cb_attr} uint32_t {cb_raw}(uintptr_t, void*);\n"));
        let mut op_decls = String::new();
        let mut op_header_defs = String::new();
        let mut op_seen = BTreeSet::new();
        for f in m.funcs.iter().filter(|f| {
            f.call == Call::Apply
                && f.params.first().is_some_and(|(_, t)| matches!(t, Ty::Obj(n) | Ty::Generic(n, _) if n == &c.name))
        }) {
            let mut op_f = f.clone();
            op_f.call = Call::Invoke;
            op_f.params = f.params[1..].to_vec();
            op_f.type_params = f.type_params.get(c.type_params.len()..).unwrap_or(&[]).to_vec();
            for func in function_variants(&op_f, "operator()".into(), None, &owner, &tpl, Place::Member, unions, &callbacks, wasm_module, &raw_prefix) {
                if !op_seen.insert(func.key) {
                    continue;
                }
                op_decls += &func.decl;
                used.extend(func.refs);
                for t in &op_f.type_params {
                    param_types(std::slice::from_ref(t), &mut used);
                }
                if func.templated {
                    op_header_defs += &func.def;
                    cb_imports.insert(func.import);
                } else {
                    let lib = library.entry(f.module.clone()).or_default();
                    lib.0.insert(func.import);
                    lib.1 += &func.def;
                }
            }
        }
        used.remove(&c.name);
        let body = wrap_ns(
            ns,
            &format!(
                "{tpl}struct {name} : ts::Callback {{\n{}    \
                 template <class F>\n        requires ({req})\n    \
                 {name}(F f) : ts::Handle(ts::handle, {cb_raw}(reinterpret_cast<uintptr_t>(&call<F>), ts::closure(std::move(f)))) {{}}\n\
                 {op_decls}\n  \
                 private:\n    template <class F>\n    static {ret} call(void* ctx{raw_args}) {{ {result} }}\n}};\n",
                constructors(name),
            ),
        );
        let mut text = format!(
            "// Generated.\n#pragma once\n{}{}\n{body}\n{}",
            include(&path, "ts_types.hpp"),
            cb_imports.into_iter().collect::<String>(),
            includes(&path, &used)
        );
        if !op_header_defs.is_empty() {
            text += &format!("\n{}", wrap_ns(ns, &op_header_defs));
        }
        out.push((path, text));
    }

    // Each class: its struct, its header, and its definitions.
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
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| cpp_name(f, "create")).collect() };
    for c in &m.classes {
        let base = &home[&c.name];
        let (decl_path, impl_path) = (format!("{base}.decl.hpp"), format!("{base}.hpp"));
        // A class template is written `Box<T>` where its members are defined.
        let class_head = head(&c.type_params, false);
        let owner = if c.type_params.is_empty() { c.name.clone() } else { format!("{}<{}>", c.name, c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        // A member named `create` leaves the constructors `make`.
        let ctor = if c.funcs.iter().any(|f| !matches!(f.call, Call::New(_) | Call::Create(_)) && cpp_name(f, "") == "create") { "make" } else { "create" };

        // A name in two unrelated bases is ambiguous in C++. A more derived
        // ancestor's member already hides its own ancestors' (`Element::textContent`
        // over `Node::textContent`), as in TypeScript; the others are declared
        // again here, with the base's type arguments, and hide both.
        let all = ancestors(&c.name);
        let own = member_names(c);
        let mut holders: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for a in &all {
            if let Some(k) = m.classes.iter().find(|k| k.name == *a) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(a);
                }
            }
        }
        let mut again: Vec<Func> = Vec::new();
        // A name the type declares itself hides its ancestors' overloads in C++,
        // where TypeScript merges them: `using Base::name;` brings them back, and
        // an own overload of the same signature still hides its base's.
        let clashes_adapted = |bf: &Func, cf: &Func| -> bool {
            if bf.params.len() != cf.params.len() {
                return false;
            }
            let mut diff = false;
            for ((_, bt), (_, ct)) in bf.params.iter().zip(&cf.params) {
                let b_ty = ty_param(bt, unions, &callbacks);
                let c_ty = ty_param(ct, unions, &callbacks);
                if b_ty == c_ty {
                    continue;
                }
                let b_ad = is_adapted_param(bt, unions, &callbacks);
                let c_ad = is_adapted_param(ct, unions, &callbacks);
                if (b_ad && c_ad) || (b_ad && *ct == Ty::Any) || (c_ad && *bt == Ty::Any) {
                    diff = true;
                } else {
                    return false;
                }
            }
            diff
        };
        let mut usings = in_structs.get(&c.name).cloned().unwrap_or_default();
        for (name, hs) in &holders {
            let visible: Vec<&&String> = hs.iter().filter(|h| !hs.iter().any(|o| o != *h && ancestors(o).contains(**h))).collect();
            if own.contains(name) && visible.len() == 1 {
                let h = visible[0];
                let k = m.classes.iter().find(|k| k.name == **h).unwrap();
                let args = base_args(m, c, h).unwrap_or_default();
                let base_funcs: Vec<Func> = k
                    .funcs
                    .iter()
                    .filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name)
                    .map(|f| map_types(&with_self_args(f, k), &|t| subst(t, &k.type_params, &args)))
                    .collect();
                let own_funcs: Vec<Func> = c
                    .funcs
                    .iter()
                    .filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name)
                    .map(|f| with_self_args(f, c))
                    .collect();
                if base_funcs.iter().any(|bf| own_funcs.iter().any(|cf| clashes_adapted(bf, cf))) {
                    for bf in base_funcs {
                        if !own_funcs.iter().any(|cf| clashes_adapted(&bf, cf)) {
                            again.push(bf);
                        }
                    }
                    continue;
                }
                let base = match args.is_empty() {
                    false => ty_in(&Ty::Generic((*h).clone(), args)),
                    true => (*h).clone(),
                };
                usings += &format!("    using {base}::{};\n", ident(name));
                if name == "invoke" {
                    usings += &format!("    using {base}::operator();\n");
                }
                continue;
            }
            if visible.len() < 2 {
                continue;
            }
            for h in visible {
                let k = m.classes.iter().find(|k| k.name == **h).unwrap();
                let args = base_args(m, c, h).unwrap_or_default();
                for f in k.funcs.iter().filter(|f| Class::has_self(f) && cpp_name(f, "create") == *name) {
                    let f = with_self_args(f, k);
                    again.push(map_types(&f, &|t| subst(t, &k.type_params, &args)));
                }
            }
        }

        let mut seen = BTreeSet::new();
        let mut members = String::new();
        let mut header_defs = String::new();
        let mut header_imports = BTreeSet::new();
        let mut used = BTreeSet::new();
        param_types(&c.type_params, &mut used);
        let lib = library.entry(c.module.clone()).or_default();
        // C++ cannot overload a static member with a non-static one of the
        // same parameters, as TypeScript can: the static one says it is.
        let sig_of = |name: &str, ps: &[(String, Ty)]| format!("{name}({})", ps.iter().map(|p| ty_param(&p.1, unions, &callbacks)).collect::<Vec<_>>().join(","));
        let signature = |f: &Func| sig_of(&cpp_name(f, ctor), &f.params);
        let mut instance = BTreeSet::new();
        for f in c.funcs.iter().map(|f| with_self_args(f, c)).chain(again.iter().cloned()).filter(Class::has_self) {
            instance.insert(signature(&f));
            if f.rest && matches!(f.params.last().map(|p| &p.1), Some(Ty::Array | Ty::ArrayOf(_))) {
                instance.insert(sig_of(&cpp_name(&f, ctor), &f.params[..f.params.len() - 1]));
            }
        }
        // The functions of the namespace it shares its name with, as its statics.
        let namespaced: Vec<Func> = class_statics.get(&c.name).cloned().unwrap_or_default();
        let all_funcs: Vec<Func> = c.funcs.iter().map(|f| with_self_args(f, c)).chain(again).chain(namespaced).collect();
        for (idx, f) in all_funcs.iter().enumerate() {
            let raw_name = cpp_name(f, ctor);
            if all_funcs.iter().enumerate().any(|(j, g)| {
                j != idx && func_subsumed_by(f, g, &raw_name, &cpp_name(g, ctor), unions, &callbacks)
            }) {
                continue;
            }
            let place = if Class::has_self(f) { Place::Member } else { Place::Static };
            let name = if place == Place::Static && instance.contains(&signature(f)) { format!("{raw_name}_static") } else { raw_name };
            let zero_name = if place == Place::Static
                && f.rest
                && matches!(f.params.last().map(|p| &p.1), Some(Ty::Array | Ty::ArrayOf(_)))
                && instance.contains(&sig_of(&name, &f.params[..f.params.len() - 1]))
            {
                Some(format!("{name}_static"))
            } else {
                None
            };
            let mut emit_names = vec![name];
            if place == Place::Member && f.call == Call::Invoke && emit_names[0] == "invoke" {
                emit_names.push("operator()".to_string());
            }
            for emit_name in emit_names {
                for func in function_variants(f, emit_name, zero_name.clone(), &owner, &class_head, place, unions, &callbacks, wasm_module, &raw_prefix) {
                    // Overloads TypeScript tells apart and C++ cannot are the same C++ function.
                    if !seen.insert(func.key) {
                        continue;
                    }
                    members += &func.decl;
                    used.extend(func.refs);
                    for t in &f.type_params {
                        param_types(std::slice::from_ref(t), &mut used);
                    }
                    if func.templated {
                        header_defs += &func.def;
                        header_imports.insert(func.import);
                    } else {
                        lib.1 += &func.def;
                        lib.0.insert(func.import);
                    }
                }
            }
        }
        for (name, value) in &c.consts {
            let ty = if value.starts_with('"') { "std::string_view" } else { "double" };
            members += &format!("    static constexpr {ty} {} = {value};\n", ident(name));
        }
        // Covariant conversion for generic classes: `Shape<RectConfig>` -> `Shape<ShapeConfig>`.
        if !c.type_params.is_empty() {
            let u_names: Vec<String> = (0..c.type_params.len()).map(|i| format!("U{i}_")).collect();
            let u_decls: Vec<String> = u_names.iter().map(|u| format!("class {u}")).collect();
            let same: Vec<String> = c.type_params.iter().zip(&u_names).map(|(p, u)| format!("std::same_as<{}, {u}>", ident(&p.name))).collect();
            let mut reqs: Vec<String> = vec![format!("!({})", same.join(" && "))];
            for (p, u) in c.type_params.iter().zip(&u_names) {
                reqs.push(format!("ts::extends<{}, {u}>", ident(&p.name)));
            }
            let u_args_ty: Vec<Ty> = u_names.iter().map(|u| Ty::Param(u.clone())).collect();
            for (p, u) in c.type_params.iter().zip(&u_names) {
                if let Some(con) = &p.constraint {
                    let sub_con = subst(con, &c.type_params, &u_args_ty);
                    reqs.push(format!("ts::extends<{u}, {}>", ty_arg(&sub_con)));
                }
            }
            let target = format!("{}<{}>", ident(&c.name), u_names.join(", "));
            members += &format!(
                "    template <{}>\n        requires ({})\n    operator {target}() const {{ if (this->h) ts_raw_js_retain(this->h); return {target}(ts::handle, this->h); }}\n",
                u_decls.join(", "),
                reqs.join(" && ")
            );
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if own.contains("cast") || holders.contains_key("cast") || c.consts.iter().any(|(n, _)| n == "cast") || class_statics.get(&c.name).is_some_and(|fs| fs.iter().any(|f| cpp_name(f, ctor) == "cast")) {
                "try_cast"
            } else {
                "cast"
            };
            let is_name = format!("js_is__{}", c.name);
            let is_raw = raw_fn(&raw_prefix, &is_name);
            let is_attr = import_attr(wasm_module, &is_name);
            let is_import = format!("extern \"C\" {is_attr} int32_t {is_raw}(uint32_t);\n");
            members += &format!("    static std::optional<{owner}> {cast_name}(const ts::Handle& v);\n");
            let def = format!(
                "{class_head}{}std::optional<{owner}> {owner}::{cast_name}(const ts::Handle& v) {{ if (v.h != 0 && {is_raw}(v.h)) {{ ts_raw_js_retain(v.h); return {owner}(ts::handle, v.h); }} return std::nullopt; }}\n",
                if class_head.is_empty() { "" } else { "inline " },
            );
            if !class_head.is_empty() {
                header_defs += &def;
                header_imports.insert(is_import);
            } else {
                lib.1 += &def;
                lib.0.insert(is_import);
            }
        }
        // Iterable: `for (Node n : list)`.
        if let Some(elem) = &c.iterates
            && !own.contains("begin")
            && !own.contains("end")
        {
            let it = format!("ts::Iterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret));
            members += &format!("    {it} begin() const;\n    ts::IterationEnd end() const {{ return {{}}; }}\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::begin() const {{ return {it}(ts_raw_js_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        if let Some(elem) = &c.iterates_async
            && !own.contains("iterate_async")
        {
            let it = format!("ts::AsyncIterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create(String::new()), params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret));
            members += &format!("    {it} iterate_async() const;\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::iterate_async() const {{ return {it}(ts::handle, ts_raw_js_async_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        let bases: Vec<String> = if c.bases.is_empty() { vec!["virtual ts::Handle".into()] } else { c.bases.iter().map(|b| format!("virtual {}", ty_in(b))).collect() };
        let mut decl = format!("// Generated.\n#pragma once\n{}", include(&decl_path, "ts_types.hpp"));
        for a in &c.ancestors {
            // A class's declaration; a callback's (a call signature) one file.
            let class = m.classes.iter().any(|k| k.name == *a);
            if let Some(h) = home.get(a) {
                decl += &include(&decl_path, &format!("{h}{}", if class { ".decl.hpp" } else { ".hpp" }));
            }
        }
        let decl_body = format!(
            "{class_head}struct {0} : {1} {{\n{2}{usings}{members}}};\n",
            c.name,
            bases.join(", "),
            constructors(&c.name)
        );
        decl += &format!("\n{}", wrap_ns(ns, &decl_body));
        used.remove(&c.name);
        used.extend(c.ancestors.iter().cloned());
        let mut imp = format!("// Generated.\n#pragma once\n{}{}", include(&impl_path, &decl_path), includes(&impl_path, &used));
        if !header_defs.is_empty() {
            imp += &format!("\n{}\n{}", header_imports.into_iter().collect::<String>(), wrap_ns(ns, &header_defs));
        }
        modules.entry(c.module.clone()).or_default().push(impl_path.clone());
        out.push((decl_path, decl));
        out.push((impl_path, imp));
    }
    let mut functions: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>, String, String)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let free_funcs: Vec<(Option<String>, String, &Func)> = m
        .funcs
        .iter()
        .filter(|f| !class_statics_of.contains(&f.import))
        .map(|f| {
            // A namespace's, by its own name in a namespace of it: `marked_ns::parseInline`.
            let space = m.func_namespaces.get(&f.import).map(|(path, _, _, _)| {
                let top = match &containers.get(&path[0]) {
                    Some(Container::Namespace(n)) => n.clone(),
                    _ => format!("{}_ns", ident(&path[0])),
                };
                let mut parts = vec![top];
                for i in 1..path.len() {
                    parts.push(nested_name(&levels, &path[..i], &path[i]));
                }
                parts.join("::")
            });
            let name = m.func_namespaces.get(&f.import).map_or_else(|| cpp_name(f, "create"), |(_, own, _, _)| cpp_name_as(f, own, "create"));
            (space, name, f)
        })
        .collect();
    for (idx, (space, name, f)) in free_funcs.iter().enumerate() {
        if free_funcs.iter().enumerate().any(|(j, (os, on, g))| {
            j != idx && os == space && func_subsumed_by(f, g, name, on, unions, &callbacks)
        }) {
            continue;
        }
        for mut func in function_variants(f, name.clone(), None, "", "", Place::Free, unions, &callbacks, wasm_module, &raw_prefix) {
            if let Some(space) = &space {
                let wrap = |text: &str| if text.is_empty() { String::new() } else { format!("namespace {space} {{\n{text}}}\n") };
                func.key = format!("{space}::{}", func.key);
                func.decl = wrap(&func.decl);
                func.def = wrap(&func.def);
            }
            if !seen.insert(func.key) {
                continue;
            }
            let entry = functions.entry(f.module.clone()).or_default();
            entry.1.extend(func.refs);
            f.type_params.iter().for_each(|t| param_types(std::slice::from_ref(t), &mut entry.1));
            entry.2 += &func.decl;
            if func.templated {
                entry.0.insert(func.import);
                entry.3 += &func.def;
            } else {
                let lib = library.entry(f.module.clone()).or_default();
                lib.0.insert(func.import);
                lib.1 += &func.def;
            }
        }
    }
    for (module, (imports, used, decls, defs)) in functions {
        let path = format!("{}.hpp", super::model::file_name(&format!("{module}/functions"), &mut taken));
        let body = wrap_ns(ns, &format!("{decls}\n{defs}"));
        let text = format!("// Generated.\n#pragma once\n{}{}\n{}\n{body}", include(&path, "ts_types.hpp"), includes(&path, &used), imports.into_iter().collect::<String>());
        out.push((path.clone(), text));
        modules.entry(module).or_default().push(path);
    }
    // The definitions of each module: a translation unit to compile once.
    for (module, (imports, defs)) in library {
        let path = format!("{module}.cpp");
        let headers: String = modules[&module].iter().map(|h| include(&path, h)).collect();
        let body = wrap_ns(ns, &defs);
        out.push((path, format!("// Generated. The module's definitions: compile once, and link with the program.\n{headers}\n{}\n{body}", imports.into_iter().collect::<String>())));
    }
    let mut all = format!("// Generated.\n#pragma once\n#include \"{inc_prefix}ts_types.hpp\"\n");
    for (module, headers) in &modules {
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.hpp"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{inc_prefix}{module}.hpp\"\n");
    }
    out.push(("bindings.hpp".into(), all.clone()));
    if pkg_name != "bindings" {
        out.push((format!("{pkg_name}.hpp"), all));
    }
    let meta_types: Vec<String> = m
        .classes
        .iter()
        .filter(|c| c.type_params.is_empty())
        .map(|c| c.name.as_str())
        .chain(m.callbacks.iter().filter(|c| c.type_params.is_empty()).map(|c| c.name.as_str()))
        .chain(m.enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.num_enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.unions.iter().filter(|u| u.type_params.is_empty()).map(|u| u.name.as_str()))
        .chain(resolved_opaque.keys().map(String::as_str))
        .filter(|n| !n.is_empty() && !n.contains(':') && !n.starts_with("ts_bases_of_") && !is_builtin_cpp_type(n))
        .map(str::to_string)
        .collect();
    let entry = if pkg_name != "bindings" { format!("{pkg_name}.hpp") } else { "bindings.hpp".into() };
    let meta = super::pkg_meta::PkgMeta {
        target: "cpp".into(),
        package: pkg_name.to_string(),
        entry: Some(entry),
        types: meta_types,
        ..Default::default()
    };
    out.push((super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok(out)
}

// Whether `u` declares the decoder of its variant: unions with the same
// members are the same type, and the first one does.
fn decoder_owner(u: &NamedUnion, unions: &[NamedUnion]) -> bool {
    // As C++ sees it, through other unions' aliases: `ts::Array<OptionId>`
    // and `ts::Array<OrdinalRawValue>` are one type where both are one variant.
    let aliases: HashMap<&str, String> = unions.iter().filter(|o| o.type_params.is_empty()).map(|o| (o.name.as_str(), format!("std::variant<{}>", o.parts.iter().map(|p| ty_out(p, unions)).collect::<Vec<_>>().join(", ")))).collect();
    let expand = |text: String| {
        let mut out = text;
        for _ in 0..8 {
            let mut next = String::new();
            let mut word = String::new();
            for c in out.chars().chain(['\0']) {
                if c.is_alphanumeric() || c == '_' {
                    word.push(c);
                    continue;
                }
                next += aliases.get(word.as_str()).map_or(word.as_str(), String::as_str);
                word.clear();
                if c != '\0' {
                    next.push(c);
                }
            }
            if next == out {
                break;
            }
            out = next;
        }
        out
    };
    let key = |u: &NamedUnion| u.parts.iter().map(|p| expand(ty_out(p, unions))).collect::<Vec<_>>();
    u.type_params.is_empty() && u.decodable() && unions.iter().find(|o| o.type_params.is_empty() && o.decodable() && key(o) == key(u)).is_some_and(|o| o.name == u.name)
}

fn decoder_guard_id(u: &NamedUnion, m: &Model, pkg_name: &str, resolved_opaque: &HashMap<String, (usize, String)>, refs: &[ProjectRef]) -> String {
    let unions = &m.unions;
    let aliases: HashMap<&str, String> = unions
        .iter()
        .filter(|o| o.type_params.is_empty())
        .map(|o| (o.name.as_str(), format!("std::variant<{}>", o.parts.iter().map(|p| ty_out(p, unions)).collect::<Vec<_>>().join(", "))))
        .collect();
    let mut expanded = format!("std::variant<{}>", u.parts.iter().map(|p| ty_out(p, unions)).collect::<Vec<_>>().join(", "));
    for _ in 0..8 {
        let mut next = String::new();
        let mut word = String::new();
        for c in expanded.chars().chain(['\0']) {
            if c.is_alphanumeric() || c == '_' {
                word.push(c);
                continue;
            }
            next += aliases.get(word.as_str()).map_or(word.as_str(), String::as_str);
            word.clear();
            if c != '\0' {
                next.push(c);
            }
        }
        if next == expanded {
            break;
        }
        expanded = next;
    }
    let local_types: HashSet<&str> = m
        .classes
        .iter()
        .map(|c| c.name.as_str())
        .chain(m.callbacks.iter().map(|c| c.name.as_str()))
        .chain(m.enums.iter().map(|e| e.0.as_str()))
        .chain(m.num_enums.iter().map(|e| e.0.as_str()))
        .chain(m.opaque.iter().map(String::as_str).filter(|n| !matches!(*n, "Promise" | "PromiseLike")))
        .collect();
    let mut canonical = String::new();
    let mut word = String::new();
    let mut preceded_by_colon = false;
    for c in expanded.chars().chain(['\0']) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        if !word.is_empty() {
            let tok = word.as_str();
            let is_std = matches!(tok, "std" | "variant" | "string" | "optional" | "double" | "bool" | "ts" | "Array" | "Tuple" | "Value" | "Promise" | "PromiseLike");
            if preceded_by_colon || is_std {
                canonical.push_str(tok);
            } else if let Some((idx, orig)) = resolved_opaque.get(tok) {
                canonical += &format!("::{}::{orig}", refs[*idx].pkg_name);
            } else if pkg_name != "bindings" && local_types.contains(tok) {
                canonical += &format!("::{pkg_name}::{tok}");
            } else {
                canonical.push_str(tok);
            }
            word.clear();
        }
        if c != '\0' {
            preceded_by_colon = c == ':';
            canonical.push(c);
        }
    }
    format!("{:016x}", crate::fnv1a64(&canonical))
}

// A union as a type in its own definitions: `Either` or `Either<T>`.
fn union_type(u: &NamedUnion) -> String {
    if u.type_params.is_empty() { u.name.clone() } else { format!("{}<{}>", u.name, u.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

// `ts_types.hpp`: every type declared, so a struct may name any of them
// before it is defined. A template comes after what its defaults and
// constraints name.
fn declarations(m: &Model, pkg_name: &str, refs: &[ProjectRef], resolved_opaque: &HashMap<String, (usize, String)>) -> String {
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let raw_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}_") };
    let ns: Option<&str> = (pkg_name != "bindings").then_some(pkg_name);
    let ns_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}::") };
    let inc_prefix = if pkg_name == "bindings" { String::new() } else { format!("{pkg_name}/") };
    let used_refs: BTreeSet<usize> = resolved_opaque.values().map(|(i, _)| *i).collect();

    let unions = &m.unions;
    let mut types = format!("// Generated.\n#pragma once\n#include \"{inc_prefix}ts_runtime.hpp\"\n");
    for &idx in &used_refs {
        types += &format!("#include \"{}\"\n", refs[idx].umbrella_include);
    }
    types.push('\n');
    let enum_val_raw = raw_fn(&raw_prefix, "js_enum_value");
    let enum_idx_raw = raw_fn(&raw_prefix, "js_enum_index");
    if !m.enums.is_empty() {
        types += &format!(
            "extern \"C\" {{\n{} uint32_t {enum_val_raw}(int32_t, int32_t);\n{} int32_t {enum_idx_raw}(int32_t, uint32_t);\n}}\n\n",
            import_attr(wasm_module, "js_enum_value"),
            import_attr(wasm_module, "js_enum_index"),
        );
    }
    if let Some(pkg) = ns {
        types += &format!("namespace {pkg} {{\n");
    }
    let mut enum_specs = String::new();
    for (id, (name, values)) in m.enums.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| ident(if v.is_empty() { "empty" } else { v })).map(|v| if seen.insert(v.clone()) { v } else { format!("{v}_{}", seen.len()) }).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
        enum_specs += &format!(
            "template <>\ninline constexpr int32_t ts::enum_id<::{ns_prefix}{name}> = {id};\n\
             template <>\ninline uint32_t ts::enum_to_handle<::{ns_prefix}{name}>(::{ns_prefix}{name} x) {{ return {enum_val_raw}({id}, int32_t(x)); }}\n\
             template <>\ninline ::{ns_prefix}{name} ts::enum_from_handle<::{ns_prefix}{name}>(uint32_t h) {{ return ::{ns_prefix}{name}({enum_idx_raw}({id}, h)); }}\n"
        );
    }
    if !enum_specs.is_empty() {
        if let Some(pkg) = ns {
            types += &format!("}}  // namespace {pkg}\n{enum_specs}namespace {pkg} {{\n");
        } else {
            types += &enum_specs;
        }
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    let opaque = |name: &str, params: &[TParam]| {
        let mut body = String::from("    ");
        body += &format!("{name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n");
        if name == "Error" {
            body += "    Error(const ts::Error& e) : ts::Handle(e) {}\n\
                     static Error create(std::string_view msg) { return Error(ts::handle, ts_raw_js_error_new(msg.data(), msg.size())); }\n\
                     std::string name() const { return ts::from_handle<std::string>(ts_raw_js_error_name(this->h)); }\n\
                     std::string message() const { return ts::from_handle<std::string>(ts_raw_js_error_message(this->h)); }\n\
                     std::string stack() const { return ts::from_handle<std::string>(ts_raw_js_error_stack(this->h)); }\n";
        } else if name == "Date" {
            body += "    static Date from_time(double ms) { return Date(ts::handle, ts_raw_js_date_new(ms)); }\n\
                     static double now() { return ts_raw_js_date_now(); }\n\
                     double time() const { return ts_raw_js_date_time(this->h); }\n\
                     std::string to_iso_string() const { return ts::from_handle<std::string>(ts_raw_js_date_to_iso_string(this->h)); }\n";
        } else if let Some(s) = super::model::buffer_spec(name) {
            body += "    uint32_t byte_length() const { return ts_raw_js_bytes_len(this->h); }\n\
                     uint32_t read_bytes(void* dst, uint32_t len, uint32_t offset = 0) const { return ts_raw_js_bytes_read(this->h, offset, dst, len); }\n\
                     uint32_t write_bytes(const void* src, uint32_t len, uint32_t offset = 0) const { return ts_raw_js_bytes_write(this->h, offset, src, len); }\n";
            if s.has_buffer() {
                body += "    uint32_t byte_offset() const { return ts_raw_js_bytes_offset(this->h); }\n\
                         ts::Value buffer() const { return ts::Value(ts::handle, ts_raw_js_bytes_buffer(this->h)); }\n";
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = super::c::c_elem(s.elem);
                let eb = s.elem.bytes();
                body += &format!(
                    "    static {name} create(size_t count) {{ uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return {name}(ts::handle, ts_raw_js_bytes_new({kind}, nullptr, n)); }}\n\
                     static {name} from(const {elem}* data, size_t count) {{ uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return {name}(ts::handle, ts_raw_js_bytes_new({kind}, data, n)); }}\n\
                     static {name} from(const std::vector<{elem}>& data) {{ return from(data.data(), data.size()); }}\n\
                     size_t size() const {{ return size_t(byte_length()) / {eb}; }}\n\
                     size_t read({elem}* dst, size_t count, size_t elem_offset = 0) const {{ if (elem_offset > (UINT32_MAX / {eb}u)) return 0; uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return size_t(ts_raw_js_bytes_read(this->h, uint32_t(elem_offset * {eb}u), dst, n)) / {eb}; }}\n\
                     size_t write(const {elem}* src, size_t count, size_t elem_offset = 0) const {{ if (elem_offset > (UINT32_MAX / {eb}u)) return 0; uint32_t n = count > (UINT32_MAX / {eb}u) ? UINT32_MAX : uint32_t(count * {eb}u); return size_t(ts_raw_js_bytes_write(this->h, uint32_t(elem_offset * {eb}u), src, n)) / {eb}; }}\n\
                     std::vector<{elem}> to_vector() const {{ std::vector<{elem}> out(size()); if (!out.empty()) read(out.data(), out.size()); return out; }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    body += &format!(
                        "    static {name} create(uint32_t byte_len) {{ return {name}(ts::handle, ts_raw_js_bytes_new({kind}, nullptr, byte_len)); }}\n\
                         static {name} from(const void* data, uint32_t byte_len) {{ return {name}(ts::handle, ts_raw_js_bytes_new({kind}, data, byte_len)); }}\n\
                         static {name} from(const std::vector<uint8_t>& data) {{ return from(data.data(), uint32_t(data.size())); }}\n"
                    );
                }
                body += "    uint32_t read(void* dst, uint32_t len, uint32_t offset = 0) const { return read_bytes(dst, len, offset); }\n\
                         uint32_t write(const void* src, uint32_t len, uint32_t offset = 0) const { return write_bytes(src, len, offset); }\n\
                         std::vector<uint8_t> to_vector() const { std::vector<uint8_t> out(byte_length()); if (!out.empty()) read_bytes(out.data(), uint32_t(out.size())); return out; }\n";
            }
        }
        format!("{}struct {name} : virtual ts::Handle {{\n{body}}};\n", head(params, true))
    };
    // (name, declaration, what it needs declared first)
    let mut templates: Vec<(String, String, BTreeSet<String>)> = Vec::new();
    let mut template = |name: &str, params: &[TParam], text: String| {
        let mut needs = BTreeSet::new();
        param_types(params, &mut needs);
        needs.remove(name);
        templates.push((name.to_string(), text, needs));
    };
    for c in &m.classes {
        let text = format!("{}struct {};\n", head(&c.type_params, true), c.name);
        if c.type_params.is_empty() { types += &text } else { template(&c.name, &c.type_params, text) }
    }
    let mut cb_specs = String::new();
    for c in &m.callbacks {
        let decl = format!("{}struct {};\n", head(&c.type_params, true), c.name);
        if c.type_params.is_empty() {
            types += &decl;
            cb_specs += &format!("template <>\ninline constexpr bool ts::is_callback<::{ns_prefix}{}> = true;\n", c.name);
        } else {
            let ps: Vec<String> = (0..c.type_params.len()).map(|i| format!("B{i}__")).collect();
            let spec_head = format!("template <{}>\n", ps.iter().map(|p| format!("class {p}")).collect::<Vec<_>>().join(", "));
            let spec_args = ps.join(", ");
            let spec = format!(
                "{spec_head}inline constexpr bool ts::is_callback<::{ns_prefix}{}<{spec_args}>> = true;\n",
                c.name
            );
            let text = if let Some(pkg) = ns {
                format!("{decl}}}  // namespace {pkg}\n{spec}namespace {pkg} {{\n")
            } else {
                format!("{decl}{spec}")
            };
            template(&c.name, &c.type_params, text);
        }
    }
    if !cb_specs.is_empty() {
        if let Some(pkg) = ns {
            types += &format!("}}  // namespace {pkg}\n{cb_specs}namespace {pkg} {{\n");
        } else {
            types += &cb_specs;
        }
    }
    // Types declared elsewhere: handles, generic or not.
    // Promises are the runtime's.
    for name in m.opaque.iter().filter(|n| matches!(n.as_str(), "Promise" | "PromiseLike")) {
        types += &format!("template <class T = void>\nusing {name} = ts::Promise<T>;\n");
    }
    for name in m.opaque.iter().filter(|n| !matches!(n.as_str(), "Promise" | "PromiseLike")) {
        if let Some((idx, orig)) = resolved_opaque.get(name) {
            types += &format!("using {name} = ::{}::{orig};\n", refs[*idx].pkg_name);
            continue;
        }
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<TParam> = (1..=arity).map(|i| TParam { name: format!("T{i}"), default: Some(Ty::Any), constraint: None }).collect();
        let mark = if is_builtin_cpp_type(name) { "" } else { "// external\n" };
        if params.is_empty() { types += &format!("{mark}{}", opaque(name, &params)) } else { template(name, &params, format!("{mark}{}", opaque(name, &params))) }
    }
    // Unions, with the templates: either may name the other, in a default
    // argument (`Interval<StartType = DateArg<Date>>`) or a member (`Box<T>`).
    for u in unions {
        let parts: Vec<String> = u.parts.iter().map(|p| ty_out(p, unions)).collect();
        let mut text = format!("{}using {} = std::variant<{}>;\n", head(&u.type_params, true), u.name, parts.join(", "));
        // Defined with the union, used wherever one comes back.
        if u.decodable() {
            text += &format!("{}{} decode_{}(uint32_t h);\n", head(&u.type_params, false), union_type(u), u.name);
            // Unions with the same members are the same variant: one decoder.
            if decoder_owner(u, unions) {
                let guard = decoder_guard_id(u, m, pkg_name, resolved_opaque, refs);
                let spec = format!(
                    "#ifndef TS_DECODER_{guard}\n#define TS_DECODER_{guard}\n\
                     template <>\nstruct ts::decoder<::{ns_prefix}{0}> {{\n    static ::{ns_prefix}{0} from(uint32_t h);\n}};\n\
                     #endif\n",
                    u.name
                );
                if let Some(pkg) = ns {
                    text += &format!("}}  // namespace {pkg}\n{spec}namespace {pkg} {{\n");
                } else {
                    text += &spec;
                }
            }
        }
        let mut needs = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut needs));
        param_types(&u.type_params, &mut needs);
        needs.remove(&u.name);
        // Naming a constrained template (`Path<PathProps>`) checks its
        // constraint there: the bases of what it names come first.
        let bases: Vec<String> = needs.iter().filter(|n| m.classes.iter().any(|c| c.name == **n && !c.bases.is_empty())).map(|n| format!("bases:{n}")).collect();
        needs.extend(bases);
        templates.push((u.name.clone(), text, needs));
    }
    // What each type extends, for constraints: see `ts::bases`. After the
    // types its bases name, theirs first.
    for c in m.classes.iter().filter(|c| !c.bases.is_empty()) {
        let base_list = c.bases.iter().map(ty_in).collect::<Vec<_>>().join(", ");
        let tpl_head = if c.type_params.is_empty() {
            String::new()
        } else {
            format!("template <{}>\n", c.type_params.iter().map(|p| format!("class {}", p.name)).collect::<Vec<_>>().join(", "))
        };
        let (spec_head, spec_args) = if c.type_params.is_empty() {
            ("template <>\n".to_string(), String::new())
        } else {
            let ps: Vec<String> = (0..c.type_params.len()).map(|i| format!("B{i}__")).collect();
            (
                format!("template <{}>\n", ps.iter().map(|p| format!("class {p}")).collect::<Vec<_>>().join(", ")),
                format!("<{}>", ps.join(", ")),
            )
        };
        let text = if let Some(pkg) = ns {
            format!(
                "{tpl_head}using ts_bases_of_{0}_ = ts::list<{base_list}>;\n\
                 }}  // namespace {pkg}\n\
                 {spec_head}struct ts::bases<::{pkg}::{0}{spec_args}> {{\n    using type = ::{pkg}::ts_bases_of_{0}_{spec_args};\n}};\n\
                 namespace {pkg} {{\n",
                c.name
            )
        } else {
            format!(
                "{tpl_head}using ts_bases_of_{0}_ = ts::list<{base_list}>;\n\
                 {spec_head}struct ts::bases<::{0}{spec_args}> {{\n    using type = ::ts_bases_of_{0}_{spec_args};\n}};\n",
                c.name
            )
        };
        let mut named = BTreeSet::new();
        c.bases.iter().for_each(|b| named_types(b, &mut named));
        let mut needs: BTreeSet<String> = named.iter().filter(|n| **n != c.name && m.classes.iter().any(|k| k.name == **n && !k.bases.is_empty())).map(|n| format!("bases:{n}")).collect();
        needs.extend(named.into_iter().filter(|n| *n != c.name));
        // A template is specialized once declared.
        if !c.type_params.is_empty() {
            needs.insert(c.name.clone());
        }
        templates.push((format!("bases:{}", c.name), text, needs));
    }
    let names: BTreeSet<String> = templates.iter().map(|t| t.0.clone()).collect();
    let mut done = BTreeSet::new();
    while !templates.is_empty() {
        let ready = templates.iter().position(|(_, _, needs)| needs.iter().all(|n| done.contains(n) || !names.contains(n))).unwrap_or(0);
        let (name, text, _) = templates.remove(ready);
        types += &text;
        done.insert(name);
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    inline constexpr ts::Key<{}, {}> {}{{{i}}};\n", k.name, ty_out(v, unions), ident(key))).collect();
        types += &format!("namespace {}Keys {{\n{}}}\n", k.name, keys.concat());
    }
    if let Some(pkg) = ns {
        types += &format!("}}  // namespace {pkg}\n");
    }
    types
}

// A callback argument. Objects are made as a type that depends on the
// lambda's, so they need be complete only where a lambda is passed.
fn raise_callback(t: &Ty, a: &str, unions: &[NamedUnion]) -> String {
    match t {
        Ty::Num | Ty::NumEnum(_) | Ty::Bool | Ty::Enum(_) | Ty::Str => raise(t, a, unions),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => raise(t, a, unions),
        // A union that can be decoded is the variant; any other a value.
        Ty::NamedUnion(n) | Ty::Generic(n, _) if decodable(n, unions) => format!("ts::from_handle<ts::dep<F, {}>>({a})", ty_out(t, unions)),
        Ty::NamedUnion(_) | Ty::Union(_) | Ty::Generic(..) if !matches!(t, Ty::Generic(n, _) if !is_union(n, unions)) => format!("ts::Value(ts::handle, {a})"),
        t => format!("ts::from_handle<ts::dep<F, {}>>({a})", ty_out(t, unions)),
    }
}

// What a callback returns: a number, a flag, or a handle.
fn lower_callback(t: &Ty, x: &str) -> String {
    match t {
        Ty::Num => x.into(),
        Ty::NumEnum(_) => format!("double({x})"),
        Ty::Bool => format!("({x} ? 1 : 0)"),
        Ty::Enum(_) => format!("int32_t({x})"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("[](std::optional<bool> v) {{ return v ? int32_t(*v) : -1; }}({x})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("[](auto v) {{ return v ? int32_t(*v) : -1; }}({x})"),
        _ => format!("ts::Arg({x}).give()"),
    }
}

// `text` with each whole word in `to` replaced.
fn rename_words(text: &str, to: &HashMap<String, String>) -> String {
    let mut out = String::new();
    let mut word = String::new();
    for c in text.chars().chain(['\0']) {
        if c.is_alphanumeric() || c == '_' {
            word.push(c);
            continue;
        }
        out += to.get(&word).map_or(word.as_str(), String::as_str);
        word.clear();
        if c != '\0' {
            out.push(c);
        }
    }
    out
}

// A namespace's types by their own names in it, as aliases of theirs:
// `Intl.Locale` is `Intl::Locale` too, a namespace of aliases after every
// type is declared. A namespace a class shares its name with (node's
// `Stream`) is in that class: `Stream::PipeOptions`. One a function or a
// generic class shares its name with is `<name>_ns`, as C++ allows a name
// one kind of thing in a scope. Returns the namespaces, and each class's
// aliases.
fn namespace_aliases(m: &Model, ns_prefix: &str) -> (String, HashMap<String, String>) {
    let params_of = |n: &str| -> Option<Vec<TParam>> {
        m.classes.iter().find(|c| c.name == n).map(|c| c.type_params.clone())
            .or_else(|| m.callbacks.iter().find(|c| c.name == n).map(|c| c.type_params.clone()))
            .or_else(|| m.unions.iter().find(|u| u.name == n).map(|u| u.type_params.clone()))
            .or_else(|| (m.enums.iter().any(|e| e.0 == n) || m.num_enums.iter().any(|e| e.0 == n)).then(Vec::new))
    };
    // (namespaces, own name, the type): every namespaced type C++ has.
    let mut entries: Vec<(Vec<String>, String, String)> = Vec::new();
    for (name, path) in &m.namespaced {
        if params_of(name).is_some() {
            entries.push((path[..path.len() - 1].to_vec(), path.last().unwrap().clone(), name.clone()));
        }
    }
    entries.sort();
    let mut taken: HashSet<String> = m.classes.iter().map(|c| c.name.clone()).chain(m.callbacks.iter().map(|c| c.name.clone())).chain(m.unions.iter().map(|u| u.name.clone())).collect();
    taken.extend(m.enums.iter().map(|e| e.0.clone()).chain(m.num_enums.iter().map(|e| e.0.clone())).chain(m.opaque.iter().cloned()));
    taken.extend(m.funcs.iter().map(|f| cpp_name(f, "create")).chain(m.maps.iter().map(|k| format!("{}Keys", k.name))).chain(["ts".to_string()]));
    let alias = |leaf: &str, target: &str| -> String {
        let params = params_of(target).unwrap_or_default();
        let args = if params.is_empty() { String::new() } else { format!("<{}>", params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        format!("{}using {} = ::{ns_prefix}{target}{args};\n", head(&params, true).replace('\n', " "), ident(leaf))
    };
    // One level: its own types' aliases, then each namespace in it.
    let levels = level_names(m);
    fn level(entries: &[(Vec<String>, String, String)], at: &[String], kind: &str, indent: &str, alias: &dyn Fn(&str, &str) -> String, levels: &HashMap<Vec<String>, HashSet<String>>) -> String {
        let mut out = String::new();
        let own: Vec<&String> = entries.iter().filter(|(p, _, _)| p == at).map(|(_, l, _)| l).collect();
        // Each name once (node's `cluster` has a class and an interface `Worker`).
        let mut seen: HashSet<String> = HashSet::new();
        for (_, leaf, target) in entries.iter().filter(|(p, _, _)| p == at) {
            let mut name = leaf.clone();
            let mut i = 2;
            while !seen.insert(name.clone()) {
                name = format!("{leaf}_{i}");
                i += 1;
            }
            out += &format!("{indent}{}", alias(&name, target));
        }
        let children: BTreeSet<&String> = entries.iter().filter(|(p, _, _)| p.len() > at.len() && p[..at.len()] == *at).map(|(p, _, _)| &p[at.len()]).collect();
        for child in children {
            let mut path = at.to_vec();
            path.push(child.clone());
            // Beside a type or function of its name it is apart.
            let name = if own.contains(&child) || levels.get(at).is_some_and(|n| n.contains(&ident(child))) { format!("{}_ns", ident(child)) } else { ident(child) };
            let inner = level(entries, &path, kind, &format!("{indent}    "), alias, levels);
            out += &if kind == "struct" { format!("{indent}struct {name} {{\n{inner}{indent}}};\n") } else { format!("{indent}namespace {name} {{\n{inner}{indent}}}\n") };
        }
        out
    }
    let mut namespaces = String::new();
    let mut in_structs: HashMap<String, String> = HashMap::new();
    let _ = taken;
    let tops: BTreeSet<&String> = entries.iter().map(|(p, _, _)| &p[0]).collect();
    let containers = namespace_containers(m);
    for top in tops {
        match &containers[top] {
            Container::Class(c) => {
                in_structs.insert(c.clone(), level(&entries, std::slice::from_ref(top), "struct", "    ", &alias, &levels));
            }
            Container::Namespace(name) => {
                namespaces += &format!("namespace {name} {{\n{}}}\n", level(&entries, std::slice::from_ref(top), "namespace", "    ", &alias, &levels));
            }
        }
    }
    (namespaces, in_structs)
}

// The names directly in each namespace: its types' and functions' own.
fn level_names(m: &Model) -> HashMap<Vec<String>, HashSet<String>> {
    let mut out: HashMap<Vec<String>, HashSet<String>> = HashMap::new();
    for path in m.namespaced.values() {
        out.entry(path[..path.len() - 1].to_vec()).or_default().insert(ident(path.last().unwrap()));
    }
    for (path, own, _, _) in m.func_namespaces.values() {
        out.entry(path.clone()).or_default().insert(ident(own));
    }
    out
}

// A nested namespace's name: `<name>_ns` where its parent has something of
// its name (`events`' class `EventEmitter` beside its namespace of functions).
fn nested_name(levels: &HashMap<Vec<String>, HashSet<String>>, parent: &[String], name: &str) -> String {
    if levels.get(parent).is_some_and(|n| n.contains(&ident(name))) { format!("{}_ns", ident(name)) } else { ident(name) }
}

// Where a top-level namespace's types and functions are: in the class it
// shares its name with (node's `Stream`: `Stream::Duplex`,
// `Stream::finished`), but for a generic one (`Control<O>::Zoom` would want
// its arguments) or one a member of which has a name of theirs; else in a
// namespace of its name, `<name>_ns` where something else has it (marked's
// function `marked`), as C++ allows a name one kind of thing in a scope.
#[derive(Clone, PartialEq)]
enum Container {
    Class(String),
    Namespace(String),
}

fn namespace_containers(m: &Model) -> HashMap<String, Container> {
    // Each top-level namespace's own names, directly in it.
    let mut own: BTreeMap<String, HashSet<String>> = BTreeMap::new();
    for path in m.namespaced.values() {
        let e = own.entry(path[0].clone()).or_default();
        if path.len() == 2 {
            e.insert(ident(&path[1]));
        }
    }
    for (path, leaf, _, _) in m.func_namespaces.values() {
        let e = own.entry(path[0].clone()).or_default();
        if path.len() == 1 {
            e.insert(ident(leaf));
        }
    }
    let mut taken: HashSet<String> = m.classes.iter().map(|c| c.name.clone()).chain(m.callbacks.iter().map(|c| c.name.clone())).chain(m.unions.iter().map(|u| u.name.clone())).collect();
    taken.extend(m.enums.iter().map(|e| e.0.clone()).chain(m.num_enums.iter().map(|e| e.0.clone())).chain(m.opaque.iter().cloned()));
    taken.extend(m.funcs.iter().filter(|f| !m.func_namespaces.contains_key(&f.import)).map(|f| cpp_name(f, "create")).chain(m.maps.iter().map(|k| format!("{}Keys", k.name))).chain(["ts".to_string()]));
    let mut out = HashMap::new();
    for (top, names) in own {
        let class = m.classes.iter().find(|c| c.name == top && !m.namespaced.contains_key(&c.name));
        let members: HashSet<String> = class.map(|c| c.funcs.iter().map(|f| cpp_name(f, &c.name)).collect()).unwrap_or_default();
        let clashes = names.iter().any(|n| members.contains(n) || *n == top);
        let container = match class {
            Some(c) if c.type_params.is_empty() && !clashes => Container::Class(c.name.clone()),
            _ => Container::Namespace(if taken.contains(&top) { format!("{}_ns", ident(&top)) } else { ident(&top) }),
        };
        out.insert(top, container);
    }
    out
}
