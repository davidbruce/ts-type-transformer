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
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        // A null handle is zero.
        // NULL, or a null handle, is the absent value.
        Ty::Nullable(inner, _) => typed(inner, param),
        Ty::Union(_) | Ty::NamedUnion(_) => panic!("{T} generator cannot express a union: the target config must erase unions"),
        Ty::Key(_) | Ty::Mapped(_) | Ty::Listener(_) => panic!("{T} generator cannot express keyed generics: the target config must expand them"),
    }
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
        types += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
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
    for c in &m.callbacks {
        let mut params = vec!["void* ctx".to_string()];
        // A numeric enum arrives as the number it is.
        let plain = |t: &Ty| if matches!(t, Ty::NumEnum(_)) { "double".to_string() } else { typed(t, false) };
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
        // A cast to every ancestor: an inherited member is called on the type that declares it.
        for a in all_ancestors(m, &c.name) {
            body += &format!("static inline {a} {}_as_{a}({} self) {{ return ({a}){{ self.h }}; }}\n", c.name, c.name);
        }
        for f in &c.funcs {
            func(&mut body, Some(&c.name), f);
        }
        let path = format!("{}.h", super::wasm::file_name(&format!("{}/{}", c.module, c.name), &mut taken));
        modules.entry(c.module.clone()).or_default().push(path.clone());
        out.push((path, header(&c.module, body)));
    }
    let mut functions: std::collections::BTreeMap<String, String> = Default::default();
    for f in &m.funcs {
        func(functions.entry(f.module.clone()).or_default(), None, f);
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
