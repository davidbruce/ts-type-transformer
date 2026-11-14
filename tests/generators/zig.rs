// XML -> Zig module for a wasm32 module. As C, but unions of simple types
// become tagged unions, and nullable objects become optionals.

use std::collections::{BTreeMap, HashMap, HashSet};

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
        // An optional flag or enum is -1 when absent.
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "i32",
        _ => "u32",
    }
}

fn union_field(t: &Ty) -> String {
    match t {
        Ty::Num => "number".into(),
        Ty::Str => "string".into(),
        Ty::Bool => "boolean".into(),
        Ty::Obj(n) => n.clone(),
        Ty::Array | Ty::ArrayOf(_) | Ty::Tuple(_) => "array".into(),
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
    let decode = if u.decodable() {
        format!(
            "    // The member a handle a function returned holds; takes the handle over.\n    pub fn fromHandle(h: u32) @This() {{ return {}; }}\n    \
             // The member a handle lent to a callback holds; it stays lent.\n    pub fn lent(h: u32) @This() {{ return {}; }}\n",
            decode_union(u, "h", true),
            decode_union(u, "h", false)
        )
    } else {
        String::new()
    };
    format!(
        "pub const {0} = union(enum) {{\n    {1},\n    // Which member a value is: its index.\n    pub fn tag(h: u32) i32 {{ return raw.{0}_tag(h, 0); }}\n{decode}}};\n",
        u.name,
        fields.join(",\n    ")
    )
}

// Turns a handle into the union's member: `raw.U_tag` says which. A number
// or flag is read from the handle, and with `take` the handle released.
fn decode_union(u: &NamedUnion, h: &str, take: bool) -> String {
    let arms: Vec<String> = u
        .parts
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let value = match (p, take) {
                (Ty::Num, true) => format!("JsValue.takeNumber({h})"),
                (Ty::Bool, true) => format!("JsValue.takeBool({h})"),
                (Ty::Num, false) => format!("(JsValue{{ .h = {h} }}).toNumber()"),
                (Ty::Bool, false) => format!("(JsValue{{ .h = {h} }}).toBool()"),
                _ => format!(".{{ .h = {h} }}"),
            };
            format!("{i} => .{{ .{} = {value} }}", ident(&union_field(p)))
        })
        .collect();
    format!("switch (tag({h})) {{ {}, else => unreachable }}", arms.join(", "))
}

fn typed(t: &Ty, is_param: bool) -> String {
    match t {
        Ty::Void => "void".into(),
        Ty::Num => "f64".into(),
        Ty::Bool => "bool".into(),
        Ty::Str if is_param => "[]const u8".into(),
        Ty::Str => "JsString".into(),
        Ty::Array => "JsArray".into(),
        Ty::ArrayOf(e) => format!("JsArrayOf({})", typed(e, false)),
        Ty::Tuple(es) => format!("JsTuple(&.{{ {} }})", es.iter().map(|e| typed(e, false)).collect::<Vec<_>>().join(", ")),
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
        Ty::Param(_) | Ty::Generic(..) => panic!("Zig generator cannot express generics: the target config must erase them"),
        Ty::Any | Ty::Union(_) => "JsValue".into(),
    }
}

// Types a C-convention callback can take: no slices, no optionals, no unions.
// An optional number arrives as a lent handle, an optional flag or enum as -1 when absent.
fn callback_typed(t: &Ty) -> String {
    match t {
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_)) => "JsValue".into(),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Bool | Ty::Enum(_)) => "i32".into(),
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
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("if ({name}) |v| @as(i32, @intFromBool(v)) else -1"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("if ({name}) |v| @intFromEnum(v) else -1"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_)) => format!("lent_{name}.h"),
        Ty::Nullable(..) => format!("if ({name}) |v| v.h else 0"),
        // A union is lent for the call: see `func`.
        Ty::NamedUnion(_) | Ty::Union(_) => format!("lent_{name}.h"),
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
    // Statements before the call: a union argument made a JavaScript value,
    // released after the call if it was made for it.
    let mut pre = String::new();
    for (name, t) in &f.params {
        let id = param(name);
        if matches!(t, Ty::NamedUnion(_)) || matches!(t, Ty::Union(parts) if !parts.is_empty()) {
            pre += &format!("const lent_{id} = switch ({id}) {{ inline else => |v| Lent.of(v) }}; defer lent_{id}.done(); ");
        }
        if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::NamedUnion(_))) {
            pre += &format!("const lent_{id} = if ({id}) |u| switch (u) {{ inline else => |v| Lent.of(v) }} else Lent{{ .h = 0, .own = false }}; defer lent_{id}.done(); ");
        }
        let comptime = if matches!(t, Ty::Key(_)) { "comptime " } else { "" };
        params.push(format!("{comptime}{id}: {}", typed(t, true).replace("KEY", &key)));
        if *t == Ty::Str {
            raw_params.extend(["[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("{id}.ptr"), format!("{id}.len")]);
        } else if matches!(t, Ty::Nullable(inner, _) if matches!(**inner, Ty::Num | Ty::NumEnum(_))) {
            // An optional number: whether it is there, and the number.
            let value = if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Num) { format!("{id} orelse 0") } else { format!("if ({id}) |v| @floatFromInt(@intFromEnum(v)) else 0") };
            raw_params.extend(["i32".to_string(), "f64".to_string()]);
            args.extend([format!("@intFromBool({id} != null)"), value]);
        } else if matches!(t, Ty::Nullable(inner, _) if **inner == Ty::Str) {
            raw_params.extend(["?[*]const u8".to_string(), "usize".to_string()]);
            args.extend([format!("if ({id}) |v| v.ptr else null"), format!("if ({id}) |v| v.len else 0")]);
        } else {
            raw_params.push(raw(t).to_string());
            args.push(lower_arg(&id, t));
        }
    }
    // A shared import takes the index of this function's fixed arguments after `self`.
    let import = match &f.shared {
        Some((shared, k)) => {
            let at = usize::from(Class::has_self(f));
            raw_params.insert(at, "i32".into());
            args.insert(at, k.to_string());
            shared.as_str()
        }
        None => f.import.as_str(),
    };
    let call = format!("raw.{import}({})", args.join(", "));
    // A union result that cannot be told apart is an untyped value.
    let union = match &f.ret {
        Ty::NamedUnion(n) => unions.iter().find(|u| u.name == *n),
        Ty::Nullable(inner, _) => match &**inner {
            Ty::NamedUnion(n) => unions.iter().find(|u| u.name == *n),
            _ => None,
        },
        _ => None,
    };
    let ret_type = match union {
        Some(u) if !u.decodable() => "JsValue".to_string(),
        _ => typed(&f.ret, false).replace("KEY", &key),
    };
    let body = match &f.ret {
        Ty::NamedUnion(n) if union.is_some_and(|u| u.decodable()) => format!("return {n}.fromHandle({call});"),
        Ty::Nullable(inner, _) if union.is_some_and(|u| u.decodable()) => format!("const h = {call}; return if (h == 0) null else {}.fromHandle(h);", typed(inner, false)),
        Ty::Nullable(..) if union.is_some() => format!("return .{{ .h = {call} }};"),
        Ty::Nullable(inner, _) if **inner == Ty::Num => format!("return jsTake(?f64, {call});"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::NumEnum(_)) => format!("const h = {call}; return if (h == 0) null else @enumFromInt(@as(i32, @intFromFloat(JsValue.takeNumber(h))));"),
        Ty::Nullable(inner, _) if **inner == Ty::Bool => format!("const r = {call}; return if (r < 0) null else r != 0;"),
        Ty::Nullable(inner, _) if matches!(**inner, Ty::Enum(_)) => format!("const r = {call}; return if (r < 0) null else @enumFromInt(r);"),
        Ty::Void => format!("{call};"),
        Ty::Num => format!("return {call};"),
        Ty::Bool => format!("return {call} != 0;"),
        Ty::Enum(_) => format!("return @enumFromInt({call});"),
        Ty::NumEnum(_) => format!("return @enumFromInt(@as(i32, @intFromFloat({call})));"),
        Ty::Nullable(..) => format!("const h = {call}; return if (h == 0) null else .{{ .h = h }};"),
        _ => format!("return .{{ .h = {call} }};"),
    };
    let decl = format!("    extern \"env\" fn {import}({}) {};\n", raw_params.join(", "), raw(&f.ret));
    // Declared once per file, though several functions use it.
    // Only a shared import can repeat; checking others would be quadratic.
    if f.shared.is_none() || !externs.contains(&decl) {
        *externs += &decl;
    }
    *out += &format!("{indent}pub fn {}({}) {ret_type} {{ {pre}{body} }}\n", ident(&f.name), params.join(", "));
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
    // A number or flag from a handle a function returned, which is released.
    pub fn takeNumber(h: u32) f64 { const x = raw.js_to_number(h); raw.js_release(h); return x; }
    pub fn takeBool(h: u32) bool { const x = raw.js_to_bool(h) != 0; raw.js_release(h); return x; }
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
// A handle a function returns is the caller's: `defer jsRelease(x)`.
// Arguments, and a callback's arguments, are lent: `jsRetain` one to keep it.
// What a callback returns is given to JavaScript: `jsRetain` a handle to return it and keep it.
pub fn jsRelease(x: anytype) void { raw.js_release(x.h); }
pub fn jsRetain(x: anytype) @TypeOf(x) { raw.js_retain(x.h); return x; }
// The exception a call threw, taken. A call that throws returns a zero value.
pub fn jsTakeError() ?JsValue { const e = raw.js_take_error(); return if (e == 0) null else .{ .h = e }; }
// An exception's message, the caller's to release.
pub fn jsErrorMessage(e: JsValue) JsString { return .{ .h = raw.js_error_message(e.h) }; }
// A scope: every handle a function returns while it is the innermost open
// scope is released when it closes. `jsRetain` one to keep it.
pub const JsScope = struct {
    pub fn open() JsScope { raw.js_scope_open(); return .{}; }
    pub fn close(_: JsScope) void { raw.js_scope_close(); }
};
// How many JavaScript values wasm holds handles to.
pub fn jsLiveHandles() u32 { return raw.js_live(); }
// Forgets a callback made with `from`, and returns its context. Its handle is released as any other.
pub fn jsForgetCallback(x: anytype) ?*anyopaque { return @ptrFromInt(raw.js_callback_release(x.h)); }

// A slice of bytes, or a string literal.
fn isString(comptime T: type) bool {
    return switch (@typeInfo(T)) {
        .pointer => |p| p.child == u8 or (@typeInfo(p.child) == .array and @typeInfo(p.child).array.child == u8),
        else => false,
    };
}

// A value lent to JavaScript for one call: an object's own handle, or a
// new one for a number, flag or string, released by `done`.
pub const Lent = struct {
    h: u32,
    own: bool,
    pub fn of(v: anytype) Lent {
        const T = @TypeOf(v);
        if (T == f64 or T == comptime_float or T == comptime_int) return .{ .h = raw.js_from_number(v), .own = true };
        if (T == bool) return .{ .h = raw.js_from_bool(@intFromBool(v)), .own = true };
        if (comptime isString(T)) { const s: []const u8 = v; return .{ .h = raw.js_from_string(s.ptr, s.len), .own = true }; }
        if (@typeInfo(T) == .@\"union\") return switch (v) { inline else => |x| of(x) };
        return .{ .h = v.h, .own = false };
    }
    pub fn done(self: Lent) void { if (self.own) raw.js_release(self.h); }
};

// A T from a handle a function returned: takes the handle over.
pub fn jsTake(comptime T: type, h: u32) T {
    if (@typeInfo(T) == .optional) return if (h == 0) null else jsTake(@typeInfo(T).optional.child, h);
    if (T == f64) return JsValue.takeNumber(h);
    if (T == bool) return JsValue.takeBool(h);
    if (@typeInfo(T) == .@\"union\") return T.fromHandle(h);
    return .{ .h = h };
}

// A JavaScript iterator yielding T. Each value is the caller's; so is the iterator.
pub fn JsIteratorOf(comptime T: type) type {
    return extern struct {
        h: u32,
        pub fn over(h: u32) @This() { return .{ .h = raw.js_iterate(h) }; }
        // The next value, or null at the end.
        pub fn next(self: @This()) ?T { const v = raw.js_iterator_next(self.h); return if (v == 0xFFFFFFFF) null else jsTake(T, v); }
    };
}

// A JavaScript array of fixed length and element types: `[number, string]` is
// `JsTuple(&.{ f64, JsString })`. `get(0)` reads an element, the caller's.
pub fn JsTuple(comptime Types: []const type) type {
    return extern struct {
        h: u32,
        // From a Zig tuple of values: `.{ 1, \"one\" }`.
        pub fn of(values: anytype) @This() {
            const a: @This() = .{ .h = raw.js_array_new() };
            inline for (values) |v| { const l = Lent.of(v); defer l.done(); raw.js_array_push(a.h, l.h); }
            return a;
        }
        pub fn get(self: @This(), comptime i: usize) Types[i] { return jsTake(Types[i], raw.js_array_get(self.h, i)); }
    };
}

// A JavaScript array of T: f64, bool, JsString, a handle, or a union.
pub fn JsArrayOf(comptime T: type) type {
    return extern struct {
        h: u32,
        pub fn new() @This() { return .{ .h = raw.js_array_new() }; }
        pub fn of(items: []const T) @This() { const a = new(); for (items) |x| a.push(x); return a; }
        pub fn len(self: @This()) usize { return raw.js_array_length(self.h); }
        // What it returns is the caller's, as from any function.
        pub fn get(self: @This(), i: usize) T { return jsTake(T, raw.js_array_get(self.h, i)); }
        pub fn push(self: @This(), v: T) void { const l = Lent.of(v); defer l.done(); raw.js_array_push(self.h, l.h); }
        pub fn pushString(self: @This(), s: []const u8) void { const l = Lent.of(s); defer l.done(); raw.js_array_push(self.h, l.h); }
        // The same array, untyped.
        pub fn any(self: @This()) JsArray { return .{ .h = self.h }; }
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
    extern \"env\" fn js_retain(h: u32) void;
    extern \"env\" fn js_release(h: u32) void;
    extern \"env\" fn js_live() u32;
    extern \"env\" fn js_take_error() u32;
    extern \"env\" fn js_scope_open() void;
    extern \"env\" fn js_scope_close() void;
    extern \"env\" fn js_iterate(h: u32) u32;
    extern \"env\" fn js_iterator_next(it: u32) u32;
    extern \"env\" fn js_error_message(h: u32) u32;
    extern \"env\" fn js_callback_release(h: u32) usize;
";

// The bindings as files: `runtime.zig`; `types.zig` with enums, unions,
// callbacks and handles declared elsewhere; one file per module with its
// types and functions; and `bindings.zig`, which re-exports every name. Each
// file imports by name what it uses from the others, and Zig only reads a
// file when one of its names is used.
pub fn files(m: &Model) -> Vec<(String, String)> {
    // File -> (declarations, externs, names it declares).
    let mut files: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    files.insert("runtime".into(), (RUNTIME.into(), RUNTIME_EXTERNS.into(), ["JsValue", "JsString", "JsArray", "JsArrayOf", "JsTuple", "JsIteratorOf", "JsListener", "Lent", "jsRelease", "jsRetain", "jsLiveHandles", "jsForgetCallback", "jsTakeError", "jsErrorMessage", "JsScope", "jsTake"].map(String::from).to_vec()));
    let mut types = (String::new(), String::new(), Vec::new());
    for (name, values) in &m.enums {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<String> = values.iter().map(|v| if v.is_empty() { "empty" } else { v }).filter(|v| seen.insert(v.to_string())).map(ident).collect();
        types.0 += &format!("pub const {name} = enum(i32) {{ {} }};\n", members.join(", "));
        types.2.push(name.clone());
    }
    for (name, members) in &m.num_enums {
        let members: Vec<String> = members.iter().map(|(member, value)| format!("{} = {value}", ident(member))).collect();
        types.0 += &format!("pub const {name} = enum(i32) {{ {} }};\n", members.join(", "));
        types.2.push(name.clone());
    }
    for u in &m.unions {
        types.0 += &union_decl(u);
        types.1 += &format!("    extern \"env\" fn {}_tag(h: u32, k: i32) i32;\n", u.name);
        types.2.push(u.name.clone());
    }
    let iterates_async = m.classes.iter().any(|c| c.iterates_async.is_some());
    let mut opaque = m.opaque.clone();
    if iterates_async && !opaque.iter().any(|n| n == "Promise") {
        opaque.push("Promise".into());
    }
    if iterates_async {
        types.0 += "// A JavaScript async iterator: `next()` is a promise of the next value, or of a null handle at the end.\n\
                    pub const JsAsyncIterator = extern struct {\n    h: u32,\n    \
                    pub fn over(h: u32) JsAsyncIterator { return .{ .h = raw.js_async_iterate(h) }; }\n    \
                    pub fn next(self: JsAsyncIterator) Promise { return .{ .h = raw.js_async_iterator_next(self.h) }; }\n};\n";
        types.1 += "    extern \"env\" fn js_async_iterate(h: u32) u32;\n    extern \"env\" fn js_async_iterator_next(it: u32) u32;\n";
        types.2.push("JsAsyncIterator".into());
    }
    for name in &opaque {
        if name == "Promise" {
            // Calls f(ctx, ok, value) once it settles, ok with its value or not with its reason; the value is lent.
            types.0 += "pub const Promise = extern struct {\n    h: u32,\n    \
                        pub fn then(self: Promise, comptime f: fn (?*anyopaque, bool, JsValue) void, ctx: ?*anyopaque) void {\n        \
                        const T = struct {\n            fn call(c: ?*anyopaque, ok: i32, v: u32) callconv(.c) void { f(c, ok != 0, .{ .h = v }); }\n        };\n        \
                        raw.js_promise_then(self.h, @intFromPtr(&T.call), ctx);\n    }\n};\n";
            types.1 += "    extern \"env\" fn js_promise_then(p: u32, f: usize, ctx: ?*anyopaque) void;\n";
        } else {
            types.0 += &format!("pub const {name} = extern struct {{ h: u32 }};\n");
        }
        types.2.push(name.clone());
    }
    // A callback is made from a function and a context pointer passed back to it.
    let decodable = |t: &Ty| matches!(t, Ty::NamedUnion(n) if m.unions.iter().any(|u| u.name == *n && u.decodable()));
    for c in m.callbacks.iter().filter(|c| c.params.iter().any(|p| decodable(&p.1))) {
        // The function is known at compile time: a C-convention trampoline
        // decodes the union arguments and calls it.
        let mut params = vec!["?*anyopaque".to_string()];
        params.extend(c.params.iter().map(|(_, t)| if decodable(t) { typed(t, false) } else { callback_typed(t) }));
        let raw_params: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| format!("a{i}: {}", if decodable(t) { "u32".into() } else { callback_typed(t) })).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| if decodable(t) { format!("{}.lent(a{i})", typed(t, false)) } else { format!("a{i}") }).collect();
        let ret = callback_typed(&c.ret);
        types.0 += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        types.0 += &format!("    pub const Fn = fn ({}) {ret};\n", params.join(", "));
        types.0 += &format!(
            "    pub fn from(comptime f: Fn, ctx: ?*anyopaque) {0} {{\n        const T = struct {{\n            fn call(c: ?*anyopaque, {1}) callconv(.c) {ret} {{ return f(c, {2}); }}\n        }};\n        return .{{ .h = raw.{0}_from(@intFromPtr(&T.call), ctx) }};\n    }}\n}};\n",
            c.name,
            raw_params.join(", "),
            args.join(", ")
        );
        types.1 += &format!("    extern \"env\" fn {}_from(f: usize, ctx: ?*anyopaque) u32;\n", c.name);
        types.2.push(c.name.clone());
    }
    for c in m.callbacks.iter().filter(|c| !c.params.iter().any(|p| decodable(&p.1))) {
        let mut params = vec!["?*anyopaque".to_string()];
        params.extend(c.params.iter().map(|(_, t)| callback_typed(t)));
        types.0 += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        types.0 += &format!("    pub const Fn = *const fn ({}) callconv(.c) {};\n", params.join(", "), callback_typed(&c.ret));
        types.0 += &format!("    pub fn from(f: Fn, ctx: ?*anyopaque) {} {{ return .{{ .h = raw.{}_from(f, ctx) }}; }}\n}};\n", c.name, c.name);
        types.1 += &format!("    extern \"env\" fn {0}_from(f: {0}.Fn, ctx: ?*anyopaque) u32;\n", c.name);
        types.2.push(c.name.clone());
    }
    files.insert("types".into(), types);
    // Every object is an opaque handle; methods live in the struct's namespace.
    for c in &m.classes {
        let (out, externs, names) = files.entry(c.module.clone()).or_default();
        names.push(c.name.clone());
        *out += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        for (name, value) in &c.consts {
            *out += &format!("    pub const {} = {value};\n", ident(name));
        }
        // Asynchronous iteration: `it.next()` is a promise of the next value, or of a null handle at the end.
        if c.iterates_async.is_some() && !c.funcs.iter().any(|f| f.name == "iterateAsync") {
            *out += &format!("    pub fn iterateAsync(self: {}) JsAsyncIterator {{ return JsAsyncIterator.over(self.h); }}\n", c.name);
        }
        // Iteration: `var it = list.iterate(); while (it.next()) |n| ...`.
        if let Some(elem) = &c.iterates
            && !c.funcs.iter().any(|f| f.name == "iterate")
        {
            *out += &format!("    pub fn iterate(self: {}) JsIteratorOf({}) {{ return JsIteratorOf({1}).over(self.h); }}\n", c.name, typed(elem, false));
        }
        // A map's keys, and the type each selects, known at compile time.
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let keys: Vec<String> = map.keys.iter().map(|k| ident(k)).collect();
            *out += &format!("    pub const Key = enum(i32) {{ {} }};\n", keys.join(", "));
            let arms: Vec<String> = keys.iter().zip(&map.values).map(|(k, v)| format!(".{k} => {}", typed(v, false))).collect();
            *out += &format!("    pub fn Type(comptime key: Key) type {{ return switch (key) {{ {} }}; }}\n", arms.join(", "));
        }
        for a in &c.ancestors {
            *out += &format!("    pub fn {}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", ident(&format!("as{a}")), c.name);
        }
        for f in &c.funcs {
            func(out, externs, Some(&c.name), f, "    ", &m.unions);
        }
        *out += "};\n";
    }
    for f in &m.funcs {
        let (out, externs, names) = files.entry(f.module.clone()).or_default();
        names.push(f.name.clone());
        func(out, externs, None, f, "", &m.unions);
    }

    // Which file declares each name.
    let home: HashMap<&str, &str> = files.iter().flat_map(|(file, (_, _, names))| names.iter().map(move |n| (n.as_str(), file.as_str()))).collect();
    let mut out = Vec::new();
    let mut root = String::from("// Generated. Every name, from the file that declares it.\n");
    for (file, (body, externs, names)) in &files {
        let own: HashSet<&str> = names.iter().map(String::as_str).collect();
        let mut used: Vec<&str> = identifiers(body).into_iter().filter(|n| !own.contains(n) && home.contains_key(n)).collect();
        used.sort();
        used.dedup();
        // Paths are relative to the importing file: `areas/css.zig` imports `../types.zig`.
        let up = "../".repeat(file.matches('/').count());
        let imports: String = used.iter().map(|n| format!("const {n} = @import(\"{up}{}.zig\").{n};\n", home[n])).collect();
        let body = if file == "runtime" { body.clone() } else { format!("// Generated.\n{imports}\n{body}") };
        out.push((format!("{file}.zig"), format!("{body}\nconst raw = struct {{\n{externs}}};\n")));
        for n in names {
            root += &format!("pub const {} = @import(\"{file}.zig\").{};\n", ident(n), ident(n));
        }
    }
    out.push(("bindings.zig".into(), root));
    out
}

// The plain identifiers in Zig source: not quoted ones (`@"x"`), not strings.
fn identifiers(src: &str) -> HashSet<&str> {
    let mut out = HashSet::new();
    let b = src.as_bytes();
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'"' => {
                i += 1;
                while i < b.len() && b[i] != b'"' {
                    i += if b[i] == b'\\' { 2 } else { 1 };
                }
                i += 1;
            }
            b'/' if b.get(i + 1) == Some(&b'/') => {
                while i < b.len() && b[i] != b'\n' {
                    i += 1;
                }
            }
            c if c.is_ascii_alphabetic() || c == b'_' => {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                // Not a field (`.name`) or builtin (`@name`).
                if start == 0 || !matches!(b[start - 1], b'.' | b'@') {
                    out.insert(&src[start..i]);
                }
            }
            _ => i += 1,
        }
    }
    out
}
