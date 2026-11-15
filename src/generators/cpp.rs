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

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::model::{Call, Class, Func, Model, NamedUnion, TParam, Ty, base_args, map_types, subst, with_self_args};

fn ident(name: &str) -> String {
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
    let s = super::model::ident(name);
    if KEYWORDS.contains(&s.as_str()) { format!("{s}_") } else { s }
}

// C++ type of a value going in. `V` is the value a typed key selects.
fn ty_in(t: &Ty) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str => "std::string_view".into(),
        Ty::Any => "ts::Value".into(),
        Ty::Array => "ts::Array<>".into(),
        Ty::ArrayOf(e) => format!("ts::Array<{}>", ty_arg(e)),
        Ty::Tuple(es) => format!("ts::Tuple<{}>", es.iter().map(ty_arg).collect::<Vec<_>>().join(", ")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_)) => format!("std::optional<{}>", ty_in(inner).trim_start_matches("const ").trim_end_matches('&')),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_in(inner)),
        Ty::NamedUnion(n) => format!("const {n}&"),
        Ty::Union(_) => "ts::Value".into(),
        Ty::Key(m) => format!("ts::Key<{m}, V>"),
        Ty::Mapped(_) => "V".into(),
        // Not deduced from the lambda: the key says what the listener takes.
        Ty::Listener(ev) => format!("std::type_identity_t<ts::Listener<{}>>", ty_in(ev)),
        Ty::Param(n) => n.clone(),
        Ty::Generic(n, args) => format!("{n}<{}>", args.iter().map(ty_arg).collect::<Vec<_>>().join(", ")),
    }
}

thread_local! {
    // The unions of the model being generated that values can be decoded to.
    static DECODABLE: std::cell::RefCell<BTreeSet<String>> = const { std::cell::RefCell::new(BTreeSet::new()) };
}

// A type as a template argument: a value type, not a view.
fn ty_arg(t: &Ty) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_arg(inner)),
        // A union a handle can be decoded to; any other stays a value.
        Ty::NamedUnion(n) if DECODABLE.with(|d| d.borrow().contains(n)) => n.clone(),
        Ty::NamedUnion(_) | Ty::Union(_) => "ts::Value".into(),
        t => ty_in(t),
    }
}

// C++ type of a value coming back.
fn ty_out(t: &Ty, unions: &[NamedUnion]) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_out(inner, unions)),
        Ty::NamedUnion(n) | Ty::Generic(n, _) if is_union(n, unions) && !decodable(n, unions) => "ts::Value".into(),
        Ty::NamedUnion(n) => n.clone(),
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
            Some(d) if defaults && trailing => format!("class {} = {}", p.name, ty_arg(d)),
            _ => format!("class {}", p.name),
        })
        .collect();
    let requires: Vec<String> = params.iter().filter_map(|p| constraint(&p.name, p.constraint.as_ref()?)).collect();
    let requires = if requires.is_empty() { String::new() } else { format!(" requires {}", requires.join(" && ")) };
    format!("template <{}>{requires}\n", ps.join(", "))
}

// A type's constructors: none, or from a handle.
fn constructors(name: &str) -> String {
    format!("    {name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n")
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
    match t {
        Ty::Void => vec![],
        Ty::Num | Ty::NumEnum(_) => vec!["double"],
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => vec!["int32_t"],
        Ty::Str => vec!["const char*", "size_t"],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec!["const char*", "size_t"],
        // An optional number: whether it is there, and the number.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => vec!["int32_t", "double"],
        // An optional flag or enum: -1 when absent.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => vec!["int32_t"],
        _ => vec!["uint32_t"],
    }
}

fn raw_ret(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t",
        _ => "uint32_t",
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
        Ty::Nullable(..) => vec![format!("({x} ? {x}->h : 0)")],
        Ty::Param(_) | Ty::Generic(..) | Ty::NamedUnion(_) => vec![format!("ts::Arg({x}).h")],
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
        Ty::Generic(n, args) if decodable(n, unions) => format!("decode_{n}<{}>({r})", args.iter().map(ty_arg).collect::<Vec<_>>().join(", ")),
        Ty::Generic(n, _) if is_union(n, unions) => format!("ts::Value(ts::handle, {r})"),
        Ty::Obj(n) => format!("{n}(ts::handle, {r})"),
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
    let base = match &f.call {
        Call::Get(p) | Call::Set(p) => ident(p),
        Call::StaticGet(p) | Call::StaticSet(p) if f.module.is_empty() => ident(p.rsplit('.').next().unwrap_or(p)),
        Call::IndexGet => "at".into(),
        Call::IndexSet => "set_at".into(),
        Call::New(_) | Call::Create => ctor.into(),
        _ => ident(&f.name),
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
fn function(f: &Func, name: String, owner: &str, owner_head: &str, place: Place, unions: &[NamedUnion]) -> Function {
    let mut raw_params: Vec<&str> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if Class::has_self(f) {
        raw_params.push("uint32_t");
        // `this->`: in a class template, the handle is in a dependent base.
        args.push("this->h".into());
    }
    let mut params = Vec::new();
    let mut refs = BTreeSet::new();
    for (name, t) in &f.params {
        // `w_`; `h`, reserved, is already `h_`: two underscores are the implementation's.
        let base = ident(name);
        let name = if base.ends_with('_') { base } else { format!("{base}_") };
        params.push(format!("{} {name}", ty_in(t)));
        raw_params.extend(raw(t));
        args.extend(lower(t, &name));
        named_types(t, &mut refs);
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
    let import_decl = format!("extern \"C\" TS_IMPORT({import}) {} ts_raw_{import}({});\n", raw_ret(&f.ret), raw_params.join(", "));
    let call = format!("ts_raw_{import}({})", args.join(", "));
    let body = match &f.ret {
        Ty::Void => format!("{call};"),
        t => format!("auto r = {call}; return {};", raise(t, "r", unions)),
    };
    let mut type_params = f.type_params.clone();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        type_params.insert(0, TParam { name: "V".into(), default: None, constraint: None });
    }
    // Defaults where the function is first declared: in the class, or at a free function.
    let (first, again) = (head(&type_params, true), head(&type_params, false));
    let templated = !owner_head.is_empty() || !type_params.is_empty();
    let inline = if templated { "inline " } else { "" };
    let (ret, params) = (ty_out(&f.ret, unions), params.join(", "));
    let (decl, def) = match place {
        Place::Member => (format!("    {}{ret} {name}({params}) const;\n", first.replace('\n', " ")), format!("{owner_head}{again}{inline}{ret} {owner}::{name}({params}) const {{ {body} }}\n")),
        Place::Static => (format!("    {}static {ret} {name}({params});\n", first.replace('\n', " ")), format!("{owner_head}{again}{inline}{ret} {owner}::{name}({params}) {{ {body} }}\n")),
        Place::Free if templated => (String::new(), format!("{first}inline {ret} {name}({params}) {{ {body} }}\n")),
        Place::Free => (format!("{ret} {name}({params});\n"), format!("{ret} {name}({params}) {{ {body} }}\n")),
    };
    let key = format!("{}{name}({})", if place == Place::Static { "static " } else { "" }, f.params.iter().map(|p| ty_in(&p.1)).collect::<Vec<_>>().join(","));
    Function { import: import_decl, decl, def, templated, key, refs }
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

const RUNTIME: &str = r#"// Generated.
#pragma once
#include <concepts>
#include <coroutine>
#include <cstddef>
#include <cstdint>
#include <initializer_list>
#include <optional>
#include <string>
#include <string_view>
#include <tuple>
#include <type_traits>
#include <utility>
#include <variant>
#include <vector>

#define TS_IMPORT(name) __attribute__((import_module("env"), import_name(#name)))

extern "C" {
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_bool) uint32_t ts_raw_js_from_bool(int32_t);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, size_t);
TS_IMPORT(js_to_number) double ts_raw_js_to_number(uint32_t);
TS_IMPORT(js_to_bool) int32_t ts_raw_js_to_bool(uint32_t);
TS_IMPORT(js_equals) int32_t ts_raw_js_equals(uint32_t, uint32_t);
TS_IMPORT(js_array_new) uint32_t ts_raw_js_array_new();
TS_IMPORT(js_array_length) uint32_t ts_raw_js_array_length(uint32_t);
TS_IMPORT(js_array_get) uint32_t ts_raw_js_array_get(uint32_t, uint32_t);
TS_IMPORT(js_array_push) void ts_raw_js_array_push(uint32_t, uint32_t);
TS_IMPORT(js_listener_from) uint32_t ts_raw_js_listener_from(uintptr_t, void*);
TS_IMPORT(js_callback_release) uint32_t ts_raw_js_callback_release(uint32_t);
TS_IMPORT(js_retain) void ts_raw_js_retain(uint32_t);
TS_IMPORT(js_release) void ts_raw_js_release(uint32_t);
TS_IMPORT(js_live) uint32_t ts_raw_js_live();
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error();
TS_IMPORT(js_string_take) uint32_t ts_raw_js_string_take(uint32_t, char*, size_t);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
TS_IMPORT(js_error_name) uint32_t ts_raw_js_error_name(uint32_t);
TS_IMPORT(js_error_stack) uint32_t ts_raw_js_error_stack(uint32_t);
TS_IMPORT(js_error_new) uint32_t ts_raw_js_error_new(const char*, size_t);
TS_IMPORT(js_date_new) uint32_t ts_raw_js_date_new(double);
TS_IMPORT(js_date_now) double ts_raw_js_date_now();
TS_IMPORT(js_date_time) double ts_raw_js_date_time(uint32_t);
TS_IMPORT(js_date_to_iso_string) uint32_t ts_raw_js_date_to_iso_string(uint32_t);
TS_IMPORT(js_bytes_len) uint32_t ts_raw_js_bytes_len(uint32_t);
TS_IMPORT(js_bytes_read) uint32_t ts_raw_js_bytes_read(uint32_t, uint32_t, void*, uint32_t);
TS_IMPORT(js_bytes_write) uint32_t ts_raw_js_bytes_write(uint32_t, uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_new) uint32_t ts_raw_js_bytes_new(uint32_t, const void*, uint32_t);
TS_IMPORT(js_bytes_buffer) uint32_t ts_raw_js_bytes_buffer(uint32_t);
TS_IMPORT(js_bytes_offset) uint32_t ts_raw_js_bytes_offset(uint32_t);
TS_IMPORT(js_promise_then) void ts_raw_js_promise_then(uint32_t, uintptr_t, void*);
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_enum_value) uint32_t ts_raw_js_enum_value(int32_t, int32_t);
TS_IMPORT(js_enum_index) int32_t ts_raw_js_enum_index(int32_t, uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
}

namespace ts {

// Tag for making a value from a handle.
struct handle_t {
    explicit handle_t() = default;
};
inline constexpr handle_t handle{};

// A JavaScript object. Zero is null. A handle is a counted reference:
// copies retain it, and the last one to go frees it on the JavaScript side.
struct Handle {
    uint32_t h = 0;
    Handle() = default;
    // Takes over a reference: what JavaScript returned.
    Handle(handle_t, uint32_t h) : h(h) {}
    Handle(const Handle& o) : h(o.h) {
        if (h) ts_raw_js_retain(h);
    }
    Handle(Handle&& o) noexcept : h(std::exchange(o.h, 0)) {}
    Handle& operator=(Handle o) noexcept {
        std::swap(h, o.h);
        return *this;
    }
    ~Handle() {
        if (h) ts_raw_js_release(h);
    }
    // Whether two handles are one JavaScript value.
    friend bool operator==(const Handle& a, const Handle& b) { return a.h == b.h || ts_raw_js_equals(a.h, b.h) != 0; }
};

// How many JavaScript values wasm holds handles to.
inline uint32_t live_handles() { return ts_raw_js_live(); }

template <class T>
inline constexpr bool is_handle = std::is_base_of_v<Handle, std::decay_t<T>>;

// T, as a type that depends on D: checked only once D is known.
template <class D, class T>
struct dep_ {
    using type = T;
};
template <class D, class T>
using dep = typename dep_<D, T>::type;

inline std::string read_string(uint32_t h) {
    std::string s(ts_raw_js_string_len(h), '\0');
    ts_raw_js_string_read(h, s.data());
    return s;
}

// A string from a handle JavaScript returned, which is released: in one
// call, when it fits a small buffer.
inline std::string take_string(uint32_t h) {
    char buf[256];
    uint32_t n = ts_raw_js_string_take(h, buf, sizeof buf);
    if (n <= sizeof buf) return std::string(buf, n);
    std::string s(n, '\0');
    ts_raw_js_string_read(h, s.data());
    ts_raw_js_release(h);
    return s;
}

// Which enum of strings a type is, for its values to cross as handles: -1 for
// an enum of numbers.
template <class T>
inline constexpr int32_t enum_id = -1;

// A value lent to JavaScript as a handle, for one call: an object's own
// handle, or a new one for a number, flag or string, released after the call.
struct Arg {
    uint32_t h = 0;
    bool own = false;
    Arg(const std::string& s) : h(ts_raw_js_from_string(s.data(), s.size())), own(true) {}
    Arg(std::string_view s) : h(ts_raw_js_from_string(s.data(), s.size())), own(true) {}
    Arg(const char* s) : Arg(std::string_view(s)) {}
    Arg(double x) : h(ts_raw_js_from_number(x)), own(true) {}
    Arg(bool x) : h(ts_raw_js_from_bool(x)), own(true) {}
    template <class H, std::enable_if_t<is_handle<H>, int> = 0>
    Arg(const H& v) : h(v.h) {}
    template <class E, std::enable_if_t<std::is_enum_v<E>, int> = 0>
    Arg(E x) : h(enum_id<E> < 0 ? ts_raw_js_from_number(double(int32_t(x))) : ts_raw_js_enum_value(enum_id<E>, int32_t(x))), own(true) {}
    template <class... T>
    Arg(const std::variant<T...>& v) {
        std::visit([this](const auto& x) { take(Arg(x)); }, v);
    }
    template <class T>
    Arg(const std::optional<T>& v) {
        if (v) take(Arg(*v));
    }
    Arg(const Arg&) = delete;
    Arg& operator=(const Arg&) = delete;
    ~Arg() {
        if (own) ts_raw_js_release(h);
    }
    // A reference for JavaScript to keep: what a callback returns.
    uint32_t give() {
        if (!own && h) ts_raw_js_retain(h);
        own = false;
        return h;
    }

  private:
    void take(Arg&& a) {
        h = a.h;
        own = std::exchange(a.own, false);
    }
};

// What `from_handle` turns into a union: specialized for each union a value can be decoded to.
template <class T>
struct decoder {};

// What a type argument is, for telling a bare type parameter apart in a union:
// 1 a number, 2 a string, 3 a flag, 0 anything else.
template <class T>
inline constexpr int32_t kind_of = std::is_same_v<T, double> ? 1 : std::is_same_v<T, std::string> ? 2 : std::is_same_v<T, bool> ? 3 : 0;

template <class T>
struct is_optional : std::false_type {};
template <class T>
struct is_optional<std::optional<T>> : std::true_type {};

// A value of type T from a handle JavaScript returned: a number, a flag, a
// string, a union or an object. Takes over the handle.
template <class T>
T from_handle(uint32_t h) {
    if constexpr (is_optional<T>::value) {
        if (!h) return std::nullopt;
        return from_handle<typename T::value_type>(h);
    } else if constexpr (requires { decoder<T>::from(h); }) {
        return decoder<T>::from(h);
    } else if constexpr (std::is_same_v<T, std::string>) {
        return take_string(h);
    } else if constexpr (std::is_enum_v<T>) {
        T x = enum_id<T> < 0 ? T(int32_t(ts_raw_js_to_number(h))) : T(ts_raw_js_enum_index(enum_id<T>, h));
        ts_raw_js_release(h);
        return x;
    } else if constexpr (std::is_same_v<T, double> || std::is_same_v<T, bool>) {
        T x;
        if constexpr (std::is_same_v<T, double>) x = ts_raw_js_to_number(h);
        else x = ts_raw_js_to_bool(h) != 0;
        ts_raw_js_release(h);
        return x;
    } else return T(handle, h);
}

// A JavaScript value of no particular type.
struct Value : Handle {
    using Handle::Handle;
    static Value number(double x) { return {handle, ts_raw_js_from_number(x)}; }
    static Value boolean(bool x) { return {handle, ts_raw_js_from_bool(x)}; }
    static Value string(std::string_view s) { return {handle, ts_raw_js_from_string(s.data(), s.size())}; }
    double to_number() const { return ts_raw_js_to_number(h); }
    bool to_bool() const { return ts_raw_js_to_bool(h) != 0; }
    std::string to_string() const { return read_string(h); }
    bool is_null() const { return h == 0; }
};

// The direct bases of a type, declared beside it in `ts_types.hpp`: what
// `extends` follows. A constraint is then checked of types that are only
// declared, as a template's arguments are when the template is named.
template <class... T>
struct list {};
template <class T>
struct bases {
    using type = list<>;
};

// TypeScript's `T extends U`, for a constraint. Anything extends `any`, and
// `any` anything; a union, if each of its members does.
template <class T, class U>
struct extends_t;
template <class U, class L>
struct any_extends : std::false_type {};
template <class U, class B, class... R>
struct any_extends<U, list<B, R...>> : std::bool_constant<extends_t<B, U>::value || any_extends<U, list<R...>>::value> {};
template <class T, class U>
struct extends_t : std::bool_constant<std::same_as<T, U> || std::same_as<T, Value> || std::same_as<U, Value> || any_extends<U, typename bases<T>::type>::value> {};
template <class... A, class U>
struct extends_t<std::variant<A...>, U> : std::bool_constant<(extends_t<A, U>::value && ...)> {};
// A type extends a union it is, or extends, a member of; a union, one whose
// members each do.
template <class T, class... B>
struct extends_t<T, std::variant<B...>> : std::bool_constant<(extends_t<T, B>::value || ...)> {};
template <class... A, class... B>
struct extends_t<std::variant<A...>, std::variant<B...>> : std::bool_constant<(extends_t<A, std::variant<B...>>::value && ...)> {};
template <class T, class U>
concept extends = extends_t<T, U>::value;

template <class T>
struct Iterator;
struct IterationEnd {};

// A JavaScript array of T; `Array<>` of any values. A type that extends
// Array inherits this: it can be indexed and iterated.
template <class T = Value>
struct Array : virtual Handle {
    Array() = default;
    Array(handle_t, uint32_t h) : Handle(handle, h) {}
    Array(std::initializer_list<T> items) : Handle(handle, ts_raw_js_array_new()) {
        for (const T& x : items) push(x);
    }
    static Array create() { return Array(handle, ts_raw_js_array_new()); }
    uint32_t size() const { return ts_raw_js_array_length(h); }
    T operator[](uint32_t i) const { return from_handle<T>(ts_raw_js_array_get(h, i)); }
    template <class U>
        requires std::same_as<T, Value> || std::convertible_to<const U&, T>
    void push(const U& v) const {
        if constexpr (std::same_as<T, Value>) ts_raw_js_array_push(h, Arg(v).h);
        else ts_raw_js_array_push(h, Arg(T(v)).h);
    }
    Iterator<T> begin() const;
    IterationEnd end() const { return {}; }
};

// A JavaScript exception, or the reason a promise was rejected.
struct Error : Handle {
    using Handle::Handle;
    static Error create(std::string_view msg) { return Error(handle, ts_raw_js_error_new(msg.data(), msg.size())); }
    std::string name() const { return from_handle<std::string>(ts_raw_js_error_name(h)); }
    std::string message() const { return from_handle<std::string>(ts_raw_js_error_message(h)); }
    std::string stack() const { return from_handle<std::string>(ts_raw_js_error_stack(h)); }
};

// The exception a call threw, taken. A call that throws returns a zero
// value, and an exception not taken by the time control returns to
// JavaScript is reported there: see `attempt`.
inline std::optional<Error> take_error() {
    if (uint32_t e = ts_raw_js_take_error()) return Error(handle, e);
    return std::nullopt;
}

// A value, or the error that came instead: what a promise settles with, and
// what `attempt` returns.
template <class T>
struct [[nodiscard]] Result {
    Result(T x) : v(std::in_place_index<0>, std::move(x)) {}
    Result(Error e) : v(std::in_place_index<1>, std::move(e)) {}
    explicit operator bool() const { return v.index() == 0; }
    T& operator*() { return std::get<0>(v); }
    T* operator->() { return &std::get<0>(v); }
    const Error& error() const { return std::get<1>(v); }

  private:
    std::variant<T, Error> v;
};
template <>
struct [[nodiscard]] Result<void> {
    Result() = default;
    Result(Error e) : e(std::move(e)) {}
    explicit operator bool() const { return !e; }
    const Error& error() const { return *e; }

  private:
    std::optional<Error> e;
};

// Runs f and gives what it returned, or the exception a call in it threw:
// `auto found = ts::attempt([&] { return el.querySelector(text); });`.
template <class F>
auto attempt(F f) -> Result<std::invoke_result_t<F>> {
    using T = std::invoke_result_t<F>;
    if constexpr (std::is_void_v<T>) {
        f();
        if (auto e = take_error()) return Result<void>(std::move(*e));
        return Result<void>();
    } else {
        T x = f();
        if (auto e = take_error()) return Result<T>(std::move(*e));
        return Result<T>(std::move(x));
    }
}

// A JavaScript promise. `then` calls a function with its Result once it
// settles; in a coroutine, `co_await` waits for it: `auto r = co_await fetch(url);`.
template <class T>
struct Promise : virtual Handle {
    Promise() = default;
    Promise(handle_t, uint32_t h) : Handle(handle, h) {}

    template <class F>
    void then(F f) const {
        ts_raw_js_promise_then(h, reinterpret_cast<uintptr_t>(&settle<F>), new F(std::move(f)));
    }

    struct Awaiter {
        Promise p;
        std::optional<Result<T>> r;
        bool await_ready() const { return false; }
        void await_suspend(std::coroutine_handle<> c) {
            p.then([this, c](Result<T> x) {
                r.emplace(std::move(x));
                c.resume();
            });
        }
        Result<T> await_resume() { return std::move(*r); }
    };
    Awaiter operator co_await() const { return Awaiter{*this, std::nullopt}; }

  private:
    template <class F>
    static void settle(void* ctx, int32_t ok, uint32_t v) {
        F* f = static_cast<F*>(ctx);
        if (!ok) (*f)(Result<T>(Error(handle, v)));
        else if constexpr (std::is_void_v<T>) {
            if (v) ts_raw_js_release(v);
            (*f)(Result<T>());
        } else (*f)(Result<T>(from_handle<T>(v)));
        delete f;
    }
};

// Walks a JavaScript async iterator. `next()` is a promise of the next value,
// or of nothing at the end: `while (auto r = co_await it.next()) { if (!*r) break; ... }`.
template <class T>
struct AsyncIterator : Handle {
    using Handle::Handle;
    Promise<std::optional<T>> next() const { return Promise<std::optional<T>>(handle, ts_raw_js_async_iterator_next(h)); }
};

// A coroutine that starts at once and runs to its end, awaiting promises on the way.
struct Task {
    struct promise_type {
        Task get_return_object() { return {}; }
        std::suspend_never initial_suspend() noexcept { return {}; }
        std::suspend_never final_suspend() noexcept { return {}; }
        void return_void() {}
        void unhandled_exception() {}
    };
};

// Walks a JavaScript iterator, for a range-for: `for (Node n : list)`.
template <class T>
struct Iterator {
    explicit Iterator(uint32_t it) : it(handle, it) { ++*this; }
    T operator*() const { return *cur; }
    Iterator& operator++() {
        uint32_t v = ts_raw_js_iterator_next(it.h);
        if (v == 0xFFFFFFFFu) cur.reset();
        else cur.emplace(from_handle<T>(v));
        return *this;
    }
    bool operator==(IterationEnd) const { return !cur; }

  private:
    Handle it;
    std::optional<T> cur;
};

template <class T>
Iterator<T> Array<T>::begin() const { return Iterator<T>(ts_raw_js_iterate(h)); }

// A JavaScript array of fixed length and element types: `[number, string]`
// is `Tuple<double, std::string>`. `get<0>()` reads an element.
template <class... T>
struct Tuple : Handle {
    Tuple() = default;
    Tuple(handle_t, uint32_t h) : Handle(handle, h) {}
    explicit Tuple(const T&... xs) : Handle(handle, ts_raw_js_array_new()) { (ts_raw_js_array_push(h, Arg(xs).h), ...); }
    template <size_t I>
    std::tuple_element_t<I, std::tuple<T...>> get() const {
        return from_handle<std::tuple_element_t<I, std::tuple<T...>>>(ts_raw_js_array_get(h, I));
    }
};

// A key of a map whose value has type V: `HTMLElementEventMapKeys::click`.
template <class Map, class V>
struct Key {
    int32_t index;
};

// A lambda kept for JavaScript to call, until released.
struct Closure {
    virtual ~Closure() = default;
};
template <class F>
struct ClosureOf : Closure {
    F f;
    explicit ClosureOf(F f) : f(std::move(f)) {}
};
template <class F>
void* closure(F f) { return static_cast<Closure*>(new ClosureOf<F>(std::move(f))); }
template <class F>
F& closure_fn(void* ctx) { return static_cast<ClosureOf<F>*>(static_cast<Closure*>(ctx))->f; }

// A JavaScript function. One made from a lambda lives until `release()`,
// which frees the lambda; JavaScript must not call it after that. Copies
// share the function: release one of them, once.
struct Callback : virtual Handle {
    void release() {
        if (uint32_t ctx = ts_raw_js_callback_release(h)) delete static_cast<Closure*>(reinterpret_cast<void*>(uintptr_t(ctx)));
        static_cast<Handle&>(*this) = Handle();
    }
};

// A callback released when it goes out of scope.
template <class C>
struct Scoped : C {
    template <class F>
    Scoped(F f) : Handle(C(std::move(f))) {}
    Scoped(const Scoped&) = delete;
    Scoped& operator=(const Scoped&) = delete;
    ~Scoped() { this->release(); }
};

// A callback taking one object of type E, made from a lambda. Pass the same
// Listener to add and remove it.
template <class E>
struct Listener : Callback {
    Listener() = default;
    Listener(handle_t, uint32_t h) : Handle(handle, h) {}
    template <class F, std::enable_if_t<!is_handle<F>, int> = 0>
    Listener(F f) : Handle(handle, ts_raw_js_listener_from(reinterpret_cast<uintptr_t>(&call<F>), closure(std::move(f)))) {}

  private:
    template <class F>
    static void call(void* ctx, uint32_t ev) { closure_fn<F>(ctx)(E(handle, ev)); }
};

}  // namespace ts
"#;

// The bindings as files: see the top of this file.
pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    let unions = &m.unions;
    DECODABLE.with(|d| *d.borrow_mut() = unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect());
    let mut taken = HashMap::new();
    // Where each declared type is defined, without the extension.
    let mut home: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        home.insert(c.name.clone(), super::model::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
    }
    for name in m.callbacks.iter().map(|c| &c.name).chain(m.unions.iter().map(|u| &u.name)) {
        home.insert(name.clone(), super::model::file_name(&format!("types/{name}"), &mut taken));
    }
    // `#include` from one generated file to another.
    let include = |from: &str, to: &str| format!("#include \"{}{to}\"\n", "../".repeat(from.matches('/').count()));
    // The headers of the given types.
    let includes = |from: &str, types: &BTreeSet<String>| -> String {
        types.iter().filter_map(|t| home.get(t)).map(|h| format!("{h}.hpp")).filter(|h| h != from).map(|h| include(from, &h)).collect()
    };

    let types = declarations(m);
    let mut out = vec![("ts_runtime.hpp".to_string(), RUNTIME.to_string()), ("ts_types.hpp".to_string(), types)];

    // A union: its alternatives, and how to tell which one came back.
    for u in unions {
        let path = format!("{}.hpp", home[&u.name]);
        let mut parts = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut parts));
        parts.remove(&u.name);
        let mut text = format!("// Generated.\n#pragma once\n{}{}", include(&path, "ts_types.hpp"), includes(&path, &parts));
        text += &format!("extern \"C\" TS_IMPORT({0}_tag) int32_t ts_raw_{0}_tag(uint32_t, int32_t);\n", u.name);
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
            text += &format!(
                "{}inline {name} decode_{}(uint32_t h) {{\n{check}    switch (ts_raw_{1}_tag(h, {kind})) {{\n{2}    }}\n    __builtin_trap();\n}}\n",
                head(&u.type_params, false),
                u.name,
                cases.concat()
            );
            if decoder_owner(u, unions) {
                text += &format!("inline ::{0} ts::decoder<::{0}>::from(uint32_t h) {{ return decode_{0}(h); }}\n", u.name);
            }
        }
        out.push((path, text));
    }
    // A callback, made from a lambda. Its arguments are made only once the
    // lambda's type is known, so the types they name need not be complete yet.
    for c in &m.callbacks {
        let path = format!("{}.hpp", home[&c.name]);
        let mut used = BTreeSet::new();
        c.params.iter().for_each(|p| named_types(&p.1, &mut used));
        named_types(&c.ret, &mut used);
        used.remove(&c.name);
        let raw_args: String = c.params.iter().enumerate().map(|(i, (_, t))| format!(", {} a{i}", raw_callback(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| raise_callback(t, &format!("a{i}"), unions)).collect();
        let call = format!("ts::closure_fn<F>(ctx)({})", args.join(", "));
        let result = match &c.ret {
            Ty::Void => format!("{call};"),
            t => format!("return {};", lower_callback(t, &call)),
        };
        let (name, ret) = (&c.name, raw_callback(&c.ret));
        let tpl = head(&c.type_params, false);
        let text = format!(
            "// Generated.\n#pragma once\n{}extern \"C\" TS_IMPORT({name}_from) uint32_t ts_raw_{name}_from(uintptr_t, void*);\n\n\
             {tpl}struct {name} : ts::Callback {{\n{}    \
             template <class F, std::enable_if_t<!ts::is_handle<F>, int> = 0>\n    \
             {name}(F f) : ts::Handle(ts::handle, ts_raw_{name}_from(reinterpret_cast<uintptr_t>(&call<F>), ts::closure(std::move(f)))) {{}}\n\n  \
             private:\n    template <class F>\n    static {ret} call(void* ctx{raw_args}) {{ {result} }}\n}};\n\n{}",
            include(&path, "ts_types.hpp"),
            constructors(name),
            includes(&path, &used)
        );
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
    // Per module: headers, and the definitions compiled once.
    let mut modules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut library: BTreeMap<String, (BTreeSet<String>, String)> = BTreeMap::new();
    for c in &m.classes {
        let base = &home[&c.name];
        let (decl_path, impl_path) = (format!("{base}.decl.hpp"), format!("{base}.hpp"));
        // A class template is written `Box<T>` where its members are defined.
        let class_head = head(&c.type_params, false);
        let owner = if c.type_params.is_empty() { c.name.clone() } else { format!("{}<{}>", c.name, c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        // A member named `create` leaves the constructors `make`.
        let ctor = if c.funcs.iter().any(|f| !matches!(f.call, Call::New(_) | Call::Create) && cpp_name(f, "") == "create") { "make" } else { "create" };

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
        let mut usings = String::new();
        for (name, hs) in &holders {
            let visible: Vec<&&String> = hs.iter().filter(|h| !hs.iter().any(|o| o != *h && ancestors(o).contains(**h))).collect();
            if own.contains(name) && visible.len() == 1 {
                let h = visible[0];
                let base = match base_args(m, c, h) {
                    Some(args) if !args.is_empty() => ty_in(&Ty::Generic((*h).clone(), args)),
                    _ => (*h).clone(),
                };
                usings += &format!("    using {base}::{};\n", ident(name));
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
        let signature = |f: &Func| format!("{}({})", cpp_name(f, ctor), f.params.iter().map(|p| ty_in(&p.1)).collect::<Vec<_>>().join(","));
        let instance: BTreeSet<String> = c.funcs.iter().filter(|f| Class::has_self(f)).map(|f| signature(&with_self_args(f, c))).collect();
        for f in c.funcs.iter().map(|f| with_self_args(f, c)).chain(again) {
            let place = if Class::has_self(&f) { Place::Member } else { Place::Static };
            let name = if place == Place::Static && instance.contains(&signature(&f)) { format!("{}_static", cpp_name(&f, ctor)) } else { cpp_name(&f, ctor) };
            let func = function(&f, name, &owner, &class_head, place, unions);
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
        for (name, value) in &c.consts {
            let ty = if value.starts_with('"') { "std::string_view" } else { "double" };
            members += &format!("    static constexpr {ty} {} = {value};\n", ident(name));
        }
        // Iterable: `for (Node n : list)`.
        if let Some(elem) = &c.iterates
            && !own.contains("begin")
            && !own.contains("end")
        {
            let it = format!("ts::Iterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret));
            members += &format!("    {it} begin() const;\n    ts::IterationEnd end() const {{ return {{}}; }}\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::begin() const {{ return {it}(ts_raw_js_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        if let Some(elem) = &c.iterates_async
            && !own.contains("iterate_async")
        {
            let it = format!("ts::AsyncIterator<{}>", ty_arg(&with_self_args(&Func { type_params: vec![], module: String::new(), import: String::new(), name: String::new(), call: Call::Create, params: vec![], rest: false, fixed: vec![], shared: None, ret: elem.clone() }, c).ret));
            members += &format!("    {it} iterate_async() const;\n");
            let templated = !class_head.is_empty();
            let def = format!("{class_head}{}{it} {owner}::iterate_async() const {{ return {it}(ts::handle, ts_raw_js_async_iterate(this->h)); }}\n", if templated { "inline " } else { "" });
            named_types(elem, &mut used);
            if templated { header_defs += &def } else { lib.1 += &def }
        }
        let bases: Vec<String> = if c.bases.is_empty() { vec!["virtual ts::Handle".into()] } else { c.bases.iter().map(|b| format!("virtual {}", ty_in(b))).collect() };
        let mut decl = format!("// Generated.\n#pragma once\n{}", include(&decl_path, "ts_types.hpp"));
        for a in &c.ancestors {
            if let Some(h) = home.get(a) {
                decl += &include(&decl_path, &format!("{h}.decl.hpp"));
            }
        }
        decl += &format!(
            "\n{class_head}struct {0} : {1} {{\n{2}{usings}{members}}};\n",
            c.name,
            bases.join(", "),
            constructors(&c.name)
        );
        used.remove(&c.name);
        used.extend(c.ancestors.iter().cloned());
        let mut imp = format!("// Generated.\n#pragma once\n{}{}", include(&impl_path, &decl_path), includes(&impl_path, &used));
        if !header_defs.is_empty() {
            imp += &format!("\n{}\n{header_defs}", header_imports.into_iter().collect::<String>());
        }
        modules.entry(c.module.clone()).or_default().push(impl_path.clone());
        out.push((decl_path, decl));
        out.push((impl_path, imp));
    }
    let mut functions: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>, String, String)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for f in &m.funcs {
        let func = function(f, cpp_name(f, "create"), "", "", Place::Free, unions);
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
    for (module, (imports, used, decls, defs)) in functions {
        let path = format!("{}.hpp", super::model::file_name(&format!("{module}/functions"), &mut taken));
        let text = format!("// Generated.\n#pragma once\n{}{}\n{}\n{decls}\n{defs}", include(&path, "ts_types.hpp"), includes(&path, &used), imports.into_iter().collect::<String>());
        out.push((path.clone(), text));
        modules.entry(module).or_default().push(path);
    }
    // The definitions of each module: a translation unit to compile once.
    for (module, (imports, defs)) in library {
        let path = format!("{module}.cpp");
        let headers: String = modules[&module].iter().map(|h| include(&path, h)).collect();
        out.push((path, format!("// Generated. The module's definitions: compile once, and link with the program.\n{headers}\n{}\n{defs}", imports.into_iter().collect::<String>())));
    }
    let mut all = String::from("// Generated.\n#pragma once\n");
    for (module, headers) in &modules {
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.hpp"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{module}.hpp\"\n");
    }
    out.push(("bindings.hpp".into(), all));
    Ok(out)
}

// Whether `u` declares the decoder of its variant: unions with the same
// members are the same type, and the first one does.
fn decoder_owner(u: &NamedUnion, unions: &[NamedUnion]) -> bool {
    let key = |u: &NamedUnion| u.parts.iter().map(|p| ty_out(p, unions)).collect::<Vec<_>>();
    u.type_params.is_empty() && u.decodable() && unions.iter().find(|o| o.type_params.is_empty() && o.decodable() && key(o) == key(u)).is_some_and(|o| o.name == u.name)
}

// A union as a type in its own definitions: `Either` or `Either<T>`.
fn union_type(u: &NamedUnion) -> String {
    if u.type_params.is_empty() { u.name.clone() } else { format!("{}<{}>", u.name, u.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) }
}

// `ts_types.hpp`: every type declared, so a struct may name any of them
// before it is defined. A template comes after what its defaults and
// constraints name.
fn declarations(m: &Model) -> String {
    let unions = &m.unions;
    let mut types = String::from("// Generated.\n#pragma once\n#include \"ts_runtime.hpp\"\n\n");
    for (id, (name, values)) in m.enums.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| ident(if v.is_empty() { "empty" } else { v })).map(|v| if seen.insert(v.clone()) { v } else { format!("{v}_{}", seen.len()) }).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\ntemplate <>\ninline constexpr int32_t ts::enum_id<{name}> = {id};\n", members.join(", "));
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
        format!("{}struct {name} : virtual ts::Handle {{\n{body}}};\n", head(params, false))
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
    for c in &m.callbacks {
        let text = format!("{}struct {};\n", head(&c.type_params, true), c.name);
        if c.type_params.is_empty() { types += &text } else { template(&c.name, &c.type_params, text) }
    }
    // Types declared elsewhere: handles, generic or not.
    // Promises are the runtime's.
    for name in m.opaque.iter().filter(|n| matches!(n.as_str(), "Promise" | "PromiseLike")) {
        types += &format!("template <class T = void>\nusing {name} = ts::Promise<T>;\n");
    }
    for name in m.opaque.iter().filter(|n| !matches!(n.as_str(), "Promise" | "PromiseLike")) {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<TParam> = (1..=arity).map(|i| TParam { name: format!("T{i}"), default: None, constraint: None }).collect();
        if params.is_empty() { types += &opaque(name, &params) } else { template(name, &params, opaque(name, &params)) }
    }
    // Unions, with the templates: either may name the other, in a default
    // argument (`Interval<StartType = DateArg<Date>>`) or a member (`Box<T>`).
    for u in unions {
        let parts: Vec<String> = u.parts.iter().map(|p| ty_out(p, unions)).collect();
        let mut text = format!("{}using {} = std::variant<{}>;\n", head(&u.type_params, false), u.name, parts.join(", "));
        // Defined with the union, used wherever one comes back.
        if u.decodable() {
            text += &format!("{}{} decode_{}(uint32_t h);\n", head(&u.type_params, false), union_type(u), u.name);
            // Unions with the same members are the same variant: one decoder.
            if decoder_owner(u, unions) {
                text += &format!("template <>\nstruct ts::decoder<::{0}> {{\n    static ::{0} from(uint32_t h);\n}};\n", u.name);
            }
        }
        let mut needs = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut needs));
        param_types(&u.type_params, &mut needs);
        needs.remove(&u.name);
        templates.push((u.name.clone(), text, needs));
    }
    let names: BTreeSet<String> = templates.iter().map(|t| t.0.clone()).collect();
    let mut done = BTreeSet::new();
    while !templates.is_empty() {
        let ready = templates.iter().position(|(_, _, needs)| needs.iter().all(|n| done.contains(n) || !names.contains(n))).unwrap_or(0);
        let (name, text, _) = templates.remove(ready);
        types += &text;
        done.insert(name);
    }
    // What each type extends, for constraints: see `ts::bases`.
    // A type's bases, and the types they name, come first: naming a
    // constrained template checks its constraint at once.
    let mut ordered: Vec<&Class> = Vec::new();
    let mut state: HashMap<&str, bool> = HashMap::new();
    fn visit<'a>(c: &'a Class, m: &'a Model, state: &mut HashMap<&'a str, bool>, out: &mut Vec<&'a Class>) {
        if state.insert(c.name.as_str(), false).is_some() {
            return;
        }
        let mut named = BTreeSet::new();
        c.bases.iter().for_each(|b| named_types(b, &mut named));
        for n in &named {
            if let Some(k) = m.classes.iter().find(|k| k.name == *n) {
                visit(k, m, state, out);
            }
        }
        out.push(c);
    }
    for c in &m.classes {
        visit(c, m, &mut state, &mut ordered);
    }
    for c in ordered.into_iter().filter(|c| !c.bases.is_empty()) {
        let args = if c.type_params.is_empty() { String::new() } else { format!("<{}>", c.type_params.iter().map(|p| p.name.as_str()).collect::<Vec<_>>().join(", ")) };
        let header = if c.type_params.is_empty() { "template <>\n".to_string() } else { head(&c.type_params, false) };
        types += &format!("{header}struct ts::bases<{}{args}> {{\n    using type = ts::list<{}>;\n}};\n", c.name, c.bases.iter().map(ty_in).collect::<Vec<_>>().join(", "));
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    inline constexpr ts::Key<{}, {}> {}{{{i}}};\n", k.name, ty_out(v, unions), ident(key))).collect();
        types += &format!("namespace {}Keys {{\n{}}}\n", k.name, keys.concat());
    }
    types
}

// What a callback argument arrives as: the glue's `to_wasm`.
fn raw_callback(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t",
        _ => "uint32_t",
    }
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
