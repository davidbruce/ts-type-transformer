use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use super::super::comments::{Style, comment};
use super::types::{
    callback_arg, callback_param_ty, callback_result, func, names_of, raw_callback, type_refs,
    typed, union_decl, zig_elem,
};
use super::{ident, zig_func_name, ProjectRef, RUNTIME, RUNTIME_EXTERNS, RUNTIME_NAMES};
use super::super::model::{Call, Class, Func, Model, Ty};

// The bindings as files: `runtime.zig`; `types.zig` with enums, unions,
// callbacks and handles declared elsewhere; one file per module with its
// types and functions; and `bindings.zig`, which re-exports every name. Each
// file imports by name what it uses from the others, and Zig only reads a
// file when one of its names is used.
// What Zig cannot express of a model: generics.
fn check(m: &Model) -> super::super::Problems {
    let mut problems = super::super::Problems::default();
    super::super::model::each_type(m, &mut |at, t| {
        if matches!(t, Ty::Param(_) | Ty::Generic(..)) {
            problems.add("Zig generator cannot express generics: set generics = \"erase\"".into(), at.to_string());
        }
    });
    problems
}

// Enum members on one line; where some have documentation, one to a line.
fn enum_fields<'a>(fields: impl Iterator<Item = (String, Option<&'a str>)>) -> String {
    let fields: Vec<(String, Option<&str>)> = fields.collect();
    if fields.iter().all(|f| f.1.is_none()) {
        return fields.into_iter().map(|f| f.0).collect::<Vec<_>>().join(", ");
    }
    let mut out = String::from("\n");
    for (field, doc) in fields {
        out += &doc.map(|d| comment(d, "///", "    ", Style::Plain)).unwrap_or_default();
        out += &format!("    {field},\n");
    }
    out.trim_end_matches(",\n").to_string()
}

pub(super) fn files_with_imports(m: &Model, pkg_name: &str, refs: &[ProjectRef]) -> Result<(Vec<(String, String)>, HashSet<String>), Vec<String>> {
    let problems = check(m);
    if !problems.is_empty() {
        return Err(problems.into_messages());
    }
    let wasm_module = if pkg_name == "bindings" { "env" } else { pkg_name };
    let mut emitted_imports: HashSet<String> = HashSet::new();
    let mut resolved_opaque: HashMap<String, (usize, String)> = HashMap::new();
    let mut used_refs: BTreeSet<usize> = BTreeSet::new();
    for name in &m.opaque {
        if matches!(name.as_str(), "Promise" | "Error" | "Date") || super::super::model::buffer_spec(name).is_some() {
            continue;
        }
        let orig = m.opaque_names.get(name).map(String::as_str).unwrap_or(name.as_str());
        if let Some((idx, _)) = refs.iter().enumerate().find(|(_, r)| r.types.contains(orig)) {
            resolved_opaque.insert(name.clone(), (idx, orig.to_string()));
        }
    }
    // File -> (declarations, externs, names it declares).
    let mut files: BTreeMap<String, (String, String, Vec<String>)> = BTreeMap::new();
    // File -> the names of other files its declarations may use: the types
    // they name, from the model, not from the source written.
    let mut uses: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    files.insert("runtime".into(), (RUNTIME.into(), RUNTIME_EXTERNS.into(), RUNTIME_NAMES.iter().map(|n| n.to_string()).collect()));
    let mut types = (String::new(), String::new(), Vec::new());
    for (id, (name, values)) in m.enums.iter().enumerate() {
        let mut seen = std::collections::HashSet::new();
        let members: Vec<(usize, String)> = values.iter().enumerate().map(|(i, v)| (i, if v.is_empty() { "empty" } else { v.as_str() })).filter(|(_, v)| seen.insert(v.to_string())).map(|(i, v)| (i, ident(v))).collect();
        let docs = m.enum_docs.get(id);
        types.0 += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default();
        // `js_id`: which enum of strings it is, for its values to cross as handles.
        types.0 += &format!(
            "pub const {name} = enum(i32) {{ {}, pub const js_id: i32 = {id}; pub fn toJsHandle(self: @This()) u32 {{ return raw.js_enum_value({id}, @intFromEnum(self)); }} pub fn fromJsHandle(h: u32) @This() {{ return @enumFromInt(raw.js_enum_index({id}, h)); }} }};\n",
            enum_fields(members.iter().map(|(i, name)| (name.clone(), docs.and_then(|d| d.members[*i].as_deref()))))
        );
        types.2.push(name.clone());
    }
    if !m.enums.is_empty() {
        types.1 += &format!(
            "    extern \"{wasm_module}\" fn js_enum_value(id: i32, i: i32) u32;\n    extern \"{wasm_module}\" fn js_enum_index(id: i32, h: u32) i32;\n"
        );
    }
    for (id, (name, members)) in m.num_enums.iter().enumerate() {
        // A value named twice (`LEFT = 0`, `ROTATE = 0`): the second is a constant of the first.
        let mut first: HashMap<&str, &str> = HashMap::new();
        let mut fields = Vec::new();
        let mut aliases = String::new();
        let docs = m.num_enum_docs.get(id);
        for (i, (member, value)) in members.iter().enumerate() {
            match first.get(value.as_str()) {
                Some(f) => aliases += &format!(" pub const {} = @This().{};", ident(member), ident(f)),
                None => {
                    first.insert(value, member);
                    fields.push((format!("{} = {value}", ident(member)), docs.and_then(|d| d.members[i].as_deref())));
                }
            }
        }
        types.0 += &docs.and_then(|d| d.doc.as_deref()).map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default();
        types.0 += &format!("pub const {name} = enum(i32) {{ {},{aliases} }};\n", enum_fields(fields.into_iter()));
        types.2.push(name.clone());
    }
    let types_uses = uses.entry("types".into()).or_default();
    for u in &m.unions {
        u.parts.iter().for_each(|p| type_refs(p, types_uses));
        types.0 += &union_decl(u);
        emitted_imports.insert(format!("{}_tag", u.name));
        types.1 += &format!("    extern \"{wasm_module}\" fn {}_tag(h: u32, k: i32) i32;\n", u.name);
        types.2.push(u.name.clone());
    }
    for (alias, to) in m.type_aliases.iter().filter(|(_, to)| m.unions.iter().any(|u| u.name == *to)) {
        types.0 += &format!("pub const {alias} = {to};\n");
        types.2.push(alias.clone());
    }
    let iterates_async = m.classes.iter().any(|c| c.iterates_async.is_some());
    let mut opaque = m.opaque.clone();
    if iterates_async && !opaque.iter().any(|n| n == "Promise") {
        opaque.push("Promise".into());
    }
    for c in &m.callbacks {
        c.params.iter().for_each(|p| type_refs(&p.1, types_uses));
        type_refs(&c.ret, types_uses);
    }
    if iterates_async {
        types.0 += "// A JavaScript async iterator: `next()` is a promise of the next value, or of a null handle at the end.\n\
                    pub const JsAsyncIterator = extern struct {\n    h: u32,\n    \
                    pub fn over(h: u32) JsAsyncIterator { return .{ .h = raw.js_async_iterate(h) }; }\n    \
                    pub fn next(self: JsAsyncIterator) Promise { return .{ .h = raw.js_async_iterator_next(self.h) }; }\n    \
                    pub fn deinit(self: JsAsyncIterator) void { jsRelease(self); }\n    \
                    pub fn retain(self: JsAsyncIterator) JsAsyncIterator { return jsRetain(self); }\n    \
                    pub fn as(self: JsAsyncIterator, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        types.1 += "    extern \"env\" fn js_async_iterate(h: u32) u32;\n    extern \"env\" fn js_async_iterator_next(it: u32) u32;\n";
        types.2.push("JsAsyncIterator".into());
    }
    let mut need_error_externs = false;
    let mut need_date_externs = false;
    let mut need_bytes_externs = false;
    for name in &opaque {
        if name == "Promise" {
            // Calls f(ok, value) / f(ctx, ok, value) once it settles; the value is lent.
            types.0 += r#"pub const Promise = extern struct {
    h: u32,
    pub fn then(self: Promise, comptime f_: anytype, ctx_: anytype) void {
        const info_ = @typeInfo(@TypeOf(f_)).@"fn";
        if (info_.param_types.len == 2) {
            const T_ = struct { fn call(_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void { f_(ok_ != 0, .{ .h = v_ }); } };
            raw.js_promise_then(self.h, @intFromPtr(&T_.call), null);
            return;
        }
        const Ptr_ = info_.param_types[0].?;
        if (Ptr_ == ?*anyopaque) {
            const T_ = struct { fn call(c_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void { f_(c_, ok_ != 0, .{ .h = v_ }); } };
            raw.js_promise_then(self.h, @intFromPtr(&T_.call), Lent.toRawCtx(ctx_));
            return;
        }
        const typed_ctx_: Ptr_ = ctx_;
        const raw_ctx_ = Lent.toRawCtx(typed_ctx_);
        const T_ = struct {
            fn call(c_: ?*anyopaque, ok_: i32, v_: u32) callconv(.c) void {
                if (@typeInfo(Ptr_) == .optional) {
                    f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null, ok_ != 0, .{ .h = v_ });
                } else {
                    f_(@ptrCast(@alignCast(c_.?)), ok_ != 0, .{ .h = v_ });
                }
            }
        };
        raw.js_promise_then(self.h, @intFromPtr(&T_.call), raw_ctx_);
    }
    pub fn thenFn(self: Promise, comptime f_: anytype) void { self.then(f_, null); }
    pub fn deinit(self: Promise) void { jsRelease(self); }
    pub fn retain(self: Promise) Promise { return jsRetain(self); }
    pub fn as(self: Promise, comptime Target_: type) Target_ { return .{ .h = self.h }; }
};
"#;
            types.1 += "    extern \"env\" fn js_promise_then(p: u32, f: usize, ctx: ?*anyopaque) void;\n";
        } else if name == "Error" {
            need_error_externs = true;
            types.0 += "pub const Error = extern struct {\n    h: u32,\n    \
                        pub fn new(msg: []const u8) Error { return .{ .h = raw.js_error_new(msg.ptr, msg.len) }; }\n    \
                        pub fn name(self: Error) JsString { return .{ .h = raw.js_error_name(self.h) }; }\n    \
                        pub fn message(self: Error) JsString { return .{ .h = raw.js_error_message(self.h) }; }\n    \
                        pub fn stack(self: Error) JsString { return .{ .h = raw.js_error_stack(self.h) }; }\n    \
                        pub fn deinit(self: Error) void { jsRelease(self); }\n    \
                        pub fn retain(self: Error) Error { return jsRetain(self); }\n    \
                        pub fn as(self: Error, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        } else if name == "Date" {
            need_date_externs = true;
            types.0 += "pub const Date = extern struct {\n    h: u32,\n    \
                        pub fn fromTime(ms: f64) Date { return .{ .h = raw.js_date_new(ms) }; }\n    \
                        pub fn now() f64 { return raw.js_date_now(); }\n    \
                        pub fn time(self: Date) f64 { return raw.js_date_time(self.h); }\n    \
                        pub fn toIsoString(self: Date) JsString { return .{ .h = raw.js_date_to_iso_string(self.h) }; }\n    \
                        pub fn deinit(self: Date) void { jsRelease(self); }\n    \
                        pub fn retain(self: Date) Date { return jsRetain(self); }\n    \
                        pub fn as(self: Date, comptime Target_: type) Target_ { return .{ .h = self.h }; }\n};\n";
        } else if let Some(s) = super::super::model::buffer_spec(name) {
            need_bytes_externs = true;
            types.0 += &format!(
                "pub const {name} = extern struct {{\n    h: u32,\n    \
                 pub fn byteLength(self: {name}) usize {{ return raw.js_bytes_len(self.h); }}\n    \
                 pub fn readBytes(self: {name}, offset: usize, dst: []u8) usize {{ return raw.js_bytes_read(self.h, offset, dst.ptr, dst.len); }}\n    \
                 pub fn writeBytes(self: {name}, offset: usize, src: []const u8) usize {{ return raw.js_bytes_write(self.h, offset, src.ptr, src.len); }}\n"
            );
            if s.has_buffer() {
                types.0 += &format!(
                    "    pub fn byteOffset(self: {name}) usize {{ return raw.js_bytes_offset(self.h); }}\n    \
                     pub fn buffer(self: {name}) JsValue {{ return .{{ .h = raw.js_bytes_buffer(self.h) }}; }}\n"
                );
            }
            if s.is_typed_array() {
                let kind = s.kind.unwrap();
                let elem = zig_elem(s.elem);
                let eb = s.elem.bytes();
                types.0 += &format!(
                    "    pub fn new(count: usize) {name} {{ const n = if (count > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else count * {eb}; return .{{ .h = raw.js_bytes_new({kind}, null, n) }}; }}\n    \
                     pub fn from(data: []const {elem}) {name} {{ const n = if (data.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else data.len * {eb}; return .{{ .h = raw.js_bytes_new({kind}, @ptrCast(data.ptr), n) }}; }}\n    \
                     pub fn len(self: {name}) usize {{ return raw.js_bytes_len(self.h) / {eb}; }}\n    \
                     pub fn read(self: {name}, elem_offset: usize, dst: []{elem}) usize {{ if (elem_offset > (~@as(usize, 0)) / {eb}) return 0; const n = if (dst.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else dst.len * {eb}; return raw.js_bytes_read(self.h, elem_offset * {eb}, @ptrCast(dst.ptr), n) / {eb}; }}\n    \
                     pub fn write(self: {name}, elem_offset: usize, src: []const {elem}) usize {{ if (elem_offset > (~@as(usize, 0)) / {eb}) return 0; const n = if (src.len > (~@as(usize, 0)) / {eb}) ~@as(usize, 0) else src.len * {eb}; return raw.js_bytes_write(self.h, elem_offset * {eb}, @ptrCast(src.ptr), n) / {eb}; }}\n"
                );
            } else {
                if let Some(kind) = s.kind {
                    types.0 += &format!(
                        "    pub fn new(byte_len: usize) {name} {{ return .{{ .h = raw.js_bytes_new({kind}, null, byte_len) }}; }}\n    \
                         pub fn from(data: []const u8) {name} {{ return .{{ .h = raw.js_bytes_new({kind}, data.ptr, data.len) }}; }}\n"
                    );
                }
                types.0 += &format!(
                    "    pub fn read(self: {name}, offset: usize, dst: []u8) usize {{ return raw.js_bytes_read(self.h, offset, dst.ptr, dst.len); }}\n    \
                     pub fn write(self: {name}, offset: usize, src: []const u8) usize {{ return raw.js_bytes_write(self.h, offset, src.ptr, src.len); }}\n"
                );
            }
            types.0 += &format!(
                "    pub fn deinit(self: {name}) void {{ jsRelease(self); }}\n    \
                 pub fn retain(self: {name}) {name} {{ return jsRetain(self); }}\n    \
                 pub fn as(self: {name}, comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }}\n}};\n"
            );
        } else if let Some((idx, orig)) = resolved_opaque.get(name) {
            used_refs.insert(*idx);
            types.0 += &format!("pub const {name} = @import(\"{}\").{orig};\n", refs[*idx].pkg_name);
        } else {
            types.0 += &format!("// external\npub const {name} = extern struct {{ h: u32, pub fn deinit(self: @This()) void {{ jsRelease(self); }} pub fn retain(self: @This()) @This() {{ return jsRetain(self); }} pub fn as(self: @This(), comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }} }};\n");
        }
        types.2.push(name.clone());
    }
    if need_error_externs {
        types.1 += "    extern \"env\" fn js_error_new(ptr: [*]const u8, len: usize) u32;\n    \
                    extern \"env\" fn js_error_name(h: u32) u32;\n    \
                    extern \"env\" fn js_error_message(h: u32) u32;\n    \
                    extern \"env\" fn js_error_stack(h: u32) u32;\n";
    }
    if need_date_externs {
        types.1 += "    extern \"env\" fn js_date_new(ms: f64) u32;\n    \
                    extern \"env\" fn js_date_now() f64;\n    \
                    extern \"env\" fn js_date_time(h: u32) f64;\n    \
                    extern \"env\" fn js_date_to_iso_string(h: u32) u32;\n";
    }
    if need_bytes_externs {
        types.1 += "    extern \"env\" fn js_bytes_len(h: u32) usize;\n    \
                    extern \"env\" fn js_bytes_read(h: u32, off: usize, dst: [*]u8, len: usize) usize;\n    \
                    extern \"env\" fn js_bytes_write(h: u32, off: usize, src: [*]const u8, len: usize) usize;\n    \
                    extern \"env\" fn js_bytes_new(kind: u32, src: ?[*]const u8, byte_len: usize) u32;\n    \
                    extern \"env\" fn js_bytes_buffer(h: u32) u32;\n    \
                    extern \"env\" fn js_bytes_offset(h: u32) usize;\n";
    }
    // A callback is made from a function and a context pointer passed back to it.
    // Supports stateless `fn (...) R`, typed ctx `fn (*T, ...) R`, or legacy `fn (?*anyopaque, ...) R`.
    let mut cb_apply_imports: HashSet<String> = HashSet::new();
    for c in &m.callbacks {
        let mut params = vec!["?*anyopaque".to_string()];
        params.extend(c.params.iter().map(|(_, t)| callback_param_ty(t, &m.unions)));
        let raw_params: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| format!("a{i}: {}", raw_callback(t))).collect();
        let args: Vec<String> = c.params.iter().enumerate().map(|(i, (_, t))| callback_arg(t, &format!("a{i}"), &m.unions)).collect();
        let args_str = args.join(", ");
        let n_params = c.params.len();
        let ret = typed(&c.ret, false);
        let raw_ret = raw_callback(&c.ret);
        let raw_suffix: String = raw_params.iter().map(|p| format!(", {p}")).collect();
        let call_stateless = format!("f_({args_str})");
        let call_c = if args_str.is_empty() { "f_(c_)".to_string() } else { format!("f_(c_, {args_str})") };
        let call_opt = if args_str.is_empty() {
            "f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null)".to_string()
        } else {
            format!("f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null, {args_str})")
        };
        let call_ptr = if args_str.is_empty() {
            "f_(@ptrCast(@alignCast(c_.?)))".to_string()
        } else {
            format!("f_(@ptrCast(@alignCast(c_.?)), {args_str})")
        };
        let body_stateless = callback_result(&c.ret, &call_stateless);
        let body_c = callback_result(&c.ret, &call_c);
        let body_opt = callback_result(&c.ret, &call_opt);
        let body_ptr = callback_result(&c.ret, &call_ptr);
        types.0 += &format!("\npub const {} = extern struct {{\n    h: u32,\n", c.name);
        types.0 += &format!("    pub const Fn = fn ({}) {ret};\n", params.join(", "));
        types.0 += &format!(
            "    pub fn from(comptime f_: anytype, ctx_: anytype) {0} {{\n        \
             const info_ = @typeInfo(@TypeOf(f_)).@\"fn\";\n        \
             if (info_.param_types.len == {n_params}) {{\n            \
                 const T_ = struct {{\n                fn call(_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{ {body_stateless} }}\n            }};\n            \
                 return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), null) }};\n        }}\n        \
             const Ptr_ = info_.param_types[0].?;\n        \
             if (Ptr_ == ?*anyopaque) {{\n            \
                 const T_ = struct {{\n                fn call(c_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{ {body_c} }}\n            }};\n            \
                 return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), Lent.toRawCtx(ctx_)) }};\n        }}\n        \
             const typed_ctx_: Ptr_ = ctx_;\n        \
             const raw_ctx_ = Lent.toRawCtx(typed_ctx_);\n        \
             const T_ = struct {{\n            \
                 fn call(c_: ?*anyopaque{raw_suffix}) callconv(.c) {raw_ret} {{\n                \
                     if (@typeInfo(Ptr_) == .optional) {{ {body_opt} }} else {{ {body_ptr} }}\n            \
                 }}\n        }};\n        \
             return .{{ .h = raw.{0}_from(@intFromPtr(&T_.call), raw_ctx_) }};\n    }}\n    \
             pub fn fromFn(comptime f_: anytype) {0} {{ return from(f_, null); }}\n",
            c.name
        );
        if let Some(apply_f) = m.funcs.iter().find(|f| f.call == Call::Apply && matches!(f.params.first(), Some((_, Ty::Obj(n))) if n == &c.name)) {
            cb_apply_imports.insert(apply_f.import.clone());
            for p in &apply_f.params[1..] {
                type_refs(&p.1, types_uses);
            }
            type_refs(&apply_f.ret, types_uses);
            let method = Func {
                type_params: Vec::new(),
                module: String::new(),
                import: apply_f.import.clone(),
                name: "call".into(),
                call: Call::Invoke,
                params: apply_f.params[1..].to_vec(),
                rest: apply_f.rest,
                fixed: apply_f.fixed.clone(),
                shared: apply_f.shared.clone(),
                ret: apply_f.ret.clone(),
                returns_this: false,
                doc: None,
            };
            func(&mut types.0, &mut types.1, Some(&c.name), &method, "call", "    ", &m.unions, &m.callbacks, wasm_module, &mut emitted_imports);
        }
        types.0 += &format!(
            "    pub fn deinit(self: {0}) void {{ jsRelease(self); }}\n    \
             pub fn retain(self: {0}) {0} {{ return jsRetain(self); }}\n    \
             pub fn as(self: {0}, comptime Target_: type) Target_ {{ return .{{ .h = self.h }}; }}\n}};\n",
            c.name
        );
        emitted_imports.insert(format!("{}_from", c.name));
        types.1 += &format!("    extern \"{wasm_module}\" fn {}_from(f: usize, ctx: ?*anyopaque) u32;\n", c.name);
        types.2.push(c.name.clone());
    }
    files.insert("types".into(), types);
    // With inheritance flattened, a type has its ancestors' members. Each is
    // written once, by the furthest ancestor that declares it, in
    // `X.Methods(Self)`; a subtype's member of that name is an alias of it.
    let class_of: HashMap<&str, &Class> = m.classes.iter().map(|c| (c.name.as_str(), c)).collect();
    let mut ancestors: HashMap<&str, Vec<&Class>> = HashMap::new();
    for c in &m.classes {
        let mut up: Vec<&Class> = Vec::new();
        let mut todo: Vec<&Class> = vec![c];
        while let Some(k) = todo.pop() {
            for a in k.ancestors.iter().filter_map(|a| class_of.get(a.as_str())) {
                if !up.iter().any(|u| u.name == a.name) {
                    up.push(a);
                    todo.push(a);
                }
            }
        }
        up.sort_by(|a, b| a.name.cmp(&b.name));
        ancestors.insert(&c.name, up);
    }
    let same = |a: &Func, b: &Func| a.call == b.call && a.params == b.params && (a.ret == b.ret || (a.returns_this && b.returns_this)) && a.rest == b.rest && a.fixed == b.fixed && a.type_params == b.type_params;
    let members: HashMap<&str, HashMap<String, Vec<&Func>>> = m
        .classes
        .iter()
        .map(|c| {
            let mut by_call: HashMap<String, Vec<&Func>> = HashMap::new();
            c.funcs.iter().filter(|f| Class::has_self(f)).for_each(|f| by_call.entry(format!("{:?}", f.call)).or_default().push(f));
            (c.name.as_str(), by_call)
        })
        .collect();
    let declares = |k: &Class, f: &Func| members[k.name.as_str()].get(&format!("{:?}", f.call)).and_then(|fs| fs.iter().find(|g| same(g, f)).copied());
    // (type, member) -> (the ancestor that writes it, its name there).
    let mut homes: HashMap<(&str, &str), (&str, &str)> = HashMap::new();
    for c in &m.classes {
        for f in c.funcs.iter().filter(|f| Class::has_self(f)) {
            let home = ancestors[c.name.as_str()].iter().filter_map(|a| declares(a, f).map(|g| (*a, g))).find(|(a, _)| !ancestors[a.name.as_str()].iter().any(|b| declares(b, f).is_some()));
            if let Some((a, g)) = home {
                homes.insert((&c.name, &f.name), (&a.name, &g.name));
            }
        }
    }
    let mut shares: HashSet<&str> = homes.values().map(|h| h.0).collect();
    let class_names: HashMap<&str, HashMap<String, String>> = m.classes.iter().map(|c| (c.name.as_str(), names_of(c.funcs.iter()))).collect();
    // Alias methods from unflattened ancestors (e.g. generic mixins / Container / Node)
    // directly onto subclasses when the subclass does not already define that member.
    let inherited_consts = |c: &Class| -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = c.consts.clone();
        let mut seen_const: HashSet<String> = c.consts.iter().map(|(n, _)| n.clone()).collect();
        let mut cq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while let Some(a) = cq.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            for (k, v) in &a.consts {
                if seen_const.insert(k.clone()) {
                    out.push((k.clone(), v.clone()));
                }
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    cq.push_back(pa);
                }
            }
        }
        out
    };
    let inherited_iter = |c: &Class| -> (Option<Ty>, bool) {
        let mut iter = c.iterates.clone();
        let mut async_iter = c.iterates_async.is_some();
        let mut iq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while (iter.is_none() || !async_iter)
            && let Some(a) = iq.pop_front()
        {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            if iter.is_none() {
                iter = a.iterates.clone();
            }
            if a.iterates_async.is_some() {
                async_iter = true;
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    iq.push_back(pa);
                }
            }
        }
        (iter, async_iter)
    };
    let inherited_elements = |c: &Class| -> Option<Ty> {
        if let Some(e) = &c.elements {
            return Some(e.clone());
        }
        let mut eq: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc = HashSet::from([c.name.as_str()]);
        while let Some(a) = eq.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            if let Some(e) = &a.elements {
                return Some(e.clone());
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    eq.push_back(pa);
                }
            }
        }
        None
    };
    let consts_of: HashMap<&str, Vec<(String, String)>> = m.classes.iter().map(|c| (c.name.as_str(), inherited_consts(c))).collect();
    let iters_of: HashMap<&str, (Option<Ty>, bool)> = m.classes.iter().map(|c| (c.name.as_str(), inherited_iter(c))).collect();
    let elems_of: HashMap<&str, Option<Ty>> = m.classes.iter().map(|c| (c.name.as_str(), inherited_elements(c))).collect();
    let mut extra_inherited: HashMap<&str, Vec<(&str, &str, &str)>> = HashMap::new();
    for c in &m.classes {
        let own_names = &class_names[c.name.as_str()];
        let c_consts = &consts_of[c.name.as_str()];
        let (ref c_iter, c_iter_async) = iters_of[c.name.as_str()];
        let c_elem = &elems_of[c.name.as_str()];
        let c_iter = c_iter.as_ref().or(c_elem.as_ref());
        let mut taken: HashSet<String> = ["h", "Methods", "Key", "Type"].into_iter().map(String::from).collect();
        taken.extend(c_consts.iter().map(|(n, _)| n.clone()));
        for a in ancestors[c.name.as_str()].iter().map(|a| a.name.as_str()).chain(c.ancestors.iter().map(String::as_str)) {
            if a != c.name {
                taken.insert(format!("as{a}"));
            }
        }
        if c_iter_async && !c.funcs.iter().any(|f| f.name == "iterateAsync") {
            taken.insert("iterateAsync".into());
        }
        if c_iter.is_some() && !c.funcs.iter().any(|f| f.name == "iterate") {
            taken.insert("iterate".into());
        }
        if c.js.is_some() {
            let cast_name = if own_names.values().any(|n| n == "cast") || c.consts.iter().any(|(n, _)| n == "cast") { "tryCast" } else { "cast" };
            taken.insert(cast_name.into());
        }
        taken.extend(own_names.values().cloned());

        let mut extras = Vec::new();
        let mut queue: std::collections::VecDeque<&Class> = c.ancestors.iter().filter_map(|a| class_of.get(a.as_str()).copied()).collect();
        let mut seen_anc: HashSet<&str> = HashSet::from([c.name.as_str()]);
        while let Some(a) = queue.pop_front() {
            if !seen_anc.insert(a.name.as_str()) {
                continue;
            }
            for f in a.funcs.iter().filter(|f| Class::has_self(f)) {
                let zname = class_names[a.name.as_str()][&f.name].as_str();
                if taken.insert(zname.to_string()) {
                    let (home_cls, home_fname) = homes.get(&(a.name.as_str(), f.name.as_str())).copied().unwrap_or((a.name.as_str(), f.name.as_str()));
                    let home_zname = class_names[home_cls][home_fname].as_str();
                    shares.insert(home_cls);
                    extras.push((zname, home_cls, home_zname));
                }
            }
            for pa in a.ancestors.iter().filter_map(|p| class_of.get(p.as_str()).copied()) {
                if !seen_anc.contains(pa.name.as_str()) {
                    queue.push_back(pa);
                }
            }
        }
        extra_inherited.insert(c.name.as_str(), extras);
    }
    // Every object is an opaque handle; methods live in the struct's namespace.
    for c in &m.classes {
        let extras = &extra_inherited[c.name.as_str()];
        let c_consts = &consts_of[c.name.as_str()];
        let (ref c_iter, c_iter_async) = iters_of[c.name.as_str()];
        let c_elem = &elems_of[c.name.as_str()];
        let c_iter = c_iter.as_ref().or(c_elem.as_ref());
        let refs = uses.entry(c.module.clone()).or_default();
        for f in &c.funcs {
            f.params.iter().for_each(|p| type_refs(&p.1, refs));
            type_refs(&f.ret, refs);
            if let Some((from, _)) = homes.get(&(c.name.as_str(), f.name.as_str())) {
                refs.insert(from.to_string());
            }
        }
        refs.extend(extras.iter().map(|&(_, from, _)| from.to_string()));
        refs.extend(ancestors[c.name.as_str()].iter().map(|a| a.name.clone()));
        refs.extend(c.ancestors.iter().cloned());
        c_iter.into_iter().chain(c_elem).for_each(|t| type_refs(t, refs));
        if c_iter_async {
            refs.extend(["JsAsyncIterator".to_string(), "Promise".to_string()]);
        }
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            map.values.iter().for_each(|v| type_refs(v, refs));
        }
        let (out, externs, names) = files.entry(c.module.clone()).or_default();
        names.push(c.name.clone());
        let own_names = &class_names[c.name.as_str()];
        let mut emitted: HashSet<String> = ["h", "Methods", "Key", "Type"].into_iter().map(String::from).collect();
        emitted.extend(c_consts.iter().map(|(n, _)| n.clone()));
        emitted.extend(own_names.values().cloned());

        *out += &format!("\n{}pub const {} = extern struct {{\n    h: u32,\n", c.doc.as_deref().map(|d| comment(d, "///", "", Style::Plain)).unwrap_or_default(), c.name);
        for (name, value) in c_consts {
            *out += &format!("    pub const {} = {value};\n", ident(name));
        }
        // Asynchronous iteration: `it.next()` is a promise of the next value, or of a null handle at the end.
        if c_iter_async && !c.funcs.iter().any(|f| f.name == "iterateAsync") {
            emitted.insert("iterateAsync".into());
            *out += &format!("    pub fn iterateAsync(self: {}) JsAsyncIterator {{ return JsAsyncIterator.over(self.h); }}\n", c.name);
        }
        // Iteration: `var it = list.iterate(); while (it.next()) |n| ...`.
        if let Some(elem) = c_iter
            && !c.funcs.iter().any(|f| f.name == "iterate")
        {
            emitted.insert("iterate".into());
            *out += &format!("    pub fn iterate(self: {}) JsIteratorOf({}) {{ return JsIteratorOf({1}).over(self.h); }}\n", c.name, typed(elem, false));
        }
        // A map's keys, and the type each selects, known at compile time.
        if let Some(map) = m.maps.iter().find(|k| k.name == c.name) {
            let keys: Vec<String> = map.keys.iter().map(|k| ident(k)).collect();
            *out += &format!("    pub const Key = enum(i32) {{ {} }};\n", keys.join(", "));
            let arms: Vec<String> = keys.iter().zip(&map.values).map(|(k, v)| format!(".{k} => {}", typed(v, false))).collect();
            *out += &format!("    pub fn Type(comptime key: Key) type {{ return switch (key) {{ {} }}; }}\n", arms.join(", "));
        }
        let mut seen_as: Vec<String> = Vec::new();
        for a in ancestors[c.name.as_str()].iter().map(|a| a.name.clone()).chain(c.ancestors.iter().cloned()) {
            if a != c.name && !seen_as.contains(&a) {
                seen_as.push(a.clone());
                let as_name = format!("as{a}");
                emitted.insert(as_name.clone());
                *out += &format!("    pub fn {}(self: {}) {a} {{ return .{{ .h = self.h }}; }}\n", ident(&as_name), c.name);
            }
        }
        // Checked downcast: only a class has a constructor to test against.
        if c.js.is_some() {
            let cast_name = if own_names.values().any(|n| n == "cast") || c.consts.iter().any(|(n, _)| n == "cast") { "tryCast" } else { "cast" };
            emitted.insert(cast_name.into());
            emitted_imports.insert(format!("js_is__{}", c.name));
            *externs += &format!("    extern \"{wasm_module}\" fn @\"js_is__{}\"(u32) i32;\n", c.name);
            *out += &format!("    pub fn {}(v_: anytype) ?{} {{ return if (v_.h != 0 and raw.@\"js_is__{1}\"(v_.h) != 0) .{{ .h = v_.h }} else null; }}\n", ident(cast_name), c.name);
        }
        let inherited = |f: &Func| if Class::has_self(f) { homes.get(&(c.name.as_str(), f.name.as_str())).copied() } else { None };
        let shared = |f: &Func| shares.contains(c.name.as_str()) && Class::has_self(f) && inherited(f).is_none();
        if c.funcs.iter().any(&shared) {
            *out += "    pub fn Methods(comptime Self: type) type {\n        return struct {\n";
            for f in c.funcs.iter().filter(|f| shared(f)) {
                func(out, externs, Some("Self"), f, &own_names[&f.name], "            ", &m.unions, &m.callbacks, wasm_module, &mut emitted_imports);
            }
            *out += "        };\n    }\n";
        }
        for f in &c.funcs {
            let own_name = &own_names[&f.name];
            match inherited(f) {
                Some((from, name)) => *out += &format!("    pub const {} = {from}.Methods(@This()).{};\n", ident(own_name), ident(&class_names[from][name])),
                None if shared(f) => *out += &format!("    pub const {0} = Methods(@This()).{0};\n", ident(own_name)),
                None => func(out, externs, Some(&c.name), f, own_name, "    ", &m.unions, &m.callbacks, wasm_module, &mut emitted_imports),
            }
        }
        for &(zname, from, home_zname) in extras {
            emitted.insert(zname.to_string());
            *out += &format!("    pub const {} = {from}.Methods(@This()).{};\n", ident(zname), ident(home_zname));
        }
        if let Some(elem) = c_elem {
            let elem_ty = typed(elem, false);
            if emitted.insert("asArray".into()) {
                *out += &format!("    pub fn asArray(self: @This()) JsArrayOf({elem_ty}) {{ return .{{ .h = self.h }}; }}\n");
            }
            if emitted.insert("getLength".into()) {
                *out += &format!("    pub fn getLength(self: @This()) f64 {{ return @floatFromInt(JsArrayOf({elem_ty}).len(.{{ .h = self.h }})); }}\n");
            }
            if emitted.insert("getAt".into()) {
                *out += &format!("    pub fn getAt(self: @This(), index_: f64) ?{elem_ty} {{ if (index_ < 0) return null; return JsArrayOf(?{elem_ty}).get(.{{ .h = self.h }}, @intFromFloat(index_)); }}\n");
            }
        }
        if c.funcs.iter().any(|f| matches!(f.call, Call::Create(_))) {
            *out += "    pub const _has_field_init = true;\n";
            let init_name = if emitted.contains("init") { "fromFields" } else { "init" };
            if emitted.insert(init_name.into()) {
                *out += &format!("    pub fn {init_name}(fields_: anytype) @This() {{ return Lent.init(@This(), fields_); }}\n");
            }
        }
        if emitted.insert("deinit".into()) {
            *out += "    pub fn deinit(self: @This()) void { jsRelease(self); }\n";
        }
        if emitted.insert("retain".into()) {
            *out += "    pub fn retain(self: @This()) @This() { return jsRetain(self); }\n";
        }
        if emitted.insert("as".into()) {
            *out += "    pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }\n";
        }
        *out += "};\n";
    }
    let top_funcs: Vec<&Func> = m.funcs.iter().filter(|f| !cb_apply_imports.contains(&f.import)).collect();
    let top_names = names_of(top_funcs.iter().copied());
    for f in top_funcs {
        let refs = uses.entry(f.module.clone()).or_default();
        f.params.iter().for_each(|p| type_refs(&p.1, refs));
        type_refs(&f.ret, refs);
        let (out, externs, names) = files.entry(f.module.clone()).or_default();
        let zig_name = &top_names[&f.name];
        names.push(zig_name.clone());
        func(out, externs, None, f, zig_name, "", &m.unions, &m.callbacks, wasm_module, &mut emitted_imports);
    }

    // Which file declares each name.
    let home: HashMap<&str, &str> = files.iter().flat_map(|(file, (_, _, names))| names.iter().map(move |n| (n.as_str(), file.as_str()))).collect();
    let mut out = Vec::new();
    let mut root = format!("// Generated. Every name, from the file that declares it.\n// package: {pkg_name}\n");
    // A namespace's types and functions: their path, and where each is declared.
    let mut namespaces: Vec<(Vec<String>, String)> = Vec::new();
    // A namespace's function by its own name: `geometry.distance`.
    let func_paths: HashMap<&String, Vec<String>> = m
        .funcs
        .iter()
        .filter(|f| !cb_apply_imports.contains(&f.import))
        .filter_map(|f| m.func_namespaces.get(&f.import).map(|(path, own, _, _)| (&top_names[&f.name], path.iter().cloned().chain([zig_func_name(own, f)]).collect())))
        .collect();
    for (file, (body, externs, names)) in &files {
        let own: HashSet<&str> = names.iter().map(String::as_str).collect();
        // The runtime's names, and the types the file's declarations name.
        let wanted = RUNTIME_NAMES.iter().copied().chain(uses.get(file).into_iter().flatten().map(String::as_str));
        let used: BTreeSet<&str> = wanted.filter(|n| !own.contains(n) && home.contains_key(n)).collect();
        // Paths are relative to the importing file: `areas/css.zig` imports `../types.zig`.
        let up = "../".repeat(file.matches('/').count());
        let imports: String = used.iter().map(|n| format!("const {n} = @import(\"{up}{}.zig\").{n};\n", home[n])).collect();
        let body = if file == "runtime" { body.clone() } else { format!("// Generated.\n{imports}\n{body}") };
        out.push((format!("{file}.zig"), format!("{body}\nconst raw = struct {{\n{externs}}};\n")));
        for n in names {
            // A namespace's type is its namespace's, not the root's.
            match m.namespaced.get(n).or_else(|| func_paths.get(n)) {
                Some(path) => namespaces.push((path.clone(), format!("@import(\"{file}.zig\").{}", ident(n)))),
                None => root += &format!("pub const {} = @import(\"{file}.zig\").{};\n", ident(n), ident(n)),
            }
        }
    }
    root += &namespace_structs(&namespaces, &root);
    out.push(("bindings.zig".into(), root));
    let dep_imports = if used_refs.is_empty() {
        "    _ = mod;\n".to_string()
    } else {
        used_refs
            .iter()
            .map(|&idx| {
                format!(
                    "    mod.addImport(\"{}\", b.createModule(.{{\n        .root_source_file = b.path(\"{}/bindings.zig\"),\n    }}));\n",
                    refs[idx].pkg_name, refs[idx].dep_path
                )
            })
            .collect()
    };
    let build_zig = format!(
        "// Generated.\nconst std = @import(\"std\");\n\npub fn build(b: *std.Build) void {{\n    const mod = b.addModule(\"{pkg_name}\", .{{\n        .root_source_file = b.path(\"bindings.zig\"),\n    }});\n{dep_imports}}}\n"
    );
    out.push(("build.zig".into(), build_zig));
    let mut meta_types: Vec<String> = m
        .enums
        .iter()
        .map(|(n, _)| n.as_str())
        .chain(m.num_enums.iter().map(|(n, _)| n.as_str()))
        .chain(m.unions.iter().map(|u| u.name.as_str()))
        .chain(m.callbacks.iter().map(|c| c.name.as_str()))
        .chain(m.classes.iter().map(|c| c.name.as_str()))
        .chain(resolved_opaque.keys().map(String::as_str))
        .chain(iterates_async.then_some("JsAsyncIterator"))
        .filter(|n| !m.namespaced.contains_key(*n) && !func_paths.contains_key(&n.to_string()))
        .filter(|n| !matches!(*n, "Promise" | "Error" | "Date") && super::super::model::buffer_spec(n).is_none())
        .map(str::to_string)
        .collect();
    meta_types.sort();
    meta_types.dedup();
    let meta = super::super::pkg_meta::PkgMeta {
        target: "zig".into(),
        package: pkg_name.to_string(),
        entry: Some("bindings.zig".into()),
        types: meta_types,
        ..Default::default()
    };
    out.push((super::super::pkg_meta::META_FILE.into(), meta.to_toml()));
    Ok((out, emitted_imports))
}

// A namespace as a struct of its types, by their own names: `Stream`'s
// `PipeOptions` is `stream.PipeOptions`. Each refers to its definition
// through its file, never by a bare name another scope could also mean.
fn namespace_structs(types: &[(Vec<String>, String)], root: &str) -> String {
    let taken: HashSet<&str> = root.lines().filter_map(|l| l.strip_prefix("pub const ")?.split(' ').next()).collect();
    fn level(types: &[(Vec<String>, String)], at: &[String], depth: usize, taken: &HashSet<&str>) -> String {
        let indent = "    ".repeat(depth);
        let mut out = String::new();
        // Its own, once each: a function and a type of one name (`function Point`
        // beside `interface Point`) are apart.
        let mut own: HashSet<String> = HashSet::new();
        for (path, target) in types.iter().filter(|(p, _)| p.len() == at.len() + 1 && p[..at.len()] == *at) {
            let base = path.last().unwrap().clone();
            let mut name = base.clone();
            let mut i = 2;
            while !own.insert(name.clone()) {
                name = format!("{base}_{i}");
                i += 1;
            }
            out += &format!("{indent}pub const {} = {target};\n", ident(&name));
        }
        let children: BTreeSet<&String> = types.iter().filter(|(p, _)| p.len() > at.len() + 1 && p[..at.len()] == *at).map(|(p, _)| &p[at.len()]).collect();
        for child in children {
            // Apart from what its scope has: the root's (leaflet's function
            // `icon`), a namespace's own (`a.b()` beside `a.b.c()`), or another
            // namespace of the same name but its case (leaflet's `Control` and
            // `control`).
            let base = snake_case(child);
            let free = |n: &str| !own.contains(n) && !(depth == 0 && taken.contains(ident(n).as_str()));
            let mut name = if free(&base) { base.clone() } else { format!("{base}_ns") };
            let mut i = 2;
            while !free(&name) {
                name = format!("{base}_ns{i}");
                i += 1;
            }
            own.insert(name.clone());
            let name = ident(&name);
            let mut path = at.to_vec();
            path.push(child.clone());
            out += &format!("{indent}pub const {name} = struct {{\n{}{indent}}};\n", level(types, &path, depth + 1, taken));
        }
        out
    }
    level(types, &[], 0, &taken)
}

// `NodeJS` -> `node_js`, `Stream` -> `stream`: a namespace's name.
fn snake_case(name: &str) -> String {
    super::super::case::snake_case_after_digit(name)
}
