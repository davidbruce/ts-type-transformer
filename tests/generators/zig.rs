// XML -> Zig module for a wasm32 module. As C, but unions of simple types
// become tagged unions, and nullable objects become optionals.

use super::wasm::{Class, Func, Model, Ty};

// Quoted identifiers accept any name, keywords included.
fn ident(name: &str) -> String {
    format!("@\"{name}\"")
}

// Parameters get a suffix: Zig forbids shadowing a declaration in scope.
fn param(name: &str) -> String {
    format!("{name}_")
}

fn raw(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num | Ty::NumEnum(_) => "f64",
        Ty::Bool | Ty::Enum(_) => "i32",
        _ => "u32",
    }
}

fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) => n.clone(),
        _ => unreachable!(),
    }
}

fn typed(t: &Ty, is_param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if is_param => "[]const u8".into(),
        Ty::Str => "JsString".into(),
        Ty::Array => "JsArray".into(),
        Ty::Obj(n) | Ty::Enum(n) | Ty::NumEnum(n) => n.clone(),
        Ty::Nullable(inner, _) => format!("?{}", typed(inner, is_param)),
        Ty::Union(parts) if !parts.is_empty() && is_param => {
            let fields: Vec<String> = parts.iter().map(|p| format!("{}: {}", union_field(p), typed(p, true))).collect();
            format!("union(enum) {{ {} }}", fields.join(", "))
        }
        Ty::Any | Ty::Union(_) => "JsValue".into(),
    }
}

// Types a C-convention callback can take: no slices, no optionals.
fn callback_typed(t: &Ty) -> String {
    match t {
        Ty::Nullable(inner, _) => callback_typed(inner),
        Ty::NumEnum(_) => "f64".into(),
        t => typed(t, false),
    }
}

// Expression turning a typed value into what crosses the boundary.
fn lower_arg(name: &str, t: &Ty) -> String {
    match t {
        Ty::Num => name.into(),
        Ty::Bool => format!("@intFromBool({name})"),
        Ty::Enum(_) => format!("@intFromEnum({name})"),
        Ty::NumEnum(_) => format!("@floatFromInt(@intFromEnum({name}))"),
        Ty::Nullable(..) => format!("if ({name}) |v| v.h else 0"),
        // Each alternative becomes a JavaScript value.
        Ty::Union(parts) if !parts.is_empty() => {
            let arms: Vec<String> = parts
                .iter()
                .map(|p| {
                    let field = union_field(p);
                    match p {
                        Ty::Num => format!(".{field} => |v| JsValue.number(v).h"),
                        Ty::Str => format!(".{field} => |v| JsValue.string(v).h"),
                        Ty::Bool => format!(".{field} => |v| JsValue.boolean(v).h"),
                        _ => format!(".{field} => |v| v.h"),
                    }
                })
                .collect();
            format!("switch ({name}) {{ {} }}", arms.join(", "))
        }
        _ => format!("{name}.h"),
    }
}

fn func(out: &mut String, externs: &mut String, owner: Option<&str>, f: &Func, indent: &str) {
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        raw_params.push("u32".to_string());
        params.push(format!("self: {}", owner.unwrap()));
        args.push("self.h".to_string());
    }
    for (name, t) in &f.params {
        let id = param(name);
        params.push(format!("{id}: {}", typed(t, true)));
        if *t == Ty::Str {
            raw_params.extend(["[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("{id}.ptr"), format!("{id}.len")]);
        } else {
            raw_params.push(raw(t).to_string());
            args.push(lower_arg(&id, t));
        }
    }
    let call = format!("raw.{}({})", f.import, args.join(", "));
    let body = match &f.ret {
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(_) => format!("return @enumFromInt({call});"),
        Ty::NumEnum(_) => format!("return @enumFromInt(@as(i32, @intFromFloat({call})));"),
        Ty::Nullable(..) => format!("const h = {call}; return if (h == 0) null else .{{ .h = h }};"),
        _ => format!("return .{{ .h = {call} }};"),
    };
    *externs += &format!("    extern \"env\" fn {}({}) {};\n", f.import, raw_params.join(", "), raw(&f.ret));
    *out += &format!("{indent}pub fn {}({}) {} {{ {body} }}\n", ident(&f.name), params.join(", "), typed(&f.ret, false));
}

const RUNTIME: &str = "// Generated.

// Handles to JavaScript values. Zero is null.
pub const JsValue = extern struct {
    h: u32,
    pub fn number(x: f64) JsValue { return .{ .h = raw.js_from_number(x) }; }
    pub fn boolean(x: bool) JsValue { return .{ .h = raw.js_from_bool(@intFromBool(x)) }; }
    pub fn string(s: []const u8) JsValue { return .{ .h = raw.js_from_string(s.ptr, s.len) }; }
    pub fn toNumber(self: JsValue) f64 { return raw.js_to_number(self.h); }
    pub fn toBool(self: JsValue) bool { return raw.js_to_bool(self.h) != 0; }
    pub fn toString(self: JsValue) JsString { return .{ .h = self.h }; }
    pub fn eql(self: JsValue, other: JsValue) bool { return raw.js_equals(self.h, other.h) != 0; }
    pub fn isNull(self: JsValue) bool { return self.h == 0; }
};
pub const JsString = extern struct {
    h: u32,
    pub fn len(self: JsString) usize { return raw.js_string_len(self.h); }
    pub fn read(self: JsString, buf: []u8) []u8 { const n = self.len(); raw.js_string_read(self.h, buf.ptr); return buf[0..n]; }
};
pub const JsArray = extern struct {
    h: u32,
    pub fn new() JsArray { return .{ .h = raw.js_array_new() }; }
    pub fn len(self: JsArray) usize { return raw.js_array_length(self.h); }
    pub fn get(self: JsArray, i: usize) JsValue { return .{ .h = raw.js_array_get(self.h, i) }; }
    pub fn push(self: JsArray, v: JsValue) void { raw.js_array_push(self.h, v.h); }
};

";

const RUNTIME_EXTERNS: &str = "    extern \"env\" fn js_string_len(h: u32) usize;
    extern \"env\" fn js_string_read(h: u32, ptr: [*]u8) void;
    extern \"env\" fn js_from_number(x: f64) u32;
    extern \"env\" fn js_from_bool(x: i32) u32;
    extern \"env\" fn js_from_string(ptr: [*]const u8, len: usize) u32;
    extern \"env\" fn js_to_number(h: u32) f64;
    extern \"env\" fn js_to_bool(h: u32) i32;
    extern \"env\" fn js_equals(a: u32, b: u32) i32;
    extern \"env\" fn js_array_new() u32;
    extern \"env\" fn js_array_length(h: u32) usize;
    extern \"env\" fn js_array_get(h: u32, i: usize) u32;
    extern \"env\" fn js_array_push(h: u32, v: u32) void;
";

pub fn module(m: &Model) -> String {
    let mut out = String::from(RUNTIME);
    let mut externs = String::from(RUNTIME_EXTERNS);
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| if v.is_empty() { "empty" } else { v }).filter(|v| seen.insert(v.to_string())).map(ident).collect();
        out += &format!("pub const {name} = enum(i32) {{ {} }};\n", members.join(", "));
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        out += &format!("pub const {name} = enum(i32) {{ {} }};\n", members.join(", "));
    }
    for name in &m.opaque {
        out += &format!("pub const {name} = extern struct {{ h: u32 }};\n");
    }
    // A callback is made from a function and a context pointer passed back to it.
    for c in &m.callbacks {
        let mut params = vec!["?*anyopaque".to_string()];
        params.extend(c.params.iter().map(|(_, t)| callback_typed(t)));
        out += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        out += &format!("    pub const Fn = *const fn ({}) callconv(.c) {};\n", params.join(", "), callback_typed(&c.ret));
        out += &format!("    pub fn from(f: Fn, ctx: ?*anyopaque) {} {{ return .{{ .h = raw.{}_from(f, ctx) }}; }}\n}};\n", c.name, c.name);
        externs += &format!("    extern \"env\" fn {0}_from(f: {0}.Fn, ctx: ?*anyopaque) u32;\n", c.name);
    }
    // Every object is an opaque handle; methods live in the struct's namespace.
    for c in &m.classes {
        out += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        for (name, value) in &c.consts {
            out += &format!("    pub const {} = {value};\n", ident(name));
        }
        for a in &c.ancestors {
            out += &format!("    pub fn {}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", ident(&format!("as{a}")), c.name);
        }
        for f in &c.funcs {
            func(&mut out, &mut externs, Some(&c.name), f, "    ");
        }
        out += "};\n";
    }
    out += "\n";
    for f in &m.funcs {
        func(&mut out, &mut externs, None, f, "");
    }
    out + "\nconst raw = struct {\n" + &externs + "};\n"
}
