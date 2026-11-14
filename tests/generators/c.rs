// XML -> C header for a wasm32 module. No inheritance, generics, overloads,
// optional parameters or unions: the config must have lowered them all.

use super::wasm::{Class, Func, Model, Ty};

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
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // An optional number or flag says whether it is there; an optional enum is `E_absent` when it is not.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsOptionalDouble".into(),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => "JsOptionalBool".into(),
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param),
        Ty::Union(_) | Ty::NamedUnion(_) => panic!("{T} generator cannot express a union: the target config must erase unions"),
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

// The element types of the arrays a type names, innermost first.
fn arrays(t: &Ty, out: &mut Vec<Ty>) {
    match t {
        Ty::ArrayOf(e) => {
            arrays(e, out);
            if !out.contains(e) {
                out.push((**e).clone());
            }
        }
        Ty::Nullable(inner, _) => arrays(inner, out),
        _ => {}
    }
}

// A typed array: `JsArray_double`, with `_new`, `_length`, `_get` and `_push`.
// What `_get` returns is the caller's to release; a number or flag is read and released.
fn array_type(e: &Ty) -> String {
    let (name, elem) = (format!("JsArray_{}", element(e)), typed(e, false));
    let (get, push_param, push) = match e {
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
    for (name, t) in &f.params {
        let name = ident(name);
        params.push(format!("{} {name}", typed(t, true)));
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
    let call = format!("ts_raw_{import}({})", args.join(", "));
    let body = match &f.ret {
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
    *out += &format!("static inline {} {name}({}) {{ {body} }}\n", typed(&f.ret, false), void(&params));
}

const RUNTIME: &str = "// Generated.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#define TS_IMPORT(name) __attribute__((import_module(\"env\"), import_name(#name)))

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
#define JS_NONE { 0 }

// A handle a function returns is the caller's: release it when done with it.
// Arguments, and a callback's arguments, are lent: retain one to keep it.
// What a callback returns is given to JavaScript: retain a handle to return it and keep it.
#define js_release(x) ts_raw_js_release((x).h)
#define js_retain(x) ts_raw_js_retain((x).h)
TS_IMPORT(js_take_error) uint32_t ts_raw_js_take_error(void);
TS_IMPORT(js_error_message) uint32_t ts_raw_js_error_message(uint32_t);
// The exception a call threw, taken, or a null handle. A call that throws returns a zero value.
static inline JsValue js_take_error(void) { return (JsValue){ ts_raw_js_take_error() }; }
// An exception's message, the caller's to release.
static inline JsString js_error_message(JsValue e) { return (JsString){ ts_raw_js_error_message(e.h) }; }
// A JavaScript iterator: see `X_iterate`.
typedef struct { uint32_t h; } JsIterator;
TS_IMPORT(js_iterate) uint32_t ts_raw_js_iterate(uint32_t);
TS_IMPORT(js_iterator_next) uint32_t ts_raw_js_iterator_next(uint32_t);
TS_IMPORT(js_async_iterate) uint32_t ts_raw_js_async_iterate(uint32_t);
TS_IMPORT(js_async_iterator_next) uint32_t ts_raw_js_async_iterator_next(uint32_t);
// How many JavaScript values wasm holds handles to.
static inline uint32_t js_live_handles(void) { return ts_raw_js_live(); }
// Forgets a callback made with `X_from`, and returns its context. Its handle is released as any other.
#define js_forget_callback(x) ((void*)(uintptr_t)ts_raw_js_callback_release((x).h))
static inline uint32_t JsString_len(JsString s) { return ts_raw_js_string_len(s.h); }
static inline void JsString_read(JsString s, char* buf) { ts_raw_js_string_read(s.h, buf); }
static inline JsValue js_number(double x) { return (JsValue){ ts_raw_js_from_number(x) }; }
static inline JsValue js_bool(bool x) { return (JsValue){ ts_raw_js_from_bool(x) }; }
static inline JsValue js_string(const char* s) { return (JsValue){ ts_raw_js_from_string(s, ts_strlen(s)) }; }
static inline double js_to_number(JsValue v) { return ts_raw_js_to_number(v.h); }
static inline bool js_to_bool(JsValue v) { return ts_raw_js_to_bool(v.h) != 0; }
static inline JsString js_to_string(JsValue v) { return (JsString){ v.h }; }
static inline bool js_equals(JsValue a, JsValue b) { return ts_raw_js_equals(a.h, b.h) != 0; }
static inline JsArray JsArray_new(void) { return (JsArray){ ts_raw_js_array_new() }; }
static inline uint32_t JsArray_length(JsArray a) { return ts_raw_js_array_length(a.h); }
static inline JsValue JsArray_get(JsArray a, uint32_t i) { return (JsValue){ ts_raw_js_array_get(a.h, i) }; }
static inline void JsArray_push(JsArray a, JsValue v) { ts_raw_js_array_push(a.h, v.h); }

";

// The function's own name as a macro that upcasts `self` and any argument
// whose type has subtypes, so those can be passed as they are:
// `Element_setAttribute(div, ...)`, `Node_appendChild(body, div)`. Inside the
// macro, the name is the function. Empty when nothing needs upcasting.
fn upcasting_macro(name: &str, owner: Option<&str>, f: &Func, descendants: &std::collections::BTreeMap<String, Vec<String>>) -> String {
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
    for (i, (_, t)) in f.params.iter().enumerate() {
        let a = format!("a{}", i + 1);
        params.push(a.clone());
        match upcast(t) {
            Some(n) => {
                any = true;
                args.push(format!("AS_{n}({a})"));
            }
            None => args.push(format!("({a})")),
        }
    }
    if !any {
        return String::new();
    }
    format!("#define {name}({}) {name}({})\n", params.join(", "), args.join(", "))
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
pub fn files(m: &Model) -> Vec<(String, String)> {
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
    for e in &elements {
        types += &array_type(e);
    }
    for c in &m.callbacks {
        let mut params = vec!["void* ctx".to_string()];
        // A numeric enum arrives as the number it is.
        // An optional number arrives as a lent handle, an optional flag as -1, 0 or 1.
        let plain = |t: &Ty| match t {
            Ty::NumEnum(_) => "double".to_string(),
            Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsValue".to_string(),
            Ty::Nullable(inner, _) if **inner == Ty::Bool => "int32_t".to_string(),
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
            body += &upcasting_macro(&format!("{}_{}", c.name, f.name), Class::has_self(f).then_some(c.name.as_str()), f, &descendants);
        }
        let path = format!("{}.h", super::wasm::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path, header(&c.module, body)));
    }
    let mut functions: std::collections::BTreeMap<String, String> = Default::default();
    for f in &m.funcs {
        let out = functions.entry(f.module.clone()).or_default();
        func(out, None, f);
        *out += &upcasting_macro(&f.name, None, f, &descendants);
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
    out
}
