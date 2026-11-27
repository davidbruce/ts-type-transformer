use std::collections::{BTreeSet, HashSet};

use super::decl::rename_words;
use super::{ident, import_attr, raw_fn, Ctx};
use super::super::comments::{Style, comment};
use super::super::model::{base_args, map_types, subst, Call, Class, Func, Model, NamedUnion, TParam, Ty};

// C++ type of a value going in. `V` is the value a typed key selects.
pub(super) fn ty_in(t: &Ty, cx: &Ctx) -> String {
    ty_in_as(t, cx, false)
}

// As `ty_in`, where a value only goes in: a union no test tells apart is typed, in an array or a tuple.
pub(super) fn ty_put(t: &Ty, cx: &Ctx) -> String {
    ty_in_as(t, cx, true)
}

fn ty_in_as(t: &Ty, cx: &Ctx, put: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "std::string_view".into(),
        Ty::Any => "ts::Value".into(),
        Ty::Array => "ts::Array<>".into(),
        Ty::ArrayOf(e) => format!("ts::Array<{}>", ty_arg_as("", e, cx, put)),
        Ty::Record(v) => format!("ts::Record<{}>", ty_arg_as("", v, cx, put)),
        Ty::Tuple(es) => format!("ts::Tuple<{}>", es.iter().map(|e| ty_arg_as("", e, cx, put)).collect::<Vec<_>>().join(", ")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => cx.tn(n),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_)) => format!("std::optional<{}>", ty_in_as(inner, cx, put).trim_start_matches("const ").trim_end_matches('&')),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_in_as(inner, cx, put)),
        Ty::NamedUnion(n) => format!("const {}&", cx.tn(n)),
        Ty::Union(_) => "ts::Value".into(),
        Ty::Key(m) => format!("ts::Key<{m}, V>"),
        Ty::Mapped(_) => "V".into(),
        // Not deduced from the lambda: the key says what the listener takes.
        Ty::Listener(ev) => format!("std::type_identity_t<ts::Listener<{}>>", ty_in(ev, cx)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, args) => format!("{}<{}>", cx.tn(n), args.iter().map(|a| ty_arg_as(n, a, cx, put)).collect::<Vec<_>>().join(", ")),
    }
}

pub(super) fn is_adapted_param(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> bool {
    match t {
        Ty::NamedUnion(_) => true,
        Ty::Generic(n, _) => is_union(n, unions),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(_) | Ty::Tuple(_) | Ty::Record(_) => true,
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
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => contains_param(inner),
        Ty::Tuple(es) | Ty::Union(es) => es.iter().any(contains_param),
        Ty::Generic(_, args) => args.iter().any(contains_param),
        _ => false,
    }
}

// C++ type of a function/method parameter, using input adapters (`ts::In`/`ts::Opt`,
// or `ts::Exact`/`ts::OptExact` for a superset union parameter that clashes with a subset overload).
fn ty_param_with_exact(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>, exact: bool, cx: &Ctx) -> String {
    let wrap = |inner: String| {
        if contains_param(t) {
            format!("const std::type_identity_t<{inner}>&")
        } else {
            format!("const {inner}&")
        }
    };
    let in_tpl = if exact { "ts::Exact" } else { "ts::In" };
    let opt_tpl = if exact { "ts::OptExact" } else { "ts::Opt" };
    match t {
        Ty::NamedUnion(n) => wrap(format!("{in_tpl}<{}>", cx.tn(n))),
        Ty::Generic(n, _) if is_union(n, unions) => wrap(format!("{in_tpl}<{}>", ty_in(t, cx))),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => wrap(format!("{opt_tpl}<{}>", cx.tn(n))),
            Ty::Generic(n, _) if is_union(n, unions) => wrap(format!("{opt_tpl}<{}>", ty_in(inner, cx))),
            Ty::Generic(n, _) if callbacks.contains(n.as_str()) => wrap(format!("ts::Opt<{}>", ty_in(inner, cx))),
            Ty::Obj(n) if callbacks.contains(n.as_str()) => wrap(format!("ts::Opt<{}>", ty_in(inner, cx))),
            Ty::Tuple(_) | Ty::Record(_) => wrap(format!("ts::Opt<{}>", ty_in(inner, cx))),
            _ => ty_put(t, cx),
        },
        _ => ty_put(t, cx),
    }
}

pub(super) fn ty_param(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>, cx: &Ctx) -> String {
    ty_param_with_exact(t, unions, callbacks, false, cx)
}

// The setters an object made in one expression names: its ancestors' before its own, each as declared,
// with the type arguments it gives them. None for a type with methods.
pub(super) fn bag_setters(m: &Model, c: &Class) -> Option<Vec<Func>> {
    fn walk<'a>(m: &'a Model, c: &'a Class, seen: &mut Vec<&'a str>, order: &mut Vec<&'a Class>) -> bool {
        if c.funcs.iter().any(|f| matches!(f.call, Call::Method(_))) {
            return false;
        }
        seen.push(&c.name);
        for a in &c.ancestors {
            if !seen.contains(&a.as_str()) && let Some(k) = m.classes.iter().find(|k| k.name == *a) && !walk(m, k, seen, order) {
                return false;
            }
        }
        order.push(c);
        true
    }
    let mut order = Vec::new();
    if !walk(m, c, &mut Vec::new(), &mut order) {
        return None;
    }
    let mut out: Vec<Func> = Vec::new();
    for k in order {
        let args = if k.name == c.name { k.type_params.iter().map(|p| Ty::Param(p.name.clone())).collect() } else { base_args(m, c, &k.name).or_else(|| k.type_params.is_empty().then(Vec::new))? };
        let own: Vec<Func> = k.funcs.iter().filter(|f| matches!(f.call, Call::Set(_))).map(|f| map_types(f, &|t| subst(t, &k.type_params, &args))).collect();
        // A property it declares again is where it is declared last.
        out.retain(|f| !own.iter().any(|o| o.call == f.call));
        out.extend(own);
    }
    Some(out)
}

// The type of a bag's field: what the setter takes, left out or given, or nothing where it is not typed.
pub(super) fn bag_field(t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>, cx: &Ctx) -> Option<String> {
    if matches!(t, Ty::Void | Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_)) {
        return None;
    }
    let p = ty_param(t, unions, callbacks, cx);
    let p = p.strip_prefix("const ").and_then(|p| p.strip_suffix('&')).unwrap_or(&p);
    Some(format!("ts::Field<{}>", p.strip_prefix("std::optional<").and_then(|p| p.strip_suffix('>')).unwrap_or(p)))
}

fn union_subsumed(sub_t: &Ty, sup_t: &Ty, unions: &[NamedUnion]) -> bool {
    match (sub_t, sup_t) {
        (Ty::NamedUnion(a), Ty::NamedUnion(b)) if a != b => {
            let Some(ua) = unions.iter().find(|u| u.name == *a && u.type_params.is_empty()) else { return false; };
            let Some(ub) = unions.iter().find(|u| u.name == *b && u.type_params.is_empty()) else { return false; };
            (ua.parts.len() < ub.parts.len() || (ua.parts.len() == ub.parts.len() && a > b))
                && ua.parts.iter().all(|p| ub.parts.contains(p))
        }
        (Ty::NamedUnion(a), Ty::Nullable(b, _)) => matches!(&**b, Ty::NamedUnion(bn) if a == bn) || union_subsumed(sub_t, b, unions),
        (Ty::Nullable(a, _), Ty::Nullable(b, _)) => union_subsumed(a, b, unions),
        _ => false,
    }
}

pub(super) fn func_subsumed_by(
    sub: &Func,
    sup: &Func,
    sub_name: &str,
    sup_name: &str,
    unions: &[NamedUnion],
    callbacks: &HashSet<&str>,
    cx: &Ctx,
) -> bool {
    if sub_name != sup_name
        || Class::has_self(sub) != Class::has_self(sup)
        || sub.params.len() != sup.params.len()
        || !sub.type_params.is_empty()
        || !sup.type_params.is_empty()
        || sub.fixed != sup.fixed
        || sub.call != sup.call
        || sub.rest != sup.rest
        || ty_out(&sub.ret, unions, cx) != ty_out(&sup.ret, unions, cx)
    {
        return false;
    }
    let mut strict = false;
    for ((_, st), (_, pt)) in sub.params.iter().zip(&sup.params) {
        if ty_param(st, unions, callbacks, cx) == ty_param(pt, unions, callbacks, cx) {
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

/// Returns parameter indices of `sup` that strictly subsume a sibling overload
/// in `peers` and should accept only the exact union type (`ts::Exact`/`ts::OptExact`).
pub(super) fn exact_params_for<'a>(
    sup: &Func,
    sup_name: &str,
    peers: impl Iterator<Item = (&'a Func, &'a str)>,
    unions: &[NamedUnion],
    callbacks: &HashSet<&str>,
    cx: &Ctx,
) -> HashSet<usize> {
    let mut out = HashSet::new();
    if !sup.type_params.is_empty() {
        return out;
    }
    for (sub, sub_name) in peers {
        if sub_name != sup_name
            || Class::has_self(sub) != Class::has_self(sup)
            || sub.params.len() != sup.params.len()
            || !sub.type_params.is_empty()
            || sub.rest != sup.rest
        {
            continue;
        }
        let mut subsumed_at = Vec::new();
        let mut clashes = true;
        for (i, ((_, st), (_, pt))) in sub.params.iter().zip(&sup.params).enumerate() {
            if ty_param(st, unions, callbacks, cx) == ty_param(pt, unions, callbacks, cx) {
                continue;
            }
            if union_subsumed(st, pt, unions) {
                subsumed_at.push(i);
            } else {
                clashes = false;
                break;
            }
        }
        if clashes && !subsumed_at.is_empty() {
            out.extend(subsumed_at);
        }
    }
    out
}

fn lower_param(t: &Ty, x: &str, unions: &[NamedUnion], callbacks: &HashSet<&str>, cx: &Ctx) -> Vec<String> {
    if is_adapted_param(t, unions, callbacks) {
        vec![format!("{x}.h")]
    } else {
        lower(t, x, cx)
    }
}

fn is_deducible(param: &str, t: &Ty, unions: &[NamedUnion], callbacks: &HashSet<&str>) -> bool {
    match t {
        Ty::Param(n) => n == param,
        Ty::ArrayOf(e) | Ty::Record(e) => is_deducible(param, e, unions, callbacks),
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

fn head_func(params: &[TParam], defaults: bool, cx: &Ctx) -> String {
    head_with_extra(params, defaults, &[], &[], cx)
}

fn head_with_extra(params: &[TParam], defaults: bool, extra_params: &[&str], extra_requires: &[String], cx: &Ctx) -> String {
    if params.is_empty() && extra_params.is_empty() {
        return String::new();
    }
    let mut ps: Vec<String> = params
        .iter()
        .map(|p| match &p.default {
            Some(d) if defaults => format!("class {} = {}", p.name, ty_arg_for("", d, cx)),
            _ => format!("class {}", p.name),
        })
        .collect();
    ps.extend(extra_params.iter().map(|s| (*s).to_string()));
    let mut requires: Vec<String> = params.iter().filter_map(|p| constraint(&p.name, p.constraint.as_ref()?, cx)).collect();
    requires.extend(extra_requires.iter().cloned());
    let requires = if requires.is_empty() { String::new() } else { format!(" requires ({})", requires.join(" && ")) };
    format!("template <{}>{requires}\n", ps.join(", "))
}

// A type as a template argument: a value type, not a view.
pub(super) fn ty_arg(t: &Ty, cx: &Ctx) -> String {
    ty_arg_in(t, cx, false)
}

fn ty_arg_in(t: &Ty, cx: &Ctx, put: bool) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_arg_in(inner, cx, put)),
        // A union a handle can be decoded to; any other stays a value, but where one only goes in.
        Ty::NamedUnion(n) if cx.decodable.contains(n) || (put && cx.typable.contains(n)) => cx.tn(n),
        Ty::NamedUnion(_) | Ty::Union(_) => "ts::Value".into(),
        t => ty_in_as(t, cx, put),
    }
}

fn ty_arg_for(owner: &str, t: &Ty, cx: &Ctx) -> String {
    ty_arg_as(owner, t, cx, false)
}

fn ty_arg_as(owner: &str, t: &Ty, cx: &Ctx, put: bool) -> String {
    if *t == Ty::Void && !matches!(owner, "Promise" | "PromiseLike") {
        "ts::Value".into()
    } else {
        ty_arg_in(t, cx, put)
    }
}

// C++ type of a value coming back.
pub(super) fn ty_out(t: &Ty, unions: &[NamedUnion], cx: &Ctx) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_out(inner, unions, cx)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n, unions) && !decodable(n, unions) => "ts::Value".into(),
        Ty::NamedUnion(n) => cx.tn(n),
        t => ty_in(t, cx),
    }
}

pub(super) fn is_union(n: &str, unions: &[NamedUnion]) -> bool {
    unions.iter().any(|u| u.name == n)
}

pub(super) fn decodable(n: &str, unions: &[NamedUnion]) -> bool {
    unions.iter().any(|u| u.name == n && u.decodable())
}

// `template <class E = Element> requires ts::extends<E, Element>`, or nothing.
// Defaults are given only where a template is first declared, and only if
// every later parameter has one too.
pub(super) fn head(params: &[TParam], defaults: bool, cx: &Ctx) -> String {
    if params.is_empty() {
        return String::new();
    }
    let trailing = params.iter().skip_while(|p| p.default.is_none()).all(|p| p.default.is_some());
    let ps: Vec<String> = params
        .iter()
        .map(|p| match &p.default {
            Some(d) if defaults && trailing => format!("class {} = {}", p.name, ty_arg_for("", d, cx)),
            _ => format!("class {}", p.name),
        })
        .collect();
    let requires: Vec<String> = params.iter().filter_map(|p| constraint(&p.name, p.constraint.as_ref()?, cx)).collect();
    // In parentheses: a type written `::T` after it would join it.
    let requires = if requires.is_empty() { String::new() } else { format!(" requires ({})", requires.join(" && ")) };
    format!("template <{}>{requires}\n", ps.join(", "))
}

// A type's constructors: none (`X(nullptr)`), or from a handle.
// With its assignments: the handle, a virtual base by several paths, is
// assigned once (an implicit move assignment may move it once per path, the
// second from what the first emptied). Where JavaScript makes one of nothing
// (`new Scene()`), `Scene()` is not a null handle: the default is a base's only.
pub(super) fn constructors(name: &str, made_of_nothing: bool) -> String {
    let default = if made_of_nothing { format!("protected:\n    {name}() = default;\npublic:\n") } else { format!("    {name}() = default;\n") };
    format!(
        "{default}    {name}(std::nullptr_t) {{}}\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n\
         \x20   {name}(const {name}&) = default;\n    {name}({name}&&) = default;\n\
         \x20   {name}& operator=(const {name}& o) {{ ts::Handle::operator=(static_cast<const ts::Handle&>(o)); return *this; }}\n\
         \x20   {name}& operator=({name}&& o) noexcept {{ ts::Handle::operator=(std::move(static_cast<ts::Handle&>(o))); return *this; }}\n"
    )
}

// A type parameter's constraint, where C++ can state it.
fn constraint(name: &str, c: &Ty, cx: &Ctx) -> Option<String> {
    match c {
        Ty::Obj(_) | Ty::Generic(..) | Ty::Param(_) | Ty::Str | Ty::Num | Ty::Bool | Ty::ArrayOf(_) | Ty::Array | Ty::Record(_) => Some(format!("ts::extends<{name}, {}>", ty_arg(c, cx))),
        _ => None,
    }
}

// The wasm types a value crosses as.
fn raw(t: &Ty) -> Vec<&'static str> {
    use super::super::wasm::{AbiSlot::*, WasmParamAbi::*};
    match super::super::wasm::param_abi(t) {
        None => vec![],
        One(F64) => vec!["double"],
        One(I32) => vec!["int32_t"],
        One(U32 | Void) => vec!["uint32_t"],
        Str => vec!["const char*", "size_t"],
        OptNum => vec!["int32_t", "double"],
    }
}

pub(super) fn raw_ret(t: &Ty) -> &'static str {
    use super::super::wasm::AbiSlot::*;
    match super::super::wasm::ret_abi(t) {
        Void => "void",
        F64 => "double",
        I32 => "int32_t",
        U32 => "uint32_t",
    }
}

// Expressions turning a C++ value into what crosses the boundary.
fn lower(t: &Ty, x: &str, cx: &Ctx) -> Vec<String> {
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
        Ty::Nullable(inner, _) if matches!(&**inner, Ty::Obj(n) if cx.is_resolved_ref(n)) => vec![format!("ts::Arg({x}).h")],
        Ty::Nullable(..) => vec![format!("({x} ? {x}->h : 0)")],
        Ty::Param(_) | Ty::Generic(..) | Ty::NamedUnion(_) => vec![format!("ts::Arg({x}).h")],
        Ty::Obj(n) if cx.is_resolved_ref(n) => vec![format!("ts::Arg({x}).h")],
        _ => vec![format!("{x}.h")],
    }
}

// Expression turning what came back, `r`, into a C++ value.
pub(super) fn raise(t: &Ty, r: &str, unions: &[NamedUnion], cx: &Ctx) -> String {
    match t {
        Ty::Num => r.into(),
        Ty::NumEnum(e) => format!("{e}(int32_t({r}))"),
        Ty::Bool => format!("({r} != 0)"),
        Ty::Enum(e) => format!("{e}({r})"),
        Ty::Str => format!("ts::from_handle<std::string>({r})"),
        Ty::Nullable(inner, _) if **inner == Ty::Num => format!("ts::from_handle<std::optional<double>>({r})"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => {
            format!("({r} ? {}({}(int32_t(ts::from_handle<double>({r})))) : std::nullopt)", ty_out(t, unions, cx), ty_in(inner, cx))
        }
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("({r} < 0 ? std::nullopt : std::optional<bool>({r} != 0))"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("({r} < 0 ? std::nullopt : {}({}({r})))", ty_out(t, unions, cx), ty_in(inner, cx)),
        Ty::Nullable(inner, _) => format!("({r} ? {}({}) : std::nullopt)", ty_out(t, unions, cx), raise(inner, r, unions, cx)),
        Ty::NamedUnion(n) if decodable(n, unions) => format!("decode_{n}({r})"),
        Ty::Generic(n, args) if decodable(n, unions) => format!("decode_{n}<{}>({r})", args.iter().map(|a| ty_arg_for(n, a, cx)).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) if is_union(n, unions) => format!("ts::Value(ts::handle, {r})"),
        Ty::Obj(n) if cx.is_resolved_ref(n) => format!("ts::from_handle<{}>({r})", cx.tn(n)),
        Ty::Obj(n) => format!("{}(ts::handle, {r})", cx.tn(n)),
        Ty::Mapped(_) => format!("V(ts::handle, {r})"),
        Ty::Param(n) => format!("ts::from_handle<{n}>({r})"),
        Ty::Generic(..) | Ty::Array | Ty::ArrayOf(_) | Ty::Record(_) | Ty::Tuple(_) => format!("{}(ts::handle, {r})", ty_in(t, cx)),
        _ => format!("ts::Value(ts::handle, {r})"),
    }
}

#[derive(PartialEq, Clone, Copy)]
pub(super) enum Place {
    // A member of the class: `h` is the handle of the object it is called on.
    Member,
    // A static member of the class.
    Static,
    // A free function.
    Free,
}

// The C++ name of a function, from what it calls. `ctor` names constructors.
pub(super) fn cpp_name(f: &Func, ctor: &str) -> String {
    cpp_name_as(f, &f.name, ctor)
}

// As `cpp_name`, were the function's model name `name`.
pub(super) fn cpp_name_as(f: &Func, name: &str, ctor: &str) -> String {
    let base = match &f.call {
        Call::Get(p) | Call::GetBound(p) | Call::Set(p) => ident(p),
        Call::StaticGet(p) | Call::StaticSet(p) if f.module.is_empty() => ident(p.rsplit('.').next().unwrap_or(p)),
        Call::IndexGet => "at".into(),
        Call::IndexSet => "set_at".into(),
        Call::New(_) | Call::Create(_) => ctor.into(),
        _ => ident(name),
    };
    // Overloads that differ only in a fixed argument are named after it; not
    // after an options object the glue passes (`{"async": false}`), which lowering named.
    let values: Vec<String> = f.fixed.iter().filter(|(_, v)| !v.starts_with('{')).map(|(_, v)| super::super::value_name(v)).collect();
    if values.is_empty() { base } else { format!("{base}_{}", values.join("_")) }
}

// One function, in its parts.
pub(super) struct Function {
    // The wasm import it calls.
    pub import: String,
    // Its declaration: in the class, or in the module's header.
    pub decl: String,
    // Its definition.
    pub def: String,
    // A template, defined in a header; else defined once, in the module's `.cpp`.
    pub templated: bool,
    // What C++ tells overloads apart by: name and parameter types.
    pub key: String,
    // The types it names.
    pub refs: BTreeSet<String>,
}

fn mentions_param(t: &Ty, name: &str) -> bool {
    match t {
        Ty::Param(n) => n == name,
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => mentions_param(inner, name),
        Ty::Tuple(es) | Ty::Union(es) => es.iter().any(|e| mentions_param(e, name)),
        Ty::Generic(_, args) => args.iter().any(|a| mentions_param(a, name)),
        _ => false,
    }
}

pub(super) struct RestPackInfo {
    pub elem_ty: String,
    pub drop_tparam: Option<String>,
    pub constraint_ty: Option<Ty>,
}

pub(super) fn rest_pack_info(f: &Func, cx: &Ctx) -> Option<RestPackInfo> {
    if !f.rest {
        return None;
    }
    let (_, last_t) = f.params.last()?;
    match last_t {
        Ty::Array => Some(RestPackInfo { elem_ty: "ts::Value".into(), drop_tparam: None, constraint_ty: None }),
        Ty::ArrayOf(e) => Some(RestPackInfo { elem_ty: ty_arg_for("", e, cx), drop_tparam: None, constraint_ty: None }),
        Ty::Param(p) => {
            let tp = f.type_params.iter().find(|tp| tp.name == *p)?;
            let c_ty = tp.constraint.as_ref()?;
            let elem_ty = match c_ty {
                Ty::Array => "ts::Value".to_string(),
                Ty::ArrayOf(e) => ty_arg_for("", e, cx),
                _ => return None,
            };
            let rest_free = f.params[..f.params.len() - 1].iter().all(|(_, t)| !mentions_param(t, p))
                && !mentions_param(&f.ret, p)
                && f.type_params
                    .iter()
                    .filter(|other| other.name != *p)
                    .all(|other| !other.constraint.as_ref().is_some_and(|t| mentions_param(t, p)) && !other.default.as_ref().is_some_and(|t| mentions_param(t, p)));
            rest_free.then(|| RestPackInfo {
                elem_ty,
                drop_tparam: Some(p.clone()),
                constraint_ty: Some(c_ty.clone()),
            })
        }
        _ => None,
    }
}

fn borrowing_cb_type(t: &Ty, callbacks: &HashSet<&str>, cx: &Ctx) -> Option<String> {
    match t {
        Ty::Listener(ev) => Some(format!("ts::Listener<{}>", ty_in(ev, cx))),
        Ty::Obj(n) | Ty::Generic(n, _) if callbacks.contains(n.as_str()) => Some(ty_in(t, cx)),
        _ => None,
    }
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
    exact_params: &HashSet<usize>,
    rest_pack: Option<&str>,
    omit_rest: bool,
    drop_tparam: Option<&str>,
    rest_constraint_ty: Option<&Ty>,
    cx: &Ctx,
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
    let mut borrowed = Vec::new();
    let mut borrow_params = Vec::new();
    let mut borrow_requires = Vec::new();
    for (idx, (pname, t)) in params_slice.iter().enumerate() {
        // `w_`; `h`, reserved, is already `h_`: two underscores are the implementation's.
        let base = ident(pname);
        let pname = if base.ends_with('_') { base } else { format!("{base}_") };
        if f.borrowing
            && let Some(cb_ty) = borrowing_cb_type(t, callbacks, cx)
        {
            params.push(format!("F{idx}__&& {pname}"));
            raw_params.push("uint32_t");
            args.push(format!("ts_b{idx}__.h"));
            borrowed.push(format!("ts::BorrowedCb ts_b{idx}__({cb_ty}::borrow_handle({pname})); "));
            borrow_params.push(format!("class F{idx}__"));
            borrow_requires.push(format!("requires(std::remove_reference_t<F{idx}__>& fn_) {{ {cb_ty}::borrow_handle(fn_); }}"));
        } else {
            params.push(format!("{} {pname}", ty_param_with_exact(t, unions, callbacks, exact_params.contains(&idx), cx)));
            raw_params.extend(raw(t));
            args.extend(lower_param(t, &pname, unions, callbacks, cx));
        }
        named_types(t, &mut refs);
    }
    if omit_rest {
        raw_params.push("uint32_t");
        args.push("0".into());
        if let Some((_, last_t)) = f.params.last() {
            named_types(last_t, &mut refs);
        }
        if let Some(ct) = rest_constraint_ty {
            named_types(ct, &mut refs);
        }
    } else if rest_pack.is_some() {
        raw_params.push("uint32_t");
        args.push("rest_arr__.h".into());
        if let Some((_, last_t)) = f.params.last() {
            named_types(last_t, &mut refs);
        }
        if let Some(ct) = rest_constraint_ty {
            named_types(ct, &mut refs);
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
            t => format!("auto r = {call}; return {};", raise(t, "r", unions, cx)),
        }
    };
    let body = format!(
        "{}{}",
        borrowed.concat(),
        if let Some(elem_ty) = rest_pack {
            format!("auto rest_arr__ = ts::Array<{elem_ty}>::of(std::forward<R0__>(r0__), std::forward<Rest__>(rest__)...); {body_call}")
        } else {
            body_call
        }
    );

    let has_key = params_slice.iter().any(|p| matches!(p.1, Ty::Key(_)));
    let mut type_params = f.type_params.clone();
    if let Some(drop) = drop_tparam {
        type_params.retain(|p| p.name != drop);
    }
    if has_key {
        type_params.insert(0, TParam { name: "V".into(), default: None, constraint: None });
    }
    // One no argument deduces is its constraint, or any value: `intersectObjects(objects)`
    // is of Object3Ds, as TypeScript infers, with no `<Object3D<>>`.
    for (i, p) in type_params.iter_mut().enumerate() {
        if !(has_key && i == 0) && p.default.is_none() && !params_slice.iter().any(|(_, t)| is_deducible(&p.name, t, unions, callbacks)) {
            p.default = Some(match &p.constraint {
                Some(c @ (Ty::Obj(_) | Ty::Generic(..))) if constraint(&p.name, c, cx).is_some() => c.clone(),
                Some(_) if p.constraint.as_ref().and_then(|c| constraint(&p.name, c, cx)).is_some() => continue,
                _ => Ty::Any,
            });
        }
    }

    if rest_pack.is_some() {
        params.push("R0__&& r0__".into());
        params.push("Rest__&&... rest__".into());
    }
    let mut extra_params = borrow_params;
    let mut extra_requires = borrow_requires;
    if let Some(elem_ty) = rest_pack {
        extra_params.extend(["class R0__".into(), "class... Rest__".into()]);
        let check_extends = if drop_tparam.is_some() { "true" } else { "false" };
        extra_requires.push(format!("ts::rest_pack<{elem_ty}, {check_extends}, R0__, Rest__...>"));
    }
    let extra_refs: Vec<&str> = extra_params.iter().map(String::as_str).collect();

    // Defaults where the function is first declared: in the class, or at a free function.
    let (first, again) = if extra_refs.is_empty() {
        (head_func(&type_params, true, cx), head_func(&type_params, false, cx))
    } else {
        (
            head_with_extra(&type_params, true, &extra_refs, &extra_requires, cx),
            head_with_extra(&type_params, false, &extra_refs, &extra_requires, cx),
        )
    };
    let templated = !owner_head.is_empty() || !type_params.is_empty() || !extra_refs.is_empty();
    let inline = if templated { "inline " } else { "" };
    let ret = if fluent { format!("const {owner}&") } else { ty_out(&f.ret, unions, cx) };
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
    // On the declaration a caller reads: in the class, else where it is defined.
    let doc = f.doc.as_deref().filter(|_| rest_pack.is_none() && !omit_rest).map(|d| comment(d, "///", if place == Place::Free { "" } else { "    " }, Style::C)).unwrap_or_default();
    let (decl, def) = if decl.is_empty() { (decl, format!("{doc}{def}")) } else { (format!("{doc}{decl}"), def) };
    // With its template head: an overload constrained and one not are two,
    // the more constrained chosen where both fit (`merge<T extends Dictionary>`).
    // Its template parameters by place: C++ tells none apart by name.
    let again_key = head_func(&type_params, false, cx);
    let key = if let Some(elem_ty) = rest_pack {
        format!(
            "{}{}{name}({})|rest<{elem_ty}>",
            again_key.replace('\n', " "),
            if place == Place::Static { "static " } else { "" },
            params_slice.iter().enumerate().map(|(idx, p)| ty_param_with_exact(&p.1, unions, callbacks, exact_params.contains(&idx), cx)).collect::<Vec<_>>().join(",")
        )
    } else {
        format!(
            "{}{}{name}({})",
            again_key.replace('\n', " "),
            if place == Place::Static { "static " } else { "" },
            params_slice.iter().enumerate().map(|(idx, p)| ty_param_with_exact(&p.1, unions, callbacks, exact_params.contains(&idx), cx)).collect::<Vec<_>>().join(",")
        )
    };
    let key = rename_words(&key, &type_params.iter().enumerate().map(|(i, p)| (p.name.clone(), format!("${i}"))).collect());
    Function { import: import_decl, decl, def, templated, key, refs }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn function_variants(
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
    exact_params: &HashSet<usize>,
    cx: &Ctx,
) -> Vec<Function> {
    let mut out = vec![build_function(f, &name, owner, owner_head, place, unions, callbacks, wasm_module, raw_prefix, &f.params, exact_params, None, false, None, None, cx)];
    if let Some(rp) = rest_pack_info(f, cx) {
        let fixed = &f.params[..f.params.len() - 1];
        let zname = zero_name.as_deref().unwrap_or(&name);
        out.push(build_function(
            f,
            zname,
            owner,
            owner_head,
            place,
            unions,
            callbacks,
            wasm_module,
            raw_prefix,
            fixed,
            exact_params,
            None,
            true,
            rp.drop_tparam.as_deref(),
            rp.constraint_ty.as_ref(),
            cx,
        ));
        out.push(build_function(
            f,
            &name,
            owner,
            owner_head,
            place,
            unions,
            callbacks,
            wasm_module,
            raw_prefix,
            fixed,
            exact_params,
            Some(&rp.elem_ty),
            false,
            rp.drop_tparam.as_deref(),
            rp.constraint_ty.as_ref(),
            cx,
        ));
    }
    out
}

// The declared types a type names: classes, callbacks, unions and maps.
pub(super) fn named_types(t: &Ty, out: &mut BTreeSet<String>) {
    match t {
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => {
            out.insert(n.clone());
        }
        Ty::Nullable(inner, _) | Ty::Listener(inner) | Ty::ArrayOf(inner) | Ty::Record(inner) => named_types(inner, out),
        Ty::Tuple(es) => es.iter().for_each(|e| named_types(e, out)),
        Ty::Generic(n, args) => {
            out.insert(n.clone());
            args.iter().for_each(|a| named_types(a, out));
        }
        _ => {}
    }
}

pub(super) fn param_types(params: &[TParam], out: &mut BTreeSet<String>) {
    for p in params {
        p.default.iter().chain(&p.constraint).for_each(|t| named_types(t, out));
    }
}
