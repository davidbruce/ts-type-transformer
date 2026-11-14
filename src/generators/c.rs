// XML -> C header for a wasm32 module. No inheritance, generics, overloads,
// optional parameters or unions: the config must have lowered them all.

use super::wasm::{Call, Class, Func, Model, NamedUnion, Ty};

thread_local! {
    // The unions of the model being generated that values can be decoded to.
    static DECODABLE: std::cell::RefCell<std::collections::HashSet<String>> = std::cell::RefCell::new(Default::default());
}

fn decodable(n: &str) -> bool {
    DECODABLE.with(|d| d.borrow().contains(n))
}

const T: &str = "C";

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &[
        "auto", "bool", "break", "case", "char", "const", "continue", "default", "do", "double", "else", "enum", "extern", "false", "float", "for",
        "goto", "if", "inline", "int", "long", "register", "restrict", "return", "self", "short", "signed", "sizeof", "static", "struct", "switch",
        "true", "typedef", "union", "unsigned", "void", "volatile", "while",
    ];
    if KEYWORDS.contains(&name) { format!("{name}_") } else { name.to_string() }
}

fn raw(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        // An optional flag or enum is -1 when absent.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "int32_t",
        _ => "uint32_t",
    }
}

fn typed(t: &Ty, param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "double".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if param => "const char*".into(),
        Ty::Str => "JsString".into(),
        Ty::Any => "JsValue".into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArray_{}", element(e)),
        Ty::Tuple(es) => format!("JsTuple_{}", es.iter().map(element).collect::<Vec<_>>().join("_")),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // An optional number or flag says whether it is there; an optional enum is `E_absent` when it is not.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsOptionalDouble".into(),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => "JsOptionalBool".into(),
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param),
        // A union a handle can be decoded to: a tagged struct. Any other is a value.
        Ty::NamedUnion(n) if decodable(n) => n.clone(),
        Ty::NamedUnion(_) => "JsValue".into(),
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must name or erase unions"),
        Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => panic!("{T} generator cannot express keyed generics: the target config must expand them"),
        Ty::Param(_) | Ty::Generic(..) => panic!("{T} generator cannot express generics: the target config must erase them"),
    }
}

// An array's element type, as a name: `double`, `JsString`, `Element`, `JsArray_double`.
fn element(t: &Ty) -> String {
    match t {
        Ty::Str => "JsString".into(),
        t => typed(t, false),
    }
}

// The typed arrays and tuples a type names, innermost first.
fn arrays(t: &Ty, out: &mut Vec<Ty>) {
    match t {
        Ty::ArrayOf(e) => arrays(e, out),
        Ty::Tuple(es) => es.iter().for_each(|e| arrays(e, out)),
        Ty::Nullable(inner, _) => return arrays(inner, out),
        _ => return,
    }
    if !out.contains(t) {
        out.push(t.clone());
    }
}

// A union's member name: `number`, `string`, `Element`, `array`.
fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
        t => panic!("{T} generator: a union member it cannot hold: {t:?}"),
    }
}

// A union: a struct with a tag and its members, tag constants, and
// conversions to and from handles: `Id_from_handle` takes a handle a function
// returned; `Id_to_handle` gives one the caller releases.
fn union_type(u: &NamedUnion, m: &Model) -> (String, String) {
    // An enum of strings crosses as a handle by its place among the enums.
    let enum_id = |e: &str| m.enums.iter().position(|x| x.0 == e).unwrap_or(0);
    let n = &u.name;
    let fields: Vec<String> = u.parts.iter().map(|p| format!("{} {};", if *p == Ty::Str { "JsString".to_string() } else { typed(p, false) }, union_field(p))).collect();
    let tags: Vec<String> = u.parts.iter().enumerate().map(|(i, p)| format!("{n}_{} = {i}", union_field(p))).collect();
    let decl = format!(
        "// {n}: `tag` says which member it holds; -1 is none.\ntypedef struct {{ int32_t tag; union {{ {} }} as; }} {n};\nenum {{ {} }};\n",
        fields.join(" "),
        tags.join(", ")
    );
    let to: String = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let f = union_field(p);
            match p {
                Ty::Num => format!(" case {i}: return ts_raw_js_from_number(u.as.{f});"),
                Ty::Bool => format!(" case {i}: return ts_raw_js_from_bool(u.as.{f});"),
                Ty::Enum(e) => format!(" case {i}: return ts_raw_js_enum_value({}, u.as.{f});", enum_id(e)),
                _ => format!(" case {i}: ts_raw_js_retain(u.as.{f}.h); return u.as.{f}.h;"),
            }
        })
        .collect();
    let from: String = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let f = union_field(p);
            match p {
                Ty::Num => format!(" case {i}: u.as.{f} = ts_raw_js_to_number(h); ts_raw_js_release(h); break;"),
                Ty::Bool => format!(" case {i}: u.as.{f} = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); break;"),
                Ty::Str => format!(" case {i}: u.as.{f} = (JsString){{ h }}; break;"),
                Ty::Enum(e) => format!(" case {i}: u.as.{f} = ({e})ts_raw_js_enum_index({}, h); ts_raw_js_release(h); break;", enum_id(e)),
                p => format!(" case {i}: u.as.{f} = ({}){{ h }}; break;", typed(p, false)),
            }
        })
        .collect();
    let functions = format!(
        "TS_IMPORT({n}_tag) int32_t ts_raw_{n}_tag(uint32_t, int32_t);\n\
         static inline uint32_t {n}_to_handle({n} u) {{ switch (u.tag) {{{to} }} return 0; }}\n\
         static inline {n} {n}_from_handle(uint32_t h) {{ {n} u; u.tag = ts_raw_{n}_tag(h, 0); switch (u.tag) {{{from} }} return u; }}\n"
    );
    // Made from a member: `Value_of_number(42)`, or by its type, `Value_of(widget)`.
    let field_type = |p: &Ty| if *p == Ty::Str { "JsString".to_string() } else { typed(p, false) };
    let mut functions = functions;
    for p in &u.parts {
        let f = union_field(p);
        functions += &format!("static inline {n} {n}_of_{f}({} v) {{ return ({n}){{ .tag = {n}_{f}, .as.{f} = v }}; }}\n", field_type(p));
    }
    let mut seen = std::collections::BTreeSet::new();
    let choices: Option<Vec<String>> = u
        .parts
        .iter()
        .map(|p| {
            let types = if *p == Ty::Str { Some(vec!["JsString".to_string()]) } else { generic_types(p) }?;
            types.iter().all(|t| seen.insert(t.clone())).then(|| types.iter().map(|t| format!("{t}: {n}_of_{}", union_field(p))).collect::<Vec<_>>().join(", "))
        })
        .collect();
    if let Some(choices) = choices {
        functions += &format!("#define {n}_of(x) _Generic((x), {})(x)\n", choices.join(", "));
    }
    (decl, functions)
}

// A tuple: `JsTuple_double_JsString`, made by `_new(1, "one")`, read by `_get_0`.
// What a getter returns is the caller's to release; a number or flag is read and released.
fn tuple_type(es: &[Ty]) -> String {
    let name = typed(&Ty::Tuple(es.to_vec()), false);
    let params: Vec<String> = es.iter().enumerate().map(|(i, e)| format!("{} v{i}", if *e == Ty::Str { "const char*".into() } else { typed(e, false) })).collect();
    let pushes: String = es
        .iter()
        .enumerate()
        .map(|(i, e)| match e {
            Ty::Num => format!(" {{ uint32_t h = ts_raw_js_from_number(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Bool => format!(" {{ uint32_t h = ts_raw_js_from_bool(v{i}); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            Ty::Str => format!(" {{ uint32_t h = ts_raw_js_from_string(v{i}, ts_strlen(v{i})); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h); }}"),
            _ => format!(" ts_raw_js_array_push(a.h, v{i}.h);"),
        })
        .collect();
    let mut out = format!("typedef struct {{ uint32_t h; }} {name};\nstatic inline {name} {name}_new({}) {{ {name} a = {{ ts_raw_js_array_new() }};{pushes} return a; }}\n", params.join(", "));
    for (i, e) in es.iter().enumerate() {
        let elem = typed(e, false);
        let get = match e {
            Ty::Num => "double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(),
            Ty::Bool => "bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(),
            _ => format!("return ({elem}){{ h }};"),
        };
        out += &format!("static inline {elem} {name}_get_{i}({name} a) {{ uint32_t h = ts_raw_js_array_get(a.h, {i}); {get} }}\n");
    }
    out + &format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}\n")
}

// A typed array: `JsArray_double`, with `_new`, `_length`, `_get` and `_push`.
// What `_get` returns is the caller's to release; a number or flag is read and released.
fn array_type(e: &Ty) -> String {
    let (name, elem) = (format!("JsArray_{}", element(e)), typed(e, false));
    let (get, push_param, push) = match e {
        Ty::NamedUnion(n) => (format!("return {n}_from_handle(h);"), format!("{n} v"), format!("uint32_t h = {n}_to_handle(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);")),
        Ty::Num => ("double x = ts_raw_js_to_number(h); ts_raw_js_release(h); return x;".to_string(), "double v".to_string(), "uint32_t h = ts_raw_js_from_number(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        Ty::Bool => ("bool x = ts_raw_js_to_bool(h) != 0; ts_raw_js_release(h); return x;".to_string(), "bool v".to_string(), "uint32_t h = ts_raw_js_from_bool(v); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        Ty::Str => ("return (JsString){ h };".to_string(), "const char* v".to_string(), "uint32_t h = ts_raw_js_from_string(v, ts_strlen(v)); ts_raw_js_array_push(a.h, h); ts_raw_js_release(h);".to_string()),
        _ => (format!("return ({elem}){{ h }};"), format!("{elem} v"), "ts_raw_js_array_push(a.h, v.h);".to_string()),
    };
    [
        format!("typedef struct {{ uint32_t h; }} {name};"),
        format!("static inline {name} {name}_new(void) {{ return ({name}){{ ts_raw_js_array_new() }}; }}"),
        format!("static inline uint32_t {name}_length({name} a) {{ return ts_raw_js_array_length(a.h); }}"),
        format!("static inline {elem} {name}_get({name} a, uint32_t i) {{ uint32_t h = ts_raw_js_array_get(a.h, i); {get} }}"),
        format!("static inline void {name}_push({name} a, {push_param}) {{ {push} }}"),
        // The same array, untyped.
        format!("static inline JsArray {name}_any({name} a) {{ return (JsArray){{ a.h }}; }}"),
    ]
    .map(|l| l + "\n")
    .concat()
}

fn func(out: &mut String, owner: Option<&str>, f: &Func) {
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        raw_params.push("uint32_t".to_string());
        params.push(format!("{} self", owner.unwrap()));
        args.push("self.h".to_string());
    }
    // A union argument is lent as a handle, released after the call.
    let mut lent = Vec::new();
    for (name, t) in &f.params {
        let name = ident(name);
        params.push(format!("{} {name}", typed(t, true)));
        let union = match t {
            Ty::NamedUnion(n) => Some(n),
            Ty::Nullable(inner, _) => match &**inner {
                Ty::NamedUnion(n) => Some(n),
                _ => None,
            },
            _ => None,
        };
        if let Some(n) = union.filter(|n| decodable(n)) {
            raw_params.push("uint32_t".into());
            args.push(format!("lent_{name}"));
            lent.push(format!("uint32_t lent_{name} = {n}_to_handle({name});"));
            continue;
        }
        match t {
            Ty::Str => {
                raw_params.extend(["const char*".to_string(), "uint32_t".to_string()]);
                args.extend([name.clone(), format!("ts_strlen({name})")]);
            }
            Ty::Nullable(inner, _) if **inner == Ty::Str => {
                raw_params.extend(["const char*".to_string(), "uint32_t".to_string()]);
                args.extend([name.clone(), format!("({name} ? ts_strlen({name}) : 0)")]);
            }
            Ty::Num | Ty::Bool | Ty::Enum(_) => {
                raw_params.push(raw(t).into());
                args.push(name);
            }
            Ty::NumEnum(_) => {
                raw_params.push(raw(t).into());
                args.push(format!("(double){name}"));
            }
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => {
                raw_params.extend(["int32_t".to_string(), "double".to_string()]);
                args.extend([format!("{name}.has"), format!("{name}.value")]);
            }
            Ty::Nullable(inner, _) if **inner == Ty::Bool => {
                raw_params.push("int32_t".into());
                args.push(format!("({name}.has ? ({name}.value ? 1 : 0) : -1)"));
            }
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => {
                raw_params.push("int32_t".into());
                args.push(format!("(int32_t){name}"));
            }
            _ => {
                raw_params.push(raw(t).into());
                args.push(format!("{name}.h"));
            }
        }
    }
    let void = |v: &Vec<String>| if v.is_empty() { "void".to_string() } else { v.join(", ") };
    let name = match owner {
        Some(o) => format!("{o}_{}", f.name),
        None => f.name.clone(),
    };
    // A shared import takes the index of this function's fixed arguments after `self`.
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(Class::has_self(f));
            raw_params.insert(at, "int32_t".into());
            args.insert(at, k.to_string());
            shared.as_str()
        }
        None => f.import.as_str(),
    };
    let mut call = format!("ts_raw_{import}({})", args.join(", "));
    // With lent arguments: the call, their release, then the result.
    let mut pre = lent.concat();
    if !lent.is_empty() {
        let release: String = lent.iter().map(|l| format!(" ts_raw_js_release({});", l.split(' ').nth(1).unwrap())).collect();
        if f.ret == Ty::Void {
            pre += &format!(" {call};{release}");
            call = String::new();
        } else {
            pre += &format!(" {} raw_result = {call};{release}", raw(&f.ret));
            call = "raw_result".into();
        }
    }
    let union_ret = match &f.ret {
        Ty::NamedUnion(n) => Some((n, false)),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => Some((n, true)),
            _ => None,
        },
        _ => None,
    };
    let body = match &f.ret {
        Ty::Void if call.is_empty() => String::new(),
        _ if union_ret.is_some_and(|(n, _)| decodable(n)) => {
            let (n, optional) = union_ret.unwrap();
            if optional { format!("uint32_t h = {call}; if (h) return {n}_from_handle(h); {n} none = {{ -1 }}; return none;") } else { format!("return {n}_from_handle({call});") }
        }
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(e) => format!("return ({e}){call};"),
        Ty::NumEnum(e) => format!("return ({e})(int32_t){call};"),
        // An optional number comes back as a handle, read and released.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => {
            format!("uint32_t h = {call}; JsOptionalDouble r = {{ h != 0, h ? ts_raw_js_to_number(h) : 0 }}; if (h) ts_raw_js_release(h); return r;")
        }
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("int32_t r = {call}; return (JsOptionalBool){{ r >= 0, r > 0 }};"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("return ({}){call};", typed(inner, false)),
        t => format!("return ({}){{ {call} }};", typed(t, false)),
    };
    let decl = format!("TS_IMPORT({import}) {} ts_raw_{import}({});\n", raw(&f.ret), void(&raw_params));
    // Declared once per header, though several functions use it.
    // Only a shared import can repeat; checking others would be quadratic.
    if f.shared.is_none() || !out.contains(&decl) {
        *out += &decl;
    }
    *out += &format!("static inline {} {name}({}) {{ {pre}{body} }}\n", typed(&f.ret, false), void(&params));
}

const RUNTIME: &str = "// Generated.
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#define TS_IMPORT(name) __attribute__((import_module(\"env\"), import_name(#name)))

// `name__N`, by the number of arguments: how trailing optional arguments are left out.
#define TS_COUNT(...) TS_COUNT_(__VA_ARGS__, 15, 14, 13, 12, 11, 10, 9, 8, 7, 6, 5, 4, 3, 2, 1, 0)
#define TS_COUNT_(a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, n, ...) n
#define TS_PASTE(name, n) TS_PASTE_(name, n)
#define TS_PASTE_(name, n) name##__##n
#define TS_BY_COUNT(name, ...) TS_PASTE(name, TS_COUNT(__VA_ARGS__))

// Handles to JavaScript values. Zero is null.
typedef struct { uint32_t h; } JsValue;
typedef struct { uint32_t h; } JsString;
typedef struct { uint32_t h; } JsArray;
static inline uint32_t ts_strlen(const char* s) { uint32_t n = 0; while (s[n]) n++; return n; }
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_bool) uint32_t ts_raw_js_from_bool(int32_t);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, uint32_t);
TS_IMPORT(js_to_number) double ts_raw_js_to_number(uint32_t);
TS_IMPORT(js_to_bool) int32_t ts_raw_js_to_bool(uint32_t);
TS_IMPORT(js_equals) int32_t ts_raw_js_equals(uint32_t, uint32_t);
TS_IMPORT(js_array_new) uint32_t ts_raw_js_array_new(void);
TS_IMPORT(js_array_length) uint32_t ts_raw_js_array_length(uint32_t);
TS_IMPORT(js_array_get) uint32_t ts_raw_js_array_get(uint32_t, uint32_t);
TS_IMPORT(js_array_push) void ts_raw_js_array_push(uint32_t, uint32_t);
TS_IMPORT(js_retain) void ts_raw_js_retain(uint32_t);
TS_IMPORT(js_release) void ts_raw_js_release(uint32_t);
TS_IMPORT(js_live) uint32_t ts_raw_js_live(void);
TS_IMPORT(js_callback_release) uint32_t ts_raw_js_callback_release(uint32_t);

// An optional number or flag: `has` says whether `value` is there.
typedef struct { bool has; double value; } JsOptionalDouble;
typedef struct { bool has; bool value; } JsOptionalBool;
static inline JsOptionalDouble js_some_double(double x) { return (JsOptionalDouble){ true, x }; }
static inline JsOptionalBool js_some_bool(bool x) { return (JsOptionalBool){ true, x }; }
static inline JsOptionalDouble js_none_double(void) { return (JsOptionalDouble){ false, 0 }; }
static inline JsOptionalBool js_none_bool(void) { return (JsOptionalBool){ false, false }; }

// A handle a function returns is the caller's: release it when done with it.
// Arguments, and a callback's arguments, are lent: retain one to keep it.
// What a callback returns is given to JavaScript: retain a handle to return it and keep it.
#define js_release(x) ts_raw_js_release((x).h)
#define js_retain(x) ts_raw_js_retain((x).h)
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error(void);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
TS_IMPORT(js_has_error) int32_t ts_raw_js_has_error(void);
// Whether two handles are one JavaScript value.
static inline bool ts_same(uint32_t a, uint32_t b) { return a == b || ts_raw_js_equals(a, b) != 0; }
#define js_same(a, b) ts_same((a).h, (b).h)
// Whether the last call threw: its exception waits to be taken.
static inline bool js_failed(void) { return ts_raw_js_has_error() != 0; }
// The exception a call threw, taken, or a null handle. A call that throws
// returns a zero value, and an exception not taken by the time control
// returns to JavaScript is reported there.
static inline JsValue js_take_error(void) { return (JsValue){ ts_raw_js_take_error() }; }
// An exception's message, the caller's to release.
static inline JsString js_error_message(JsValue e) { return (JsString){ ts_raw_js_error_message(e.h) }; }
// A JavaScript iterator: see `X_iterate`.
typedef struct { uint32_t h; } JsIterator;
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_enum_value) uint32_t ts_raw_js_enum_value(int32_t, int32_t);
TS_IMPORT(js_enum_index) int32_t ts_raw_js_enum_index(int32_t, uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
// A scope: every handle a function returns while it is the innermost open
// scope is released when it closes. Retain one to keep it.
TS_IMPORT(js_scope_open) void js_scope_open(void);
TS_IMPORT(js_scope_close) void js_scope_close(void);
// How many JavaScript values wasm holds handles to.
static inline uint32_t js_live_handles(void) { return ts_raw_js_live(); }
// Forgets a callback made with `X_from`, and returns its context. Its handle is released as any other.
#define js_forget_callback(x) ((void*)(uintptr_t)ts_raw_js_callback_release((x).h))
static inline uint32_t JsString_len(JsString s) { return ts_raw_js_string_len(s.h); }
static inline void JsString_read(JsString s, char* buf) { ts_raw_js_string_read(s.h, buf); }
TS_IMPORT(js_string_copy) uint32_t ts_raw_js_string_copy(uint32_t, char*, uint32_t);
TS_IMPORT(js_string_equals) int32_t ts_raw_js_string_equals(uint32_t, const char*, uint32_t);
// Copies at most cap - 1 bytes and a terminating NUL; returns the string's whole length in bytes.
static inline uint32_t JsString_copy(JsString s, char* buf, uint32_t cap) {
    if (cap == 0) return JsString_len(s);
    uint32_t n = ts_raw_js_string_copy(s.h, buf, cap - 1);
    buf[n < cap - 1 ? n : cap - 1] = 0;
    return n;
}
// Compares in JavaScript, without copying.
static inline bool JsString_equals(JsString s, const char* text) { return ts_raw_js_string_equals(s.h, text, ts_strlen(text)) != 0; }
static inline JsValue js_number(double x) { return (JsValue){ ts_raw_js_from_number(x) }; }
static inline JsValue js_bool(bool x) { return (JsValue){ ts_raw_js_from_bool(x) }; }
static inline JsValue js_string(const char* s) { return (JsValue){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline JsString JsString_from(const char* s) { return (JsString){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline double js_to_number(JsValue v) { return ts_raw_js_to_number(v.h); }
static inline bool js_to_bool(JsValue v) { return ts_raw_js_to_bool(v.h) != 0; }
static inline JsString js_to_string(JsValue v) { return (JsString){ v.h }; }
static inline bool js_equals(JsValue a, JsValue b) { return ts_raw_js_equals(a.h, b.h) != 0; }
static inline JsArray JsArray_new(void) { return (JsArray){ ts_raw_js_array_new() }; }
static inline uint32_t JsArray_length(JsArray a) { return ts_raw_js_array_length(a.h); }
static inline JsValue JsArray_get(JsArray a, uint32_t i) { return (JsValue){ ts_raw_js_array_get(a.h, i) }; }
static inline void JsArray_push(JsArray a, JsValue v) { ts_raw_js_array_push(a.h, v.h); }

";

// What an argument left out is: `undefined`, as each type says it.
fn absent(t: &Ty) -> Option<String> {
    let Ty::Nullable(inner, true) = t else { return None };
    Some(match &**inner {
        Ty::Num | Ty::NumEnum(_) => "js_none_double()".into(),
        Ty::Bool => "js_none_bool()".into(),
        Ty::Enum(e) => format!("{e}_absent"),
        Ty::Str => "(const char*)0".into(),
        Ty::NamedUnion(n) if decodable(n) => format!("({n}){{ .tag = -1 }}"),
        _ => format!("({}){{ 0 }}", typed(t, true)),
    })
}

// The C types `_Generic` takes for a parameter of this type, where it can
// tell: an enumerator is an int, so an enum cannot be told from a number.
fn generic_types(t: &Ty) -> Option<Vec<String>> {
    let list = |names: &[&str]| Some(names.iter().map(|n| n.to_string()).collect());
    match t {
        Ty::Str => list(&["const char*", "char*"]),
        Ty::Num => list(&["double", "float", "int", "unsigned int", "long", "unsigned long"]),
        // `true` is an int.
        Ty::Bool => list(&["bool", "int"]),
        Ty::Enum(_) | Ty::NumEnum(_) | Ty::Nullable(..) | Ty::Void => None,
        t => Some(vec![typed(t, true)]),
    }
}

// The overloads `f` is the first of, where the C type of one parameter tells
// them apart: `Shape_label(s, "a")` and `Shape_label(s, 7)`.
fn family<'a>(f: &'a Func, all: &'a [Func]) -> Option<(usize, Vec<&'a Func>)> {
    let key = |f: &Func| match &f.call {
        Call::Method(n) => Some((0, n.clone(), f.fixed.clone())),
        Call::Static(n) => Some((1, n.clone(), f.fixed.clone())),
        _ => None,
    };
    let k = key(f)?;
    let members: Vec<&Func> = all.iter().filter(|o| key(o).as_ref() == Some(&k)).collect();
    if members.len() < 2 || !std::ptr::eq(members[0], f) || members.iter().any(|o| o.params.len() != f.params.len()) {
        return None;
    }
    let differing: Vec<usize> = (0..f.params.len()).filter(|&i| members.iter().any(|o| o.params[i].1 != f.params[i].1)).collect();
    let [at] = differing[..] else { return None };
    let mut seen = std::collections::BTreeSet::new();
    for o in &members {
        if !generic_types(&o.params[at].1)?.into_iter().all(|t| seen.insert(t)) {
            return None;
        }
    }
    Some((at, members))
}

// A function's name as a macro, so a call may: pass any subtype
// (`AS_Element(x)` is applied), leave out trailing optional arguments, and
// for the first of a family of overloads, have `_Generic` choose among them.
fn call_macros(name: &str, owner: Option<&str>, f: &Func, family: Option<(usize, Vec<String>, Vec<&Func>)>, descendants: &std::collections::BTreeMap<String, Vec<String>>) -> String {
    let upcast = |t: &Ty| match t {
        Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Obj(n) if descendants.contains_key(n) => Some(n.clone()),
            _ => None,
        },
        _ => None,
    };
    let mut params = Vec::new();
    let mut args = Vec::new();
    let mut any = false;
    if let Some(o) = owner {
        params.push("self".to_string());
        let upcast_self = descendants.contains_key(o);
        any |= upcast_self;
        args.push(if upcast_self { format!("AS_{o}(self)") } else { "(self)".into() });
    }
    let chosen = family.as_ref().map(|f| f.0);
    for (i, (_, t)) in f.params.iter().enumerate() {
        let a = format!("a{}", i + 1);
        params.push(a.clone());
        match upcast(t).filter(|_| chosen != Some(i)) {
            Some(n) => {
                any = true;
                args.push(format!("AS_{n}({a})"));
            }
            None => args.push(format!("({a})")),
        }
    }
    let callee = match &family {
        Some((at, names, members)) => {
            let choices: Vec<String> = names.iter().zip(members).flat_map(|(n, o)| generic_types(&o.params[*at].1).unwrap_or_default().into_iter().map(move |t| format!("{t}: {n}"))).collect();
            format!("_Generic((a{}), {})", at + 1, choices.join(", "))
        }
        None => name.to_string(),
    };
    let optional = f.params.iter().rev().take_while(|p| absent(&p.1).is_some()).count();
    // A call without arguments cannot be told from one with an empty argument.
    if optional == 0 || params.len() > 15 || params.len() == 1 {
        if !any && family.is_none() {
            return String::new();
        }
        return format!("#define {name}({}) {callee}({})\n", params.join(", "), args.join(", "));
    }
    // By the number of arguments: what is left out is absent.
    let mut out = format!("#define {name}(...) TS_BY_COUNT({name}, __VA_ARGS__)(__VA_ARGS__)\n");
    let own = usize::from(owner.is_some());
    for given in (params.len() - optional).max(1)..=params.len() {
        let filled: Vec<String> = args[..given].iter().cloned().chain(f.params[given - own..].iter().filter_map(|p| absent(&p.1))).collect();
        out += &format!("#define {name}__{given}({}) {callee}({})\n", params[..given].join(", "), filled.join(", "));
    }
    out
}

// Direct and indirect bases, nearest first.
fn all_ancestors(m: &Model, class: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut todo = vec![class.to_string()];
    while let Some(c) = todo.pop() {
        for a in m.classes.iter().filter(|k| k.name == c).flat_map(|k| &k.ancestors) {
            if !out.contains(a) {
                out.push(a.clone());
                todo.insert(0, a.clone());
            }
        }
    }
    out
}

// The bindings as files: `ts_runtime.h`; `ts_types.h` with every handle type,
// enum and callback (so headers never need each other); one header per
// declaration in a directory per module, `<module>/<Name>.h`, with the module's
// functions in `<module>/functions.h`; an umbrella `<module>.h` per module, and
// `bindings.h` for everything.
// What C cannot express of a model: inline unions, generics, keyed signatures.
fn check(m: &Model) -> super::Problems {
    let mut problems = super::Problems::default();
    super::wasm::each_type(m, &mut |at, t| {
        let what = match t {
            Ty::Union(_) => "a union: set unions = \"name\" or \"erase\"",
            Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => "a keyed signature: set keyed_params = \"expand\"",
            Ty::Param(_) | Ty::Generic(..) => "generics: set generics = \"erase\"",
            _ => return,
        };
        problems.add(format!("{T} generator cannot express {what}"), at.to_string());
    });
    problems
}

pub fn files(m: &Model) -> Result<Vec<(String, String)>, Vec<String>> {
    let problems = check(m);
    if !problems.is_empty() {
        return Err(problems.into_messages());
    }
    DECODABLE.with(|d| *d.borrow_mut() = m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| u.name.clone()).collect());
    let mut types = String::from("// Generated.\n#pragma once\n#include \"ts_runtime.h\"\n\n");
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        // `E_absent`, -1, is an optional enum that is not there.
        let absent = format!("{name}_absent");
        seen.insert(absent.clone());
        let members: Vec<String> = values
            .iter()
            .map(|v| {
                let base = format!("{name}_{}", super::wasm::ident(v));
                let mut id = base.clone();
                let mut n = 2;
                while !seen.insert(id.clone()) {
                    id = format!("{base}_{n}");
                    n += 1;
                }
                id
            })
            .collect();
        types += &format!("typedef enum {{ {absent} = -1, {} }} {name};\n", members.join(", "));
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{name}_{member} = {value}")).collect();
        types += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
    }
    // Every object is an opaque handle; the struct keeps the types apart.
    let handles = m.opaque.iter().chain(m.classes.iter().map(|c| &c.name)).chain(m.callbacks.iter().map(|c| &c.name));
    for name in handles {
        types += &format!("typedef struct {{ uint32_t h; }} {name};\n");
    }
    // A callback is made from a function and a context pointer passed back to it.
    // C11: one upcast per type with descendants. `AS_Element(div)` is `div` as
    // an Element, and a compile error for anything that is not one.
    let mut descendants: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    for c in &m.classes {
        for a in all_ancestors(m, &c.name) {
            descendants.entry(a).or_default().push(c.name.clone());
        }
    }
    for (base, subs) in &descendants {
        let arms: Vec<String> = subs.iter().map(|s| format!("    {s}: ({base}){{ (x).h }}")).collect();
        types += &format!("#define AS_{base}(x) _Generic((x), \\\n    {base}: (x), \\\n{})\n", arms.join(", \\\n"));
    }
    // A promise: `Promise_then(p, fn, ctx)` calls fn(ctx, ok, value) once it
    // settles, ok with its value or not with its reason; the value is lent.
    let iterates_async = m.classes.iter().any(|c| c.iterates_async.is_some());
    if iterates_async && !m.opaque.iter().any(|n| n == "Promise") {
        types += "typedef struct { uint32_t h; } Promise;\n";
    }
    if iterates_async {
        types += "static inline Promise JsIterator_next_async(JsIterator it) { return (Promise){ ts_raw_js_async_iterator_next(it.h) }; }\n";
    }
    if m.opaque.iter().any(|n| n == "Promise") || iterates_async {
        types += "typedef void (*Promise_fn)(void* ctx, bool ok, JsValue value);\n\
                  TS_IMPORT(js_promise_then) void ts_raw_js_promise_then(uint32_t, Promise_fn, void*);\n\
                  static inline void Promise_then(Promise p, Promise_fn fn, void* ctx) { ts_raw_js_promise_then(p.h, fn, ctx); }\n";
    }
    // Each typed array, after its element type and before what uses it.
    let mut elements = Vec::new();
    for f in m.all_funcs() {
        f.params.iter().for_each(|p| arrays(&p.1, &mut elements));
        arrays(&f.ret, &mut elements);
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|p| arrays(&p.1, &mut elements));
        arrays(&c.ret, &mut elements);
    }
    for u in &m.unions {
        u.parts.iter().for_each(|p| arrays(p, &mut elements));
    }
    for c in m.classes.iter().filter(|c| c.elements.is_some()) {
        arrays(&Ty::ArrayOf(Box::new(c.elements.clone().unwrap())), &mut elements);
    }
    // The structs first, then unions, which may hold arrays, then the
    // functions, which may read unions out of arrays.
    let containers: Vec<String> = elements
        .iter()
        .map(|t| match t {
            Ty::ArrayOf(e) => array_type(e),
            Ty::Tuple(es) => tuple_type(es),
            _ => unreachable!(),
        })
        .collect();
    for c in &containers {
        types += c.lines().next().unwrap();
        types += "\n";
    }
    let unions: Vec<(String, String)> = m.unions.iter().filter(|u| u.decodable() && u.type_params.is_empty()).map(|u| union_type(u, m)).collect();
    for (decl, _) in &unions {
        types += decl;
    }
    for (_, functions) in &unions {
        types += functions;
    }
    for c in &containers {
        types += &c.lines().skip(1).map(|l| format!("{l}\n")).collect::<String>();
    }
    for c in &m.callbacks {
        let mut params = vec!["void* ctx".to_string()];
        // A numeric enum arrives as the number it is.
        // An optional number arrives as a lent handle, an optional flag as -1, 0 or 1.
        let plain = |t: &Ty| match t {
            Ty::NumEnum(_) => "double".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsValue".to_string(),
            Ty::Nullable(inner, _) if **inner == Ty::Bool => "int32_t".to_string(),
            // A union arrives as a lent handle.
            Ty::NamedUnion(_) | Ty::Nullable(..) if matches!(t, Ty::NamedUnion(_)) || matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_))) => "JsValue".to_string(),
            t => typed(t, false),
        };
        params.extend(c.params.iter().map(|(n, t)| format!("{} {}", plain(t), ident(n))));
        types += &format!("\ntypedef {} (*{}_fn)({});\n", plain(&c.ret), c.name, params.join(", "));
        types += &format!("TS_IMPORT({0}_from) uint32_t ts_raw_{0}_from({0}_fn, void*);\n", c.name);
        types += &format!("static inline {0} {0}_from({0}_fn fn, void* ctx) {{ return ({0}){{ ts_raw_{0}_from(fn, ctx) }}; }}\n", c.name);
    }

    let mut out = vec![("ts_runtime.h".to_string(), RUNTIME.to_string()), ("ts_types.h".to_string(), types)];
    let mut modules: std::collections::BTreeMap<String, Vec<String>> = Default::default();
    // `areas/css/X.h` includes `../../ts_types.h`.
    let header = |module: &str, body: String| format!("// Generated.\n#pragma once\n#include \"{}ts_types.h\"\n\n{body}", "../".repeat(module.split('/').count()));
    let mut taken = std::collections::HashMap::new();
    for c in &m.classes {
        let mut body = String::new();
        for (name, value) in &c.consts {
            body += &format!("#define {}_{name} {value}\n", c.name);
        }
        // A subclass of Array, as the array it is: the same handle.
        if let Some(elem) = &c.elements {
            let array = typed(&Ty::ArrayOf(Box::new(elem.clone())), false);
            body += &format!("static inline {array} {0}_as_array({0} self) {{ return ({array}){{ self.h }}; }}\n", c.name);
        }
        // Asynchronous iteration: `X_iterate_async(x)`, then `JsIterator_next_async(it)`, a
        // promise of the next value, or of a null handle at the end.
        if c.iterates_async.is_some() {
            let n = &c.name;
            body += &format!("static inline JsIterator {n}_iterate_async({n} self) {{ return (JsIterator){{ ts_raw_js_async_iterate(self.h) }}; }}\n");
        }
        // Iteration: `X_iterate(x)`, then `X_iterate_next(it, &value)` until false; each value is the caller's.
        if let Some(elem) = &c.iterates {
            let (ty, value) = match elem {
                Ty::Num => ("double".to_string(), "ts_raw_js_to_number(v); ts_raw_js_release(v)".to_string()),
                Ty::Obj(_) | Ty::Str | Ty::Array | Ty::ArrayOf(_) => (typed(elem, false), format!("({}){{ v }}", typed(elem, false))),
                _ => ("JsValue".to_string(), "(JsValue){ v }".to_string()),
            };
            let n = &c.name;
            body += &format!("static inline JsIterator {n}_iterate({n} self) {{ return (JsIterator){{ ts_raw_js_iterate(self.h) }}; }}\n");
            body += &format!("static inline bool {n}_iterate_next(JsIterator it, {ty}* out) {{ uint32_t v = ts_raw_js_iterator_next(it.h); if (v == 0xFFFFFFFFu) return false; *out = {value}; return true; }}\n");
            // Any subtype iterates: `HTMLCollectionBase_iterate(children)`.
            if descendants.contains_key(n) {
                body += &format!("#define {n}_iterate(self) {n}_iterate(AS_{n}(self))\n");
            }
        }
        // A cast to every ancestor: an inherited member is called on the type that declares it.
        for a in all_ancestors(m, &c.name) {
            body += &format!("static inline {a} {}_as_{a}({} self) {{ return ({a}){{ self.h }}; }}\n", c.name, c.name);
        }
        for f in &c.funcs {
            func(&mut body, Some(&c.name), f);
        }
        // After every function: a macro may choose a later one.
        for f in &c.funcs {
            let overloads = family(f, &c.funcs).map(|(at, members)| (at, members.iter().map(|o| format!("{}_{}", c.name, o.name)).collect(), members));
            body += &call_macros(&format!("{}_{}", c.name, f.name), Class::has_self(f).then_some(c.name.as_str()), f, overloads, &descendants);
        }
        let path = format!("{}.h", super::wasm::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path, header(&c.module, body)));
    }
    let mut functions: std::collections::BTreeMap<String, String> = Default::default();
    for f in &m.funcs {
        func(functions.entry(f.module.clone()).or_default(), None, f);
    }
    for f in &m.funcs {
        // Overloads in one module: in one header.
        let overloads = family(f, &m.funcs).filter(|(_, members)| members.iter().all(|o| o.module == f.module)).map(|(at, members)| (at, members.iter().map(|o| o.name.clone()).collect(), members));
        *functions.entry(f.module.clone()).or_default() += &call_macros(&f.name, None, f, overloads, &descendants);
    }
    for (module, body) in functions {
        let path = format!("{}.h", super::wasm::file_name(&format!("{module}/functions"), &mut taken));
        out.push((path.clone(), header(&module, body)));
        modules.entry(module).or_default().push(path);
    }
    let mut all = String::from("// Generated.\n#pragma once\n");
    for (module, headers) in &modules {
        // `areas/css.h` includes `css/X.h`, next to it.
        let parent = module.rsplit_once('/').map_or("", |p| p.0);
        let includes: String = headers.iter().map(|h| format!("#include \"{}\"\n", h.strip_prefix(&format!("{parent}/")).unwrap_or(h))).collect();
        out.push((format!("{module}.h"), format!("// Generated.\n#pragma once\n{includes}")));
        all += &format!("#include \"{module}.h\"\n");
    }
    out.push(("bindings.h".into(), all));
    Ok(out)
}
