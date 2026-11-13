// XML -> Zig module for a wasm32 module. As C, but unions of simple types
// become tagged unions, and nullable objects become optionals.

use super::wasm::{Class, Func, Model, NamedUnion, Ty};

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
        Ty::Bool | Ty::Enum(_) | Ty::Key(_) => "i32",
        _ => "u32",
    }
}

fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) => n.clone(),
        Ty::Array => "array".into(),
        _ => unreachable!(),
    }
}

// A named union: a Zig tagged union, the same for arguments and results.
// Strings in it are handles: `JsString.from("text")`.
fn union_decl(u: &NamedUnion) -> String {
    let fields: Vec<String> = u
        .parts
        .iter()
        .map(|p| {
            let t = match p {
                Ty::Str => "JsString".to_string(),
                p => typed(p, false),
            };
            format!("{}: {t}", ident(&union_field(p)))
        })
        .collect();
    format!("pub const {} = union(enum) {{ {} }};\n", u.name, fields.join(", "))
}

// Turns a handle coming back into the union's member: `raw.U_tag` says which.
fn decode_union(u: &NamedUnion, h: &str) -> String {
    let arms: Vec<String> = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let value = match p {
                Ty::Num => format!("(JsValue{{ .h = {h} }}).toNumber()"),
                Ty::Bool => format!("(JsValue{{ .h = {h} }}).toBool()"),
                _ => format!(".{{ .h = {h} }}"),
            };
            format!("{i} => .{{ .{} = {value} }}", ident(&union_field(p)))
        })
        .collect();
    format!("switch (raw.{}_tag({h})) {{ {}, else => unreachable }}", u.name, arms.join(", "))
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
        Ty::NamedUnion(n) => n.clone(),
        // `KEY` is replaced by the name of the signature's key parameter.
        Ty::Key(m) => format!("{m}.Key"),
        Ty::Mapped(m) => format!("{m}.Type(KEY)"),
        Ty::Listener(ev) => format!("JsListener({})", typed(ev, false)),
        Ty::Any | Ty::Union(_) => "JsValue".into(),
    }
}

// Types a C-convention callback can take: no slices, no optionals, no unions.
fn callback_typed(t: &Ty) -> String {
    match t {
        Ty::Nullable(inner, _) => callback_typed(inner),
        Ty::NamedUnion(_) => "JsValue".into(),
        Ty::NumEnum(_) => "f64".into(),
        t => typed(t, false),
    }
}

// Expression turning a typed value into what crosses the boundary.
fn lower_arg(name: &str, t: &Ty) -> String {
    match t {
        Ty::Num => name.into(),
        Ty::Bool => format!("@intFromBool({name})"),
        Ty::Enum(_) | Ty::Key(_) => format!("@intFromEnum({name})"),
        Ty::NumEnum(_) => format!("@floatFromInt(@intFromEnum({name}))"),
        Ty::Nullable(..) => format!("if ({name}) |v| v.h else 0"),
        // Each member becomes a JavaScript value.
        Ty::NamedUnion(_) => format!("switch ({name}) {{ inline else => |v| JsValue.of(v).h }}"),
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

fn func(out: &mut String, externs: &mut String, owner: Option<&str>, f: &Func, indent: &str, unions: &[NamedUnion]) {
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if Class::has_self(f) {
        raw_params.push("u32".to_string());
        params.push(format!("self: {}", owner.unwrap()));
        args.push("self.h".to_string());
    }
    // A key is known at compile time: the types it selects are computed from it.
    let key = f.params.iter().find(|p| matches!(p.1, Ty::Key(_))).map(|p| param(&p.0)).unwrap_or_default();
    for (name, t) in &f.params {
        let id = param(name);
        let comptime = if matches!(t, Ty::Key(_)) { "comptime " } else { "" };
        params.push(format!("{comptime}{id}: {}", typed(t, true).replace("KEY", &key)));
        if *t == Ty::Str {
            raw_params.extend(["[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("{id}.ptr"), format!("{id}.len")]);
        } else {
            raw_params.push(raw(t).to_string());
            args.push(lower_arg(&id, t));
        }
    }
    let call = format!("raw.{}({})", f.import, args.join(", "));
    // A union result that cannot be told apart is an untyped value.
    let union = match &f.ret {
        Ty::NamedUnion(n) => unions.iter().find(|u| u.name == *n),
        _ => None,
    };
    let ret_type = match union {
        Some(u) if !u.decodable() => "JsValue".to_string(),
        _ => typed(&f.ret, false).replace("KEY", &key),
    };
    let body = match &f.ret {
        Ty::NamedUnion(_) if union.is_some_and(|u| u.decodable()) => format!("const h = {call}; return {};", decode_union(union.unwrap(), "h")),
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(_) => format!("return @enumFromInt({call});"),
        Ty::NumEnum(_) => format!("return @enumFromInt(@as(i32, @intFromFloat({call})));"),
        Ty::Nullable(..) => format!("const h = {call}; return if (h == 0) null else .{{ .h = h }};"),
        _ => format!("return .{{ .h = {call} }};"),
    };
    *externs += &format!("    extern \"env\" fn {}({}) {};\n", f.import, raw_params.join(", "), raw(&f.ret));
    *out += &format!("{indent}pub fn {}({}) {ret_type} {{ {body} }}\n", ident(&f.name), params.join(", "));
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
    // Any value a union member can hold.
    pub fn of(v: anytype) JsValue {
        return switch (@TypeOf(v)) {
            f64 => number(v),
            bool => boolean(v),
            else => .{ .h = v.h },
        };
    }
};
pub const JsString = extern struct {
    h: u32,
    pub fn from(s: []const u8) JsString { return .{ .h = raw.js_from_string(s.ptr, s.len) }; }
    pub fn len(self: JsString) usize { return raw.js_string_len(self.h); }
    pub fn read(self: JsString, buf: []u8) []u8 { const n = self.len(); raw.js_string_read(self.h, buf.ptr); return buf[0..n]; }
};
// A callback taking one object: `JsListener(PointerEvent).from(onClick, &state)`.
pub fn JsListener(comptime Ev: type) type {
    return extern struct {
        h: u32,
        pub const Fn = *const fn (?*anyopaque, Ev) callconv(.c) void;
        pub fn from(f: Fn, ctx: ?*anyopaque) @This() { return .{ .h = raw.js_listener_from(@intFromPtr(f), ctx) }; }
    };
}
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
    extern \"env\" fn js_listener_from(f: usize, ctx: ?*anyopaque) u32;
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
    for u in &m.unions {
        out += &union_decl(u);
        externs += &format!("    extern \"env\" fn {}_tag(h: u32) i32;\n", u.name);
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
        // A map's keys, and the type each selects, known at compile time.
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let keys: Vec<String> = map.keys.iter().map(|k| ident(k)).collect();
            out += &format!("    pub const Key = enum(i32) {{ {} }};\n", keys.join(", "));
            let arms: Vec<String> = keys.iter().zip(&map.values).map(|(k, v)| format!(".{k} => {}", typed(v, false))).collect();
            out += &format!("    pub fn Type(comptime key: Key) type {{ return switch (key) {{ {} }}; }}\n", arms.join(", "));
        }
        for a in &c.ancestors {
            out += &format!("    pub fn {}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", ident(&format!("as{a}")), c.name);
        }
        for f in &c.funcs {
            func(&mut out, &mut externs, Some(&c.name), f, "    ", &m.unions);
        }
        out += "};\n";
    }
    out += "\n";
    for f in &m.funcs {
        func(&mut out, &mut externs, None, f, "", &m.unions);
    }
    out + "\nconst raw = struct {\n" + &externs + "};\n"
}
