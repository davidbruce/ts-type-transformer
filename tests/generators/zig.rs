// XML -> Zig module for a wasm32 module. As C, but unions are kept and
// become tagged unions, and nullable objects become optionals.

use super::wasm::{Call, Func, Model, Ty};

const T: &str = "Zig";

fn raw(t: &Ty) -> &'static str {
    match t {
        Ty::Void => "void",
        Ty::Num => "f64",
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
        _ => panic!("{T} generator only supports unions of primitives and objects"),
    }
}

fn typed(t: &Ty, param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if param => "[]const u8".into(),
        Ty::Str => "JsString".into(),
        Ty::Any => "JsValue".into(),
        Ty::Obj(n) | Ty::Enum(n) => n.clone(),
        Ty::Nullable(inner, _) => format!("?{}", typed(inner, param)),
        Ty::Union(parts) => {
            let fields: Vec<String> = parts.iter().map(|p| format!("{}: {}", union_field(p), typed(p, param))).collect();
            format!("union(enum) {{ {} }}", fields.join(", "))
        }
    }
}

// Expression turning a typed value into what crosses the boundary.
fn lower_arg(name: &str, t: &Ty) -> String {
    match t {
        Ty::Num => name.into(),
        Ty::Bool => format!("@intFromBool({name})"),
        Ty::Enum(_) => format!("@intFromEnum({name})"),
        Ty::Any | Ty::Obj(_) => format!("{name}.h"),
        Ty::Nullable(..) => format!("if ({name}) |v| v.h else 0"),
        // Each alternative becomes a JavaScript value.
        Ty::Union(parts) => {
            let arms: Vec<String> = parts
                .iter()
                .map(|p| {
                    let field = union_field(p);
                    match p {
                        Ty::Num => format!(".{field} => |v| JsValue.number(v).h"),
                        Ty::Str => format!(".{field} => |v| JsValue.string(v).h"),
                        Ty::Bool => format!(".{field} => |v| JsValue.number(@floatFromInt(@intFromBool(v))).h"),
                        _ => format!(".{field} => |v| v.h"),
                    }
                })
                .collect();
            format!("switch ({name}) {{ {} }}", arms.join(", "))
        }
        Ty::Str | Ty::Void => unreachable!(),
    }
}

fn ident(name: &str) -> String {
    const KEYWORDS: &[&str] = &["type", "error", "test", "fn", "var", "const", "union", "enum", "struct", "align", "export", "resume", "suspend"];
    if KEYWORDS.contains(&name) { format!("@\"{name}\"") } else { name.to_string() }
}

fn func(out: &mut String, externs: &mut String, owner: Option<&str>, f: &Func, indent: &str) {
    let has_self = matches!(f.call, Call::Method(_) | Call::Get(_) | Call::Set(_));
    let mut raw_params = Vec::new();
    let mut params = Vec::new();
    let mut args = Vec::new();
    if has_self {
        raw_params.push("self: u32".to_string());
        params.push(format!("self: {}", owner.unwrap()));
        args.push("self.h".to_string());
    }
    for (name, t) in &f.params {
        let id = ident(name);
        params.push(format!("{id}: {}", typed(t, true)));
        if matches!(t, Ty::Str) {
            raw_params.extend([format!("{name}_ptr: [*]const u8"), format!("{name}_len: usize")]);
            args.extend([format!("{id}.ptr"), format!("{id}.len")]);
        } else {
            raw_params.push(format!("{name}: {}", raw(t)));
            args.push(lower_arg(&id, t));
        }
    }
    let call = format!("raw.{}({})", f.import, args.join(", "));
    let body = match &f.ret {
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(_) => format!("return @enumFromInt({call});"),
        Ty::Nullable(..) => format!("const h = {call}; return if (h == 0) null else .{{ .h = h }};"),
        _ => format!("return .{{ .h = {call} }};"),
    };
    *externs += &format!("    extern \"env\" fn {}({}) {};\n", f.import, raw_params.join(", "), raw(&f.ret));
    *out += &format!("{indent}pub fn {}({}) {} {{ {body} }}\n", ident(&f.name), params.join(", "), typed(&f.ret, false));
}

pub fn module(m: &Model) -> String {
    let mut out = String::from(
        "// Generated.
pub const JsValue = struct {
    h: u32,
    pub fn number(x: f64) JsValue { return .{ .h = raw.js_from_number(x) }; }
    pub fn string(s: []const u8) JsValue { return .{ .h = raw.js_from_string(s.ptr, s.len) }; }
};
pub const JsString = struct {
    h: u32,
    pub fn len(self: JsString) usize { return raw.js_string_len(self.h); }
    pub fn read(self: JsString, buf: []u8) []u8 { const n = self.len(); raw.js_string_read(self.h, buf.ptr); return buf[0..n]; }
};

",
    );
    let mut externs = String::from(
        "    extern \"env\" fn js_string_len(h: u32) usize;
    extern \"env\" fn js_string_read(h: u32, ptr: [*]u8) void;
    extern \"env\" fn js_from_number(x: f64) u32;
    extern \"env\" fn js_from_string(ptr: [*]const u8, len: usize) u32;
",
    );
    for (name, values) in &m.enums {
        let members: Vec<String> = values.iter().map(|v| ident(v)).collect();
        out += &format!("pub const {name} = enum(i32) {{ {} }};\n", members.join(", "));
    }
    // Every object is an opaque handle; methods live in the struct's namespace.
    for c in &m.classes {
        out += &format!("\npub const {} = struct {{\n    h: u32,\n", c.name);
        for (name, value) in &c.consts {
            out += &format!("    pub const {name} = {value};\n");
        }
        for a in &c.ancestors {
            out += &format!("    pub fn as{a}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", c.name);
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
