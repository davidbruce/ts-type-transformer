// XML -> C++ bindings for a wasm32 module, built with the WASI SDK.
//
// Every JavaScript object is a value holding a handle, and every type
// inherits, virtually, from `ts::Handle`: inheritance and mixins are real C++
// inheritance, sharing one handle, and an HTMLDivElement is an Element.
// Each type has two headers. `X.decl.hpp` defines the struct, declaring its
// members, which needs only its bases complete; `X.hpp` defines the members
// inline, after including the declaration headers of every type they name.
// Strings go in as `std::string_view` and come back as `std::string`;
// nullables are `std::optional`, unions `std::variant`, callbacks lambdas, and
// keyed methods templates over typed keys:
// `el.addEventListener(HTMLElementEventMapKeys::click, [](PointerEvent e) {...})`.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::wasm::{Call, Class, Func, Model, NamedUnion, Ty};

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor", "bool", "break", "case", "catch", "char", "char8_t", "char16_t",
        "char32_t", "class", "compl", "concept", "const", "consteval", "constexpr", "constinit", "const_cast", "continue", "co_await", "co_return",
        "co_yield", "decltype", "default", "delete", "do", "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
        "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace", "new", "noexcept", "not", "not_eq", "nullptr", "operator",
        "or", "or_eq", "private", "protected", "public", "register", "reinterpret_cast", "requires", "return", "short", "signed", "sizeof", "static",
        "static_assert", "static_cast", "struct", "switch", "template", "this", "thread_local", "throw", "true", "try", "typedef", "typeid",
        "typename", "union", "unsigned", "using", "virtual", "void", "volatile", "wchar_t", "while", "xor", "xor_eq", "errno", "assert", "NULL",
    ];
    let s = super::wasm::ident(name);
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
        Ty::Array => "ts::Array".into(),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
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

// A type as a template argument: a value type, not a view.
fn ty_arg(t: &Ty) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_arg(inner)),
        t => ty_in(t),
    }
}

// `template <class E = Element>`: with defaults, where a template is first declared.
fn template_with_defaults(params: &[String], defaults: &[Option<Ty>]) -> String {
    if params.is_empty() {
        return String::new();
    }
    let ps: Vec<String> = params
        .iter()
        .enumerate()
        .map(|(i, p)| match defaults.get(i).cloned().flatten() {
            Some(d) => format!("class {p} = {}", ty_arg(&d)),
            None => format!("class {p}"),
        })
        .collect();
    // C++ allows defaults only at the end; otherwise none are given.
    let first_default = ps.iter().position(|p| p.contains(" = ")).unwrap_or(ps.len());
    if ps[first_default..].iter().all(|p| p.contains(" = ")) { format!("template <{}>\n", ps.join(", ")) } else { template(params) }
}

// `template <class T, class U>`, or nothing.
fn template(params: &[String]) -> String {
    if params.is_empty() { String::new() } else { format!("template <{}>\n", params.iter().map(|p| format!("class {p}")).collect::<Vec<_>>().join(", ")) }
}

// C++ type of a value coming back.
fn ty_out(t: &Ty, unions: &[NamedUnion]) -> String {
    match t {
        Ty::Str => "std::string".into(),
        Ty::Nullable(inner, _) => format!("std::optional<{}>", ty_out(inner, unions)),
        Ty::NamedUnion(n) if !decodable(n, unions) => "ts::Value".into(),
        Ty::NamedUnion(n) => n.clone(),
        t => ty_in(t),
    }
}

fn decodable(n: &str, unions: &[NamedUnion]) -> bool {
    unions.iter().any(|u| u.name == n && u.decodable())
}

// The wasm types a value crosses as.
fn raw(t: &Ty) -> Vec<&'static str> {
    match t {
        Ty::Void => vec![],
        Ty::Num | Ty::NumEnum(_) => vec!["double"],
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => vec!["int32_t"],
        Ty::Str => vec!["const char*", "size_t"],
        Ty::Nullable(inner, _) if **inner == Ty::Str => vec!["const char*", "size_t"],
        _ => vec!["uint32_t"],
    }
}

fn raw_ret(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
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
        Ty::Param(_) => vec![format!("ts::to_handle({x})")],
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Param(_)) => vec![format!("({x} ? ts::to_handle(*{x}) : 0)")],
        Ty::Nullable(..) => vec![format!("({x} ? {x}->h : 0)")],
        Ty::NamedUnion(_) => vec![format!("ts::to_handle({x})")],
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
        Ty::Str => format!("ts::read_string({r})"),
        Ty::Nullable(inner, _) => format!("({r} ? {}({}) : std::nullopt)", ty_out(t, unions), raise(inner, r, unions)),
        Ty::NamedUnion(n) if decodable(n, unions) => format!("decode_{n}({r})"),
        Ty::Obj(n) => format!("{n}(ts::handle, {r})"),
        Ty::Mapped(_) => format!("V(ts::handle, {r})"),
        Ty::Param(n) => format!("ts::from_handle<{n}>({r})"),
        Ty::Generic(..) => format!("{}(ts::handle, {r})", ty_in(t)),
        Ty::Array => format!("ts::Array(ts::handle, {r})"),
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

// The C++ name of a function, from what it calls.
fn cpp_name(f: &Func) -> String {
    let base = match &f.call {
        Call::Get(p) | Call::Set(p) => ident(p),
        Call::StaticGet(p) | Call::StaticSet(p) if f.module.is_empty() => ident(p.rsplit('.').next().unwrap_or(p)),
        Call::IndexGet => "at".into(),
        Call::IndexSet => "set_at".into(),
        Call::New(_) | Call::Create => "create".into(),
        _ => ident(&f.name),
    };
    // Overloads that differ only in a fixed argument are named after it.
    if f.fixed.is_empty() {
        base
    } else {
        let values: Vec<String> = f.fixed.iter().map(|(_, v)| super::wasm::ident(v.trim_matches('"'))).collect();
        format!("{base}_{}", values.join("_"))
    }
}

// One function, in its parts.
struct Function {
    // The wasm import it calls.
    import: String,
    // Its declaration in the class; empty for a free function.
    decl: String,
    // Its inline definition.
    def: String,
    // What C++ tells overloads apart by: name and parameter types.
    key: String,
    // The types it names.
    refs: BTreeSet<String>,
}

// `owner` is the class as written in a definition (`Box<T>`), with its template
// header (`template <class T>`).
fn function(f: &Func, owner: &str, owner_template: &str, place: Place, unions: &[NamedUnion]) -> Function {
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
        let name = format!("{}_", ident(name));
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
    let mut params_of = f.type_params.clone();
    let mut defaults = f.type_defaults.clone();
    if f.params.iter().any(|p| matches!(p.1, Ty::Key(_))) {
        params_of.insert(0, "V".into());
        defaults.insert(0, None);
    }
    // Defaults where the function is first declared: in the class, or at a free function.
    let first = template_with_defaults(&params_of, &defaults);
    let template = template(&params_of);
    let template = template.as_str();
    let (ret, name, params) = (ty_out(&f.ret, unions), cpp_name(f), params.join(", "));
    let (decl, def) = match place {
        Place::Member => (format!("    {}{ret} {name}({params}) const;\n", first.replace('\n', " ")), format!("{owner_template}{template}inline {ret} {owner}::{name}({params}) const {{ {body} }}\n")),
        Place::Static => (format!("    {}static {ret} {name}({params});\n", first.replace('\n', " ")), format!("{owner_template}{template}inline {ret} {owner}::{name}({params}) {{ {body} }}\n")),
        Place::Free => (String::new(), format!("{first}inline {ret} {name}({params}) {{ {body} }}\n")),
    };
    let key = format!("{}{name}({})", if place == Place::Static { "static " } else { "" }, f.params.iter().map(|p| ty_in(&p.1)).collect::<Vec<_>>().join(","));
    Function { import: import_decl, decl, def, key, refs }
}

// A function of a class template, where the class's own name means the
// class with its own parameters: `Box` is `Box<T>`.
fn with_self_args(f: &Func, c: &Class) -> Func {
    fn fix(t: &Ty, c: &Class) -> Ty {
        match t {
            Ty::Obj(n) if *n == c.name && !c.type_params.is_empty() => Ty::Generic(n.clone(), c.type_params.iter().map(|p| Ty::Param(p.clone())).collect()),
            Ty::Nullable(inner, u) => Ty::Nullable(Box::new(fix(inner, c)), *u),
            t => t.clone(),
        }
    }
    Func {
        type_params: f.type_params.clone(),
        type_defaults: f.type_defaults.clone(),
        module: f.module.clone(),
        import: f.import.clone(),
        name: f.name.clone(),
        call: f.call.clone(),
        params: f.params.iter().map(|(n, t)| (n.clone(), fix(t, c))).collect(),
        rest: f.rest,
        fixed: f.fixed.clone(),
        shared: f.shared.clone(),
        ret: fix(&f.ret, c),
    }
}

// The declared types a type names: classes, callbacks, unions and maps.
fn named_types(t: &Ty, out: &mut BTreeSet<String>) {
    match t {
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) | Ty::NamedUnion(n) | Ty::Key(n) | Ty::Mapped(n) => {
            out.insert(n.clone());
        }
        Ty::Nullable(inner, _) | Ty::Listener(inner) => named_types(inner, out),
        Ty::Generic(n, args) => {
            out.insert(n.clone());
            args.iter().for_each(|a| named_types(a, out));
        }
        _ => {}
    }
}

const RUNTIME: &str = r#"// Generated.
#pragma once
#include <cstddef>
#include <cstdint>
#include <optional>
#include <string>
#include <string_view>
#include <type_traits>
#include <utility>
#include <variant>

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
}

namespace ts {

// Tag for making a value from a handle.
struct handle_t {
    explicit handle_t() = default;
};
inline constexpr handle_t handle{};

// A JavaScript object. Zero is null.
struct Handle {
    uint32_t h = 0;
    Handle() = default;
    Handle(handle_t, uint32_t h) : h(h) {}
};

template <class T>
inline constexpr bool is_handle = std::is_base_of_v<Handle, std::decay_t<T>>;

inline std::string read_string(uint32_t h) {
    std::string s(ts_raw_js_string_len(h), '\0');
    ts_raw_js_string_read(h, s.data());
    return s;
}

// A union member, or any value, as a handle.
inline uint32_t to_handle(const std::string& s) { return ts_raw_js_from_string(s.data(), s.size()); }
inline uint32_t to_handle(std::string_view s) { return ts_raw_js_from_string(s.data(), s.size()); }
inline uint32_t to_handle(const char* s) { return to_handle(std::string_view(s)); }
inline uint32_t to_handle(double x) { return ts_raw_js_from_number(x); }
inline uint32_t to_handle(bool x) { return ts_raw_js_from_bool(x); }
template <class H, std::enable_if_t<is_handle<H>, int> = 0>
uint32_t to_handle(const H& v) { return v.h; }
template <class... T>
uint32_t to_handle(const std::variant<T...>& v) {
    return std::visit([](const auto& x) { return to_handle(x); }, v);
}

// A value of type T from a handle: a number, a flag, a string or an object.
template <class T>
T from_handle(uint32_t h) {
    if constexpr (std::is_same_v<T, double>) return ts_raw_js_to_number(h);
    else if constexpr (std::is_same_v<T, bool>) return ts_raw_js_to_bool(h) != 0;
    else if constexpr (std::is_same_v<T, std::string>) return read_string(h);
    else return T(handle, h);
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
    friend bool operator==(Value a, Value b) { return ts_raw_js_equals(a.h, b.h) != 0; }
};

struct Array : Handle {
    using Handle::Handle;
    static Array create() { return {handle, ts_raw_js_array_new()}; }
    uint32_t size() const { return ts_raw_js_array_length(h); }
    Value operator[](uint32_t i) const { return {handle, ts_raw_js_array_get(h, i)}; }
    template <class T>
    void push(const T& v) const { ts_raw_js_array_push(h, to_handle(v)); }
};

// A key of a map whose value has type V: `HTMLElementEventMapKeys::click`.
template <class Map, class V>
struct Key {
    int32_t index;
};

// A callback taking one object of type E, made from a lambda. Pass the same
// Listener to add and remove it.
template <class E>
struct Listener : Handle {
    using Handle::Handle;
    template <class F, std::enable_if_t<!is_handle<F>, int> = 0>
    Listener(F f) : Handle(handle, ts_raw_js_listener_from(reinterpret_cast<uintptr_t>(&call<F>), new F(std::move(f)))) {}

  private:
    template <class F>
    static void call(void* ctx, uint32_t ev) { (*static_cast<F*>(ctx))(E(handle, ev)); }
};

}  // namespace ts
"#;

// The bindings as files: `ts_runtime.hpp`; `ts_types.hpp` declaring every
// type; for each class `X.decl.hpp` and `X.hpp`; a header for each callback
// and union; the functions of each module; umbrella headers.
pub fn files(m: &Model) -> Vec<(String, String)> {
    let unions = &m.unions;
    let mut taken = HashMap::new();
    // Where each declared type is defined, without the extension.
    let mut home: HashMap<String, String> = HashMap::new();
    for c in &m.classes {
        home.insert(c.name.clone(), super::wasm::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
    }
    for name in m.callbacks.iter().map(|c| &c.name).chain(m.unions.iter().map(|u| &u.name)) {
        home.insert(name.clone(), super::wasm::file_name(&format!("types/{name}"), &mut taken));
    }
    let is_class = |n: &str| m.classes.iter().any(|c| c.name == n);
    // `#include` from one generated file to another.
    let include = |from: &str, to: &str| format!("#include \"{}{to}\"\n", "../".repeat(from.matches('/').count()));
    // What a header must include to use the given types: the struct of a class,
    // the full header of a callback or union.
    let includes = |from: &str, types: &BTreeSet<String>| -> String {
        types
            .iter()
            .filter_map(|t| home.get(t).map(|h| if is_class(t) { format!("{h}.decl.hpp") } else { format!("{h}.hpp") }))
            .filter(|h| h != from)
            .map(|h| include(from, &h))
            .collect()
    };

    let mut types = String::from("// Generated.\n#pragma once\n#include \"ts_runtime.hpp\"\n\n// Every type, declared: a struct may name any of them before it is defined.\n");
    // Enums first: they name nothing, and defaults of type parameters may name them.
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| ident(if v.is_empty() { "empty" } else { v })).map(|v| if seen.insert(v.clone()) { v } else { format!("{v}_{}", seen.len()) }).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    for c in &m.classes {
        types += &format!("{}struct {};\n", template_with_defaults(&c.type_params, &c.type_defaults), c.name);
    }
    for c in &m.callbacks {
        types += &format!("{}struct {};\n", template(&c.type_params), c.name);
    }
    // Types declared elsewhere: handles, generic or not.
    for name in &m.opaque {
        let arity = m.opaque_arity.get(name).copied().unwrap_or(0);
        let params: Vec<String> = (1..=arity).map(|i| format!("T{i}")).collect();
        types += &format!("{}struct {name} : virtual ts::Handle {{\n    {name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n}};\n", template(&params));
    }
    for u in unions {
        let parts: Vec<String> = u.parts.iter().map(|p| ty_out(p, unions)).collect();
        types += &format!("using {} = std::variant<{}>;\n", u.name, parts.join(", "));
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    inline constexpr ts::Key<{}, {}> {}{{{i}}};\n", k.name, ty_out(v, unions), ident(key))).collect();
        types += &format!("namespace {}Keys {{\n{}}}\n", k.name, keys.concat());
    }
    let mut out = vec![("ts_runtime.hpp".to_string(), RUNTIME.to_string()), ("ts_types.hpp".to_string(), types)];

    // A union: its alternatives, and how to tell which one came back.
    for u in unions {
        let path = format!("{}.hpp", home[&u.name]);
        let mut parts = BTreeSet::new();
        u.parts.iter().for_each(|p| named_types(p, &mut parts));
        let mut text = format!("// Generated.\n#pragma once\n{}{}", include(&path, "ts_types.hpp"), includes(&path, &parts));
        text += &format!("extern \"C\" TS_IMPORT({0}_tag) int32_t ts_raw_{0}_tag(uint32_t);\n", u.name);
        if u.decodable() {
            let cases: Vec<String> = u
                .parts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let value = match p {
                        Ty::Num => "ts_raw_js_to_number(h)".to_string(),
                        Ty::Bool => "ts_raw_js_to_bool(h) != 0".to_string(),
                        p => raise(p, "h", unions),
                    };
                    format!("        case {i}: return {}(std::in_place_index<{i}>, {value});\n", u.name)
                })
                .collect();
            text += &format!("inline {0} decode_{0}(uint32_t h) {{\n    switch (ts_raw_{0}_tag(h)) {{\n{1}    }}\n    __builtin_trap();\n}}\n", u.name, cases.concat());
        }
        out.push((path, text));
    }
    // A callback, made from a lambda that is kept alive for as long as JavaScript may call it.
    for c in &m.callbacks {
        let path = format!("{}.hpp", home[&c.name]);
        let mut used = BTreeSet::new();
        c.params.iter().for_each(|p| named_types(&p.1, &mut used));
        named_types(&c.ret, &mut used);
        used.remove(&c.name);
        let raw_args: String = c.params.iter().enumerate().map(|(i, (_, t))| format!(", {} a{i}", raw_callback(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| raise_callback(t, &format!("a{i}"), unions)).collect();
        let call = format!("(*static_cast<F*>(ctx))({})", args.join(", "));
        let result = match &c.ret {
            Ty::Void => format!("{call};"),
            t => format!("return {};", lower_callback(t, &call)),
        };
        let (name, ret, head) = (&c.name, raw_callback(&c.ret), format!("{}{}", include(&path, "ts_types.hpp"), includes(&path, &used)));
        let tpl = template(&c.type_params);
        let text = format!(
            "// Generated.\n#pragma once\n{head}extern \"C\" TS_IMPORT({name}_from) uint32_t ts_raw_{name}_from(uintptr_t, void*);\n\n\
             {tpl}struct {name} : virtual ts::Handle {{\n    {name}() = default;\n    {name}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n    \
             template <class F, std::enable_if_t<!ts::is_handle<F>, int> = 0>\n    \
             {name}(F f) : ts::Handle(ts::handle, ts_raw_{name}_from(reinterpret_cast<uintptr_t>(&call<F>), new F(std::move(f)))) {{}}\n\n  \
             private:\n    template <class F>\n    static {ret} call(void* ctx{raw_args}) {{ {result} }}\n}};\n"
        );
        out.push((path, text));
    }

    // Each class: a struct of declarations, and the definitions.
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
    let member_names = |c: &Class| -> BTreeSet<String> { c.funcs.iter().filter(|f| Class::has_self(f)).map(cpp_name).collect() };
    let mut modules: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for c in &m.classes {
        let base = &home[&c.name];
        let (decl_path, impl_path) = (format!("{base}.decl.hpp"), format!("{base}.hpp"));
        // A class template is written `Box<T>` where its members are defined.
        let class_tpl = template(&c.type_params);
        let owner = if c.type_params.is_empty() { c.name.clone() } else { format!("{}<{}>", c.name, c.type_params.join(", ")) };
        let mut seen = BTreeSet::new();
        let mut members = String::new();
        let mut defs = String::new();
        let mut imports = BTreeSet::new();
        let mut used = BTreeSet::new();
        for f in &c.funcs {
            let place = if Class::has_self(f) { Place::Member } else { Place::Static };
            // In a class template, the class itself is `Box<T>`.
            let f = &with_self_args(f, c);
            let func = function(f, &owner, &class_tpl, place, unions);
            // Overloads TypeScript tells apart and C++ cannot are the same C++ function.
            if !seen.insert(func.key) {
                continue;
            }
            members += &func.decl;
            defs += &func.def;
            imports.insert(func.import);
            used.extend(func.refs);
        }
        for (name, value) in &c.consts {
            let ty = if value.starts_with('"') { "std::string_view" } else { "double" };
            members += &format!("    static constexpr {ty} {} = {value};\n", ident(name));
        }
        // A name in two unrelated bases is ambiguous in C++. A more derived
        // ancestor's member already hides its own ancestors' (`Element::textContent`
        // over `Node::textContent`), as in TypeScript; the rest are brought in as overloads.
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
        let mut usings = String::new();
        let generic = |n: &str| m.classes.iter().any(|k| k.name == n && !k.type_params.is_empty());
        for (name, hs) in holders.iter().filter(|(n, _)| !own.contains(*n)) {
            let visible: Vec<&&String> = hs.iter().filter(|h| !hs.iter().any(|o| o != *h && ancestors(o).contains(**h))).collect();
            // A generic base would need its arguments here; left ambiguous.
            if visible.len() > 1 && !visible.iter().any(|h| generic(h)) {
                for h in visible {
                    usings += &format!("    using {h}::{name};\n");
                }
            }
        }
        let bases: Vec<String> = if c.bases.is_empty() { vec!["virtual ts::Handle".into()] } else { c.bases.iter().map(|b| format!("virtual {}", ty_in(b))).collect() };
        let mut decl = format!("// Generated.\n#pragma once\n{}", include(&decl_path, "ts_types.hpp"));
        for a in &c.ancestors {
            if let Some(h) = home.get(a) {
                decl += &include(&decl_path, &format!("{h}.decl.hpp"));
            }
        }
        decl += &format!(
            "\n{class_tpl}struct {0} : {1} {{\n    {0}() = default;\n    {0}(ts::handle_t, uint32_t h) : ts::Handle(ts::handle, h) {{}}\n{usings}{members}}};\n",
            c.name,
            bases.join(", ")
        );
        used.remove(&c.name);
        // Inherited members are defined with the ancestors.
        let inherited: String = c.ancestors.iter().filter_map(|a| home.get(a)).map(|h| include(&impl_path, &format!("{h}.hpp"))).collect();
        let imp = format!("// Generated.\n#pragma once\n{}{inherited}{}\n{}\n{defs}", include(&impl_path, &decl_path), includes(&impl_path, &used), imports.into_iter().collect::<String>());
        modules.entry(c.module.clone()).or_default().push(impl_path.clone());
        out.push((decl_path, decl));
        out.push((impl_path, imp));
    }
    let mut functions: BTreeMap<String, (BTreeSet<String>, BTreeSet<String>, String)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for f in &m.funcs {
        let func = function(f, "", "", Place::Free, unions);
        if !seen.insert(func.key) {
            continue;
        }
        let entry = functions.entry(f.module.clone()).or_default();
        entry.0.insert(func.import);
        entry.1.extend(func.refs);
        entry.2 += &func.def;
    }
    for (module, (imports, used, defs)) in functions {
        let path = format!("{}.hpp", super::wasm::file_name(&format!("{module}/functions"), &mut taken));
        let text = format!("// Generated.\n#pragma once\n{}{}\n{}\n{defs}", include(&path, "ts_types.hpp"), includes(&path, &used), imports.into_iter().collect::<String>());
        out.push((path.clone(), text));
        modules.entry(module).or_default().push(path);
    }
    let mut all = String::from("// Generated.\n#pragma once\n");
    for (module, headers) in &modules {
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.hpp"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{module}.hpp\"\n");
    }
    out.push(("bindings.hpp".into(), all));
    out
}

// What a callback argument arrives as: the glue's `to_wasm`.
fn raw_callback(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        _ => "uint32_t",
    }
}

fn raise_callback(t: &Ty, a: &str, unions: &[NamedUnion]) -> String {
    match t {
        Ty::NamedUnion(_) | Ty::Union(_) => format!("ts::Value(ts::handle, {a})"),
        t => raise(t, a, unions),
    }
}

// What a callback returns: a number, a flag, or a handle.
fn lower_callback(t: &Ty, x: &str) -> String {
    match t {
        Ty::Num => x.into(),
        Ty::NumEnum(_) => format!("double({x})"),
        Ty::Bool => format!("({x} ? 1 : 0)"),
        Ty::Enum(_) => format!("int32_t({x})"),
        _ => format!("ts::to_handle({x})"),
    }
}
