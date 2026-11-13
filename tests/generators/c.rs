// XML -> C header for a wasm32 module. No inheritance, generics, overloads,
// optional parameters or unions: the config must have lowered them all.

use super::wasm::{Call, Func, Model, Ty};

const T: &str = "C";

fn raw(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num => "double",
        Ty::Bool | Ty::Enum(_) => "int32_t",
        Ty::Str | Ty::Any | Ty::Obj(_) | Ty::Nullable(..) => "uint32_t",
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must erase unions"),
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
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        // A null object is the zero handle.
        Ty::Nullable(inner, _) => match &**inner {
            Ty::Obj(n) => n.clone(),
            _ => panic!("{T} generator only supports nullable objects"),
        },
        Ty::Union(_) => panic!("{T} generator cannot express a union: the target config must erase unions"),
    }
}

fn func(out: &mut String, owner: Option<&str>, f: &Func) {
    let has_self = matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_));
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if has_self {
        raw_params.push("uint32_t".to_string());
        params.push(format!("{} self", owner.unwrap()));
        args.push("self.h".to_string());
    }
    for (name, t) in &f.params {
        params.push(format!("{} {name}", typed(t, true)));
        match t {
            Ty::Str => {
                raw_params.extend(["const char*".to_string(), "uint32_t".to_string()]);
                args.extend([name.clone(), format!("ts_strlen({name})")]);
            }
            Ty::Num | Ty::Bool | Ty::Enum(_) => {
                raw_params.push(raw(t).into());
                args.push(name.clone());
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
    let call = format!("ts_raw_{}({})", f.import, args.join(", "));
    let body = match &f.ret {
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(e) => format!("return ({e}){call};"),
        t => format!("return ({}){{ {call} }};", typed(t, false)),
    };
    *out += &format!("TS_IMPORT({}) {} ts_raw_{}({});\n", f.import, raw(&f.ret), f.import, void(&raw_params));
    *out += &format!("static inline {} {name}({}) {{ {body} }}\n", typed(&f.ret, false), void(&params));
}

pub fn header(m: &Model) -> String {
    let mut out = String::from(
        "// Generated.
#pragma once
#include <stdbool.h>
#include <stdint.h>
#define TS_IMPORT(name) __attribute__((import_module(\"env\"), import_name(#name)))

typedef struct { uint32_t h; } JsValue;
typedef struct { uint32_t h; } JsString;
static inline uint32_t ts_strlen(const char* s) { uint32_t n = 0; while (s[n]) n++; return n; }
TS_IMPORT(js_string_len) uint32_t ts_raw_js_string_len(uint32_t);
TS_IMPORT(js_string_read) void ts_raw_js_string_read(uint32_t, char*);
TS_IMPORT(js_from_number) uint32_t ts_raw_js_from_number(double);
TS_IMPORT(js_from_string) uint32_t ts_raw_js_from_string(const char*, uint32_t);
static inline uint32_t JsString_len(JsString s) { return ts_raw_js_string_len(s.h); }
static inline void JsString_read(JsString s, char* buf) { ts_raw_js_string_read(s.h, buf); }
static inline JsValue js_number(double x) { return (JsValue){ ts_raw_js_from_number(x) }; }
static inline JsValue js_string(const char* s) { return (JsValue){ ts_raw_js_from_string(s, ts_strlen(s)) }; }

",
    );
    for (name, values) in &m.enums {
        let members: Vec<String> = values.iter().map(|v| format!("{name}_{v}")).collect();
        out += &format!("typedef enum {{ {} }} {name};\n", members.join(", "));
    }
    // Every object is an opaque handle; the struct keeps the types apart.
    for c in &m.classes {
        out += &format!("typedef struct {{ uint32_t h; }} {};\n", c.name);
    }
    for c in &m.classes {
        out += &format!("\n// {}\n", c.name);
        for (name, value) in &c.consts {
            out += &format!("#define {}_{name} {value}\n", c.name);
        }
        for a in &c.ancestors {
            out += &format!("static inline {a} {}_as_{a}({} self) {{ return ({a}){{ self.h }}; }}\n", c.name, c.name);
        }
        for f in &c.funcs {
            func(&mut out, Some(&c.name), f);
        }
    }
    out += "\n";
    for f in &m.funcs {
        func(&mut out, None, f);
    }
    out
}
