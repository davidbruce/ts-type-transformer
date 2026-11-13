// XML -> C++ bindings for a wasm32 module, built with the WASI SDK.
//
// Every JavaScript object is a value holding a handle. A type's own members
// are in a template, `<T>_members<Self>`; a class inherits the templates of
// itself and all its ancestors, and converts implicitly to each ancestor. That
// is multiple inheritance (the DOM's mixins) without repeated state.
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
    }
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
        Ty::Array => format!("ts::Array(ts::handle, {r})"),
        _ => format!("ts::Value(ts::handle, {r})"),
    }
}

#[derive(PartialEq)]
enum Place {
    // In a `_members` template: `self` is the handle of the object it is called on.
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

// One function: its raw import declaration, and its C++ definition.
fn function(f: &Func, place: Place, indent: &str, unions: &[NamedUnion], deferrable: &[&str]) -> (String, String, String) {
    let mut raw_params: Vec<&str> = Vec::new();
    let mut args: Vec<String> = Vec::new();
    if Class::has_self(f) {
        raw_params.push("uint32_t");
        args.push("ts_h()".into());
    }
    // Types that may name a class not yet complete are deferred (`ts::dep`).
    let defer = |t: String| -> String {
        if matches!(t.as_str(), "void" | "double" | "bool" | "std::string" | "std::string_view" | "ts::Value" | "ts::Array" | "V") { t } else { format!("ts::dep<{t}, D>") }
    };
    let mut params = Vec::new();
    for (name, t) in &f.params {
        let name = format!("{}_", ident(name));
        // A key is deduced: it says what V is.
        let ty = if matches!(t, Ty::Key(_)) { ty_in(t) } else { defer(ty_in(t)) };
        params.push(format!("{ty} {name}"));
        raw_params.extend(raw(t));
        args.extend(lower(t, &name));
    }
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
    let decl = format!("extern \"C\" TS_IMPORT({import}) {} ts_raw_{import}({});\n", raw_ret(&f.ret), raw_params.join(", "));
    let call = format!("ts_raw_{import}({})", args.join(", "));
    let body = match &f.ret {
        Ty::Void => format!("{call};"),
        t => format!("auto r = {call}; return {};", raise(t, "r", unions)),
    };
    // Even a template's body is checked where it is defined, but for what depends on D.
    let body = deferred(&body, deferrable, "D");
    let keyed = f.params.iter().any(|p| matches!(p.1, Ty::Key(_)));
    let template = if keyed { "template <class V, class D = void> " } else { "template <class D = void> " };
    let (qual, stat) = match place {
        Place::Member => (" const", ""),
        Place::Static => ("", "static "),
        Place::Free => ("", "inline "),
    };
    let def = format!("{indent}{template}{stat}{} {}({}){qual} {{ {body} }}\n", defer(ty_out(&f.ret, unions)), cpp_name(f), params.join(", "));
    // What C++ tells overloads apart by: name and parameter types.
    let key = format!("{stat}{}({})", cpp_name(f), f.params.iter().map(|p| ty_in(&p.1)).collect::<Vec<_>>().join(","));
    (decl, def, key)
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

// `dep<T, D>` is T, but depends on D: a member declared with it is not looked
// at until used, when every class it names is complete.
template <class T, class D>
struct dep_t {
    using type = T;
};
template <class T, class D>
using dep = typename dep_t<T, D>::type;

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
// class, enum, union, callback and key; a header per declaration in its
// module's folder, including those of its ancestors; umbrella headers.
pub fn files(m: &Model) -> Vec<(String, String)> {
    let mut types = String::from("// Generated.\n#pragma once\n#include \"ts_runtime.hpp\"\n#include <functional>\n\n");
    let classes: Vec<&str> = m.classes.iter().map(|c| c.name.as_str()).chain(m.opaque.iter().map(String::as_str)).chain(m.callbacks.iter().map(|c| c.name.as_str())).collect();
    for c in &m.classes {
        types += &format!("struct {};\n", c.name);
    }
    for name in &m.opaque {
        types += &format!("struct {name} : ts::Handle {{\n    using ts::Handle::Handle;\n}};\n");
    }
    // Callbacks are defined further down; unions may name them first.
    for c in &m.callbacks {
        types += &format!("struct {};\n", c.name);
    }
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| ident(if v.is_empty() { "empty" } else { v })).map(|v| if seen.insert(v.clone()) { v } else { format!("{v}_{}", seen.len()) }).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        types += &format!("enum class {name} : int32_t {{ {} }};\n", members.join(", "));
    }
    // A union is a variant; one coming back is told apart by the runtime tests.
    for u in &m.unions {
        let parts: Vec<String> = u.parts.iter().map(|p| ty_out(p, &m.unions)).collect();
        types += &format!("using {} = std::variant<{}>;\n", u.name, parts.join(", "));
        types += &format!("extern \"C\" TS_IMPORT({0}_tag) int32_t ts_raw_{0}_tag(uint32_t);\n", u.name);
        if u.decodable() {
            let cases: Vec<String> = u
                .parts
                .iter()
                .enumerate()
                .map(|(i, p)| {
                    let value = match p {
                        Ty::Num => "ts_raw_js_to_number(h)".to_string(),
                        Ty::Bool => "ts_raw_js_to_bool(h) != 0".to_string(),
                        p => raise(p, "h", &m.unions),
                    };
                    format!("case {i}: return ts::dep<{}, D>(std::in_place_index<{i}>, {value});", u.name)
                })
                .collect();
            let body = deferred(&format!("switch (ts_raw_{0}_tag(h)) {{ {1} }}", u.name, cases.join(" ")), &classes, "D");
            types += &format!("template <class D = void>\nts::dep<{0}, D> decode_{0}(uint32_t h) {{\n    {body}\n    __builtin_trap();\n}}\n", u.name);
        }
    }
    // A callback is made from a lambda, kept alive for as long as JavaScript may call it.
    for c in &m.callbacks {
        let raw_args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| format!("{} a{i}", raw_callback(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| raise_callback(t, &format!("a{i}"), &m.unions)).collect();
        let result = match &c.ret {
            Ty::Void => format!("(*static_cast<F*>(ctx))({});", args.join(", ")),
            t => format!("return {};", lower_callback(t, &format!("(*static_cast<F*>(ctx))({})", args.join(", ")))),
        };
        let result = deferred(&result, &classes, "F");
        types += &format!(
            "extern \"C\" TS_IMPORT({0}_from) uint32_t ts_raw_{0}_from(uintptr_t, void*);\n\
             struct {0} : ts::Handle {{\n    using ts::Handle::Handle;\n    \
             template <class F, std::enable_if_t<!ts::is_handle<F>, int> = 0>\n    \
             {0}(F f) : ts::Handle(ts::handle, ts_raw_{0}_from(reinterpret_cast<uintptr_t>(&call<F>), new F(std::move(f)))) {{}}\n\n  \
             private:\n    template <class F>\n    static {1} call(void* ctx{2}) {{ {result} }}\n}};\n",
            c.name,
            raw_callback(&c.ret),
            raw_args.iter().map(|a| format!(", {a}")).collect::<String>()
        );
    }
    // A map's keys, each typed by the value it selects.
    for k in &m.maps {
        let keys: Vec<String> = k.keys.iter().zip(&k.values).enumerate().map(|(i, (key, v))| format!("    inline constexpr ts::Key<{}, {}> {}{{{i}}};\n", k.name, ty_out(v, &m.unions), ident(key))).collect();
        types += &format!("namespace {}Keys {{\n{}}}\n", k.name, keys.concat());
    }

    let mut out = vec![("ts_runtime.hpp".to_string(), RUNTIME.to_string()), ("ts_types.hpp".to_string(), types)];
    let mut taken = HashMap::new();
    let path_of: HashMap<&str, String> = m.classes.iter().map(|c| (c.name.as_str(), format!("{}.hpp", super::wasm::file_name(&format!("{}/{}", c.module, c.name), &mut taken)))).collect();
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
        let path = &path_of[c.name.as_str()];
        let up = "../".repeat(c.module.split('/').count());
        let mut text = format!("// Generated.\n#pragma once\n#include \"{up}ts_types.hpp\"\n");
        for a in &c.ancestors {
            if let Some(p) = path_of.get(a.as_str()) {
                text += &format!("#include \"{up}{p}\"\n");
            }
        }
        let mut decls = BTreeSet::new();
        let mut members = String::new();
        let mut statics = String::new();
        let mut seen = BTreeSet::new();
        for f in &c.funcs {
            let place = if Class::has_self(f) { Place::Member } else { Place::Static };
            let (decl, def, key) = function(f, place, "    ", &m.unions, &classes);
            // Overloads TypeScript tells apart and C++ cannot are the same C++ function.
            if !seen.insert(key) {
                continue;
            }
            decls.insert(decl);
            if Class::has_self(f) {
                members += &def;
            } else {
                statics += &def;
            }
        }
        for (name, value) in &c.consts {
            let ty = if value.starts_with('"') { "std::string_view" } else { "double" };
            statics += &format!("    static constexpr {ty} {} = {value};\n", ident(name));
        }
        text += &decls.into_iter().collect::<String>();
        text += &format!(
            "\n// The members {0} declares, for {0} and every type that inherits from it.\ntemplate <class Self>\nstruct {0}_members {{\n{members}\n  private:\n    uint32_t ts_h() const {{ return static_cast<const Self&>(*this).h; }}\n}};\n\n",
            c.name
        );
        // The class: its own members and every ancestor's, and a conversion to each ancestor.
        let all = ancestors(&c.name);
        let with_members: Vec<&String> = std::iter::once(&c.name).chain(all.iter().filter(|a| path_of.contains_key(a.as_str()))).collect();
        let bases: Vec<String> = with_members.iter().map(|a| format!("{a}_members<{}>", c.name)).collect();
        text += &format!("struct {} : ts::Handle{} {{\n    using ts::Handle::Handle;\n", c.name, bases.iter().map(|b| format!(", {b}")).collect::<String>());
        // A name in several bases is ambiguous in C++: the class's own hides its ancestors',
        // as in TypeScript; otherwise all are brought in as overloads.
        let own = member_names(c);
        let mut holders: BTreeMap<String, Vec<&String>> = BTreeMap::new();
        for a in &with_members {
            if let Some(k) = m.classes.iter().find(|k| k.name == **a) {
                for n in member_names(k) {
                    holders.entry(n).or_default().push(a);
                }
            }
        }
        for (name, hs) in holders.iter().filter(|(_, hs)| hs.len() > 1) {
            if own.contains(name) {
                text += &format!("    using {}_members<{}>::{name};\n", c.name, c.name);
            } else {
                // A more derived ancestor's member hides its own ancestors' (`Element::textContent` over `Node::textContent`).
                for h in hs.iter().filter(|h| !hs.iter().any(|o| o != *h && ancestors(o).contains(**h))) {
                    text += &format!("    using {h}_members<{}>::{name};\n", c.name);
                }
            }
        }
        for a in &all {
            text += &format!("    operator {a}() const {{ return {a}(ts::handle, h); }}\n");
        }
        text += &statics;
        text += "};\n";
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path.clone(), text));
    }
    let mut functions: BTreeMap<String, (BTreeSet<String>, String)> = BTreeMap::new();
    let mut seen = BTreeSet::new();
    for f in &m.funcs {
        let (decl, def, key) = function(f, Place::Free, "", &m.unions, &classes);
        if !seen.insert(key) {
            continue;
        }
        let entry = functions.entry(f.module.clone()).or_default();
        entry.0.insert(decl);
        entry.1 += &def;
    }
    for (module, (decls, defs)) in functions {
        let up = "../".repeat(module.split('/').count());
        let path = format!("{}.hpp", super::wasm::file_name(&format!("{module}/functions"), &mut taken));
        out.push((path.clone(), format!("// Generated.\n#pragma once\n#include \"{up}ts_types.hpp\"\n\n{}\n{defs}", decls.into_iter().collect::<String>())));
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

// Code in `ts_types.hpp` runs before the classes it makes are complete: each
// construction is made to depend on the template parameter `d`, so it is only
// checked when used.
fn deferred(code: &str, classes: &[&str], d: &str) -> String {
    let mut out = code.to_string();
    for c in classes {
        out = out.replace(&format!("std::optional<{c}>("), &format!("ts::dep<std::optional<{c}>, {d}>("));
        out = out.replace(&format!(" {c}(ts::handle"), &format!(" ts::dep<{c}, {d}>(ts::handle"));
        out = out.replace(&format!("({c}(ts::handle"), &format!("(ts::dep<{c}, {d}>(ts::handle"));
    }
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
