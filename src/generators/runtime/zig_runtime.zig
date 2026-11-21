// Generated.
const std = @import("std");

// Handles to JavaScript values. Zero is null.
pub const JsValue = extern struct {
    h: u32,
    pub const _is_js_value = true;
    pub fn number(x: f64) JsValue { return .{ .h = raw.js_from_number(x) }; }
    pub fn boolean(x: bool) JsValue { return .{ .h = raw.js_from_bool(@intFromBool(x)) }; }
    pub fn string(s: []const u8) JsValue { return .{ .h = raw.js_from_string(s.ptr, s.len) }; }
    pub fn object() JsValue { return .{ .h = raw.js_object_new() }; }
    pub fn obj(fields: anytype) JsValue {
        const info = @typeInfo(@TypeOf(fields));
        if (info != .@"struct" or info.@"struct".layout != .auto or (info.@"struct".is_tuple and info.@"struct".field_names.len != 0)) {
            @compileError("expected struct literal for JsValue.obj");
        }
        const o = object();
        inline for (info.@"struct".field_names) |fname| o.set(fname, @field(fields, fname));
        return o;
    }
    pub fn get(self: JsValue, key: []const u8) JsValue { return .{ .h = raw.js_object_get(self.h, key.ptr, key.len) }; }
    pub fn set(self: JsValue, key: []const u8, v: anytype) void { const l = Lent.of(v); defer l.done(); raw.js_object_set(self.h, key.ptr, key.len, l.h); }
    pub fn has(self: JsValue, key: []const u8) bool { return raw.js_object_has(self.h, key.ptr, key.len) != 0; }
    pub fn toNumber(self: JsValue) f64 { return raw.js_to_number(self.h); }
    pub fn toBool(self: JsValue) bool { return raw.js_to_bool(self.h) != 0; }
    pub fn toString(self: JsValue) JsString { return .{ .h = self.h }; }
    pub fn toArray(self: JsValue) JsArray { return .{ .h = self.h }; }
    pub fn toArrayOf(self: JsValue, comptime T_: type) JsArrayOf(T_) { return .{ .h = self.h }; }
    pub fn eql(self: JsValue, other: JsValue) bool { return raw.js_equals(self.h, other.h) != 0; }
    pub fn isNull(self: JsValue) bool { return self.h == 0; }
    // A number or flag from a handle a function returned, which is released.
    pub fn takeNumber(h: u32) f64 { const x = raw.js_to_number(h); raw.js_release(h); return x; }
    pub fn takeBool(h: u32) bool { const x = raw.js_to_bool(h) != 0; raw.js_release(h); return x; }
    pub fn deinit(self: JsValue) void { jsRelease(self); }
    pub fn retain(self: JsValue) JsValue { return jsRetain(self); }
    pub fn as(self: JsValue, comptime Target_: type) Target_ { return .{ .h = self.h }; }
    // Any value a union member can hold.
    pub fn of(v: anytype) JsValue {
        const T_ = @TypeOf(v);
        if (T_ == f64 or T_ == f32 or T_ == comptime_float or T_ == comptime_int) return number(v);
        if (@typeInfo(T_) == .int) return number(@floatFromInt(v));
        if (T_ == bool) return boolean(v);
        if (@typeInfo(T_) == .@"enum") return .{ .h = if (comptime Lent.hasDecl(T_, "toJsHandle")) v.toJsHandle() else raw.js_from_number(@floatFromInt(@intFromEnum(v))) };
        if (comptime Lent.isStr(T_)) return string(v);
        return .{ .h = v.h };
    }
};
pub const JsString = extern struct {
    h: u32,
    pub const _is_js_string = true;
    pub fn from(s: []const u8) JsString { return .{ .h = raw.js_from_string(s.ptr, s.len) }; }
    pub fn len(self: JsString) usize { return raw.js_string_len(self.h); }
    pub fn read(self: JsString, buf: []u8) []u8 { const n = raw.js_string_copy(self.h, buf.ptr, buf.len); if (n > buf.len) @trap(); return buf[0..@min(n, buf.len)]; }
    // As much as fits in buf, releasing this JsString handle.
    pub fn take(self: JsString, buf: []u8) []u8 { const n = raw.js_string_take(self.h, buf.ptr, buf.len); if (n > buf.len) raw.js_release(self.h); return buf[0..@min(n, buf.len)]; }
    // As much as fits in buf.
    pub fn copy(self: JsString, buf: []u8) []u8 { const n = raw.js_string_copy(self.h, buf.ptr, buf.len); return buf[0..@min(n, buf.len)]; }
    // A copy the caller owns.
    pub fn toOwned(self: JsString, allocator: anytype) ![]u8 { const buf = try allocator.alloc(u8, self.len()); return self.copy(buf); }
    // Compares in JavaScript, without copying.
    pub fn eql(self: JsString, s: []const u8) bool { return raw.js_string_equals(self.h, s.ptr, s.len) != 0; }
    pub fn format(self: JsString, writer: *std.Io.Writer) std.Io.Writer.Error!void {
        const n = self.len();
        if (n == 0) return;
        if (writer.buffer.len - writer.end >= n) {
            _ = raw.js_string_copy(self.h, writer.buffer[writer.end..].ptr, n);
            writer.advance(n);
            return;
        }
        var buf: [4096]u8 = undefined;
        const got = raw.js_string_copy(self.h, &buf, buf.len);
        try writer.writeAll(buf[0..@min(got, buf.len)]);
    }
    pub fn deinit(self: JsString) void { jsRelease(self); }
    pub fn retain(self: JsString) JsString { return jsRetain(self); }
    pub fn as(self: JsString, comptime Target_: type) Target_ { return .{ .h = self.h }; }
};
// A callback taking one object: `JsListener(PointerEvent).from(onClick, &state)` or `fromFn(onClick)`.
pub fn JsListener(comptime Ev_: type) type {
    return extern struct {
        h: u32,
        pub const _listener_event = Ev_;
        pub const Fn = fn (?*anyopaque, Ev_) void;
        pub fn from(comptime f_: anytype, ctx_: anytype) @This() {
            const info_ = @typeInfo(@TypeOf(f_)).@"fn";
            if (info_.param_types.len == 1) {
                const T_ = struct {
                    fn call(_: ?*anyopaque, ev_: u32) callconv(.c) void { f_(.{ .h = ev_ }); }
                };
                return .{ .h = raw.js_listener_from(@intFromPtr(&T_.call), null) };
            }
            const Ptr_ = info_.param_types[0].?;
            if (Ptr_ == ?*anyopaque) {
                const T_ = struct {
                    fn call(c_: ?*anyopaque, ev_: u32) callconv(.c) void { f_(c_, .{ .h = ev_ }); }
                };
                return .{ .h = raw.js_listener_from(@intFromPtr(&T_.call), Lent.toRawCtx(ctx_)) };
            }
            const typed_ctx_: Ptr_ = ctx_;
            const raw_ctx_ = Lent.toRawCtx(typed_ctx_);
            const T_ = struct {
                fn call(c_: ?*anyopaque, ev_: u32) callconv(.c) void {
                    if (@typeInfo(Ptr_) == .optional) {
                        f_(if (c_) |p_| @ptrCast(@alignCast(p_)) else null, .{ .h = ev_ });
                    } else {
                        f_(@ptrCast(@alignCast(c_.?)), .{ .h = ev_ });
                    }
                }
            };
            return .{ .h = raw.js_listener_from(@intFromPtr(&T_.call), raw_ctx_) };
        }
        pub fn fromFn(comptime f_: anytype) @This() { return from(f_, null); }
        pub fn deinit(self: @This()) void { jsRelease(self); }
        pub fn retain(self: @This()) @This() { return jsRetain(self); }
        pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }
    };
}
// A handle a function returns is the caller's: `defer x.deinit()` or `defer jsRelease(x)`.
// Arguments, and a callback's arguments, are lent: `x.retain()` / `jsRetain(x)` to keep one.
// What a callback returns is given to JavaScript: `x.retain()` to return it and keep it.
pub fn jsRelease(x: anytype) void { raw.js_release(x.h); }
pub fn jsRetain(x: anytype) @TypeOf(x) { raw.js_retain(x.h); return x; }
// Whether two handles are one JavaScript value.
pub fn jsSame(a: anytype, b: anytype) bool { return a.h == b.h or raw.js_equals(a.h, b.h) != 0; }
// What a call returned, or error.JsException if it threw; the exception is
// then `jsTakeError()`: `const el = try b.jsTry(doc.querySelector(text));`.
pub fn jsHasError() bool { return raw.js_has_error() != 0; }
pub fn jsTry(value: anytype) error{JsException}!@TypeOf(value) { return if (raw.js_has_error() != 0) error.JsException else value; }
// The exception a call threw, taken. A call that throws returns a zero value,
// and an exception not taken by the time control returns to JavaScript is
// reported there.
pub fn jsTakeError() ?JsValue { const e = raw.js_take_error(); return if (e == 0) null else .{ .h = e }; }
// An exception's message, name or stack, the caller's to release.
pub fn jsErrorMessage(e: JsValue) JsString { return .{ .h = raw.js_error_message(e.h) }; }
pub fn jsErrorName(e: JsValue) JsString { return .{ .h = raw.js_error_name(e.h) }; }
pub fn jsErrorStack(e: JsValue) JsString { return .{ .h = raw.js_error_stack(e.h) }; }
pub fn jsErrorNew(msg: []const u8) JsValue { return .{ .h = raw.js_error_new(msg.ptr, msg.len) }; }
pub fn jsDateNow() f64 { return raw.js_date_now(); }
// A scope: every handle a function returns while it is the innermost open
// scope is released when it closes. `jsRetain` one to keep it.
pub const JsScope = struct {
    pub fn open() JsScope { raw.js_scope_open(); return .{}; }
    pub fn close(_: JsScope) void { raw.js_scope_close(); }
    pub fn deinit(_: JsScope) void { raw.js_scope_close(); }
};
// How many JavaScript values wasm holds handles to.
pub fn jsLiveHandles() u32 { return raw.js_live(); }
// Forgets a callback made with `from`, and returns its context. Its handle is released as any other.
pub fn jsForgetCallback(x: anytype) ?*anyopaque { return @ptrFromInt(raw.js_callback_release(x.h)); }

// A value lent to JavaScript for one call: an object's own handle, or a
// temporary value created for a coercion and released by `done`.
pub const Lent = struct {
    h: u32,
    own: bool,
    pub const none: Lent = .{ .h = 0, .own = false };

    fn hasDecl(comptime T_: type, comptime decl_name: []const u8) bool {
        return switch (@typeInfo(T_)) {
            .@"struct", .@"enum", .@"union", .@"opaque" => @hasDecl(T_, decl_name),
            else => false,
        };
    }

    fn isStr(comptime T_: type) bool {
        return switch (@typeInfo(T_)) {
            .pointer => |p| switch (p.size) {
                .slice => p.child == u8,
                .one => @typeInfo(p.child) == .array and @typeInfo(p.child).array.child == u8,
                else => false,
            },
            else => false,
        };
    }

    fn isRawHandle(comptime T_: type) bool {
        const info = @typeInfo(T_);
        return info == .@"struct" and info.@"struct".layout == .auto and !info.@"struct".is_tuple and info.@"struct".field_names.len == 1 and std.mem.eql(u8, info.@"struct".field_names[0], "h") and (info.@"struct".field_types[0] == u32 or info.@"struct".field_types[0] == comptime_int);
    }

    fn isHandle(comptime T_: type) bool {
        const info = @typeInfo(T_);
        return info == .@"struct" and !info.@"struct".is_tuple and info.@"struct".field_names.len == 1 and std.mem.eql(u8, info.@"struct".field_names[0], "h") and (info.@"struct".field_types[0] == u32 or info.@"struct".field_types[0] == comptime_int);
    }

    fn isSeq(comptime T_: type) bool {
        if (isStr(T_)) return false;
        return switch (@typeInfo(T_)) {
            .@"struct" => |s| s.is_tuple,
            .array => |a| a.child != u8,
            .pointer => |p| switch (p.size) {
                .slice => p.child != u8,
                .one => switch (@typeInfo(p.child)) {
                    .@"struct" => |s| s.is_tuple,
                    .array => |a| a.child != u8,
                    else => false,
                },
                else => false,
            },
            else => false,
        };
    }

    fn shortTypeName(comptime full: []const u8) []const u8 {
        var start: usize = 0;
        for (full, 0..) |ch, i| {
            if (ch == '.') start = i + 1;
        }
        const out = full[start..].*;
        return &out;
    }

    fn setterName(comptime field: []const u8) []const u8 {
        var buf: [field.len + 3]u8 = undefined;
        buf[0..3].* = "set".*;
        var n: usize = 3;
        var cap = true;
        for (field) |ch| {
            if (ch == '_') {
                if (n == 3) {
                    buf[n] = '_';
                    n += 1;
                }
                cap = true;
            } else {
                buf[n] = if (cap and ch >= 'a' and ch <= 'z') ch - 32 else ch;
                n += 1;
                cap = false;
            }
        }
        const out = buf[0..n].*;
        return &out;
    }

    fn enumHasTag(comptime E_: type, comptime tag: []const u8) bool {
        inline for (@typeInfo(E_).@"enum".field_names) |fname| {
            if (comptime std.mem.eql(u8, fname, tag)) return true;
        }
        return false;
    }

    fn isTwoTupleFn(comptime T_: type) bool {
        const info = @typeInfo(T_);
        return info == .@"struct" and info.@"struct".is_tuple and info.@"struct".field_names.len == 2 and @typeInfo(info.@"struct".field_types[0]) == .@"fn";
    }

    fn isCallbackTy(comptime T_: type) bool {
        return hasDecl(T_, "fromFn") or hasDecl(T_, "_listener_event");
    }

    pub fn toRawCtx(ctx: anytype) ?*anyopaque {
        const C_ = @TypeOf(ctx);
        if (C_ == @TypeOf(null) or C_ == void) return null;
        if (C_ == ?*anyopaque) return ctx;
        const info = @typeInfo(C_);
        if (info == .optional) {
            const pinfo = @typeInfo(info.optional.child);
            if (pinfo == .pointer and pinfo.pointer.size == .one) {
                return if (ctx) |p| if (pinfo.pointer.attrs.@"const") @as(*anyopaque, @ptrCast(@constCast(p))) else @as(*anyopaque, @ptrCast(p)) else null;
            }
        }
        if (info == .pointer and info.pointer.size == .one) {
            return if (info.pointer.attrs.@"const") @as(*anyopaque, @ptrCast(@constCast(ctx))) else @as(*anyopaque, @ptrCast(ctx));
        }
        @compileError("expected null, {}, or pointer as callback context, got " ++ @typeName(C_));
    }

    fn hasAllFields(comptime Self_: type, comptime F_: type) bool {
        const finfo = @typeInfo(F_);
        if (finfo != .@"struct" or finfo.@"struct".layout != .auto or (finfo.@"struct".is_tuple and finfo.@"struct".field_names.len != 0)) return false;
        if (hasDecl(Self_, "setAt")) return true;
        inline for (finfo.@"struct".field_names) |fname| {
            if (!hasDecl(Self_, setterName(fname))) return false;
        }
        return true;
    }

    fn canUpcast(comptime From_: type, comptime Base_: type) bool {
        if (@typeInfo(From_) != .@"struct" or @typeInfo(Base_) != .@"struct") return false;
        const decl = "as" ++ shortTypeName(@typeName(Base_));
        if (!@hasDecl(From_, decl)) return false;
        const F_ = @TypeOf(@field(From_, decl));
        if (@typeInfo(F_) != .@"fn") return false;
        return @typeInfo(F_).@"fn".param_types.len == 1 and @typeInfo(F_).@"fn".return_type == Base_;
    }

    fn shouldUnwrapSingle(comptime InnerTy_: type, comptime fname: []const u8, comptime ValTy_: type) bool {
        if (hasDecl(InnerTy_, "_has_field_init") and hasDecl(InnerTy_, setterName(fname))) {
            if (ValTy_ == InnerTy_ or ValTy_ == ?InnerTy_ or ValTy_ == @TypeOf(null) or isRawHandle(ValTy_) or canUpcast(ValTy_, InnerTy_)) {
                return true;
            }
            return false;
        }
        return true;
    }

    fn SingleOptType(comptime InnerTy_: type, comptime fname: []const u8, comptime Opts_: type) type {
        if (Opts_ == @TypeOf(null)) return @TypeOf(null);
        const info = @typeInfo(Opts_);
        if (info == .@"struct" and info.@"struct".layout == .auto) {
            if (info.@"struct".field_names.len == 0) return @TypeOf(null);
            if (!info.@"struct".is_tuple and info.@"struct".field_names.len == 1 and std.mem.eql(u8, info.@"struct".field_names[0], fname)) {
                const V_ = info.@"struct".field_types[0];
                if (shouldUnwrapSingle(InnerTy_, fname, V_)) return V_;
            }
        }
        return Opts_;
    }

    pub fn hasOpt(comptime Opts_: type, comptime fname: []const u8) bool {
        if (Opts_ == @TypeOf(null)) return false;
        const info = @typeInfo(Opts_);
        return info == .@"struct" and !info.@"struct".is_tuple and @hasField(Opts_, fname);
    }

    pub fn of(v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == f64 or T_ == f32 or T_ == comptime_float or T_ == comptime_int) return .{ .h = raw.js_from_number(v), .own = true };
        if (@typeInfo(T_) == .int) return .{ .h = raw.js_from_number(@floatFromInt(v)), .own = true };
        if (T_ == bool) return .{ .h = raw.js_from_bool(@intFromBool(v)), .own = true };
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| of(x) else .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .@"enum") return .{ .h = if (comptime hasDecl(T_, "toJsHandle")) v.toJsHandle() else raw.js_from_number(@floatFromInt(@intFromEnum(v))), .own = true };
        if (comptime isStr(T_)) { const s: []const u8 = v; return .{ .h = raw.js_from_string(s.ptr, s.len), .own = true }; }
        if (@typeInfo(T_) == .@"union") return switch (v) { inline else => |x| of(x) };
        if (comptime isHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (comptime isSeq(T_)) return .{ .h = JsArray.of(v).h, .own = true };
        const info = @typeInfo(T_);
        if (info == .@"struct" and !info.@"struct".is_tuple and info.@"struct".layout == .auto) {
            return .{ .h = JsValue.obj(v).h, .own = true };
        }
        @compileError("cannot pass " ++ @typeName(T_) ++ " to JavaScript");
    }

    pub fn init(comptime Self_: type, fields: anytype) Self_ {
        const info = @typeInfo(@TypeOf(fields));
        if (info != .@"struct" or info.@"struct".layout != .auto or (info.@"struct".is_tuple and info.@"struct".field_names.len != 0)) {
            @compileError("expected struct literal for " ++ @typeName(Self_));
        }
        const res = Self_.create();
        inline for (info.@"struct".field_names) |fname| {
            const setter = comptime setterName(fname);
            if (@hasDecl(Self_, setter)) {
                @field(Self_, setter)(res, @field(fields, fname));
            } else if (@hasDecl(Self_, "setAt")) {
                res.setAt(fname, @field(fields, fname));
            } else {
                @compileError(@typeName(Self_) ++ " has no field '" ++ fname ++ "'");
            }
        }
        return res;
    }

    fn callInit(comptime T_: type, fields: anytype) T_ {
        return if (@hasDecl(T_, "init") and !@hasDecl(T_, "fromFields")) T_.init(fields) else T_.fromFields(fields);
    }

    pub fn obj(comptime Base_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == Base_) return .{ .h = v.h, .own = false };
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (comptime canUpcast(T_, Base_)) return .{ .h = v.h, .own = false };
        if (comptime (hasDecl(Base_, "_is_js_string") and (isStr(T_) or hasDecl(T_, "_is_js_string")))) return of(v);
        if (comptime hasDecl(Base_, "_is_js_value")) return of(v);
        if (comptime (hasDecl(Base_, "_is_js_array") and (isSeq(T_) or hasDecl(T_, "_is_js_array")))) return array(v);
        if (comptime (hasDecl(Base_, "fromFn") and @typeInfo(T_) == .@"fn")) return .{ .h = Base_.fromFn(v).h, .own = true };
        if (comptime (hasDecl(Base_, "from") and isTwoTupleFn(T_))) return .{ .h = Base_.from(v[0], v[1]).h, .own = true };
        if (comptime hasDecl(Base_, "_has_field_init")) {
            const info = @typeInfo(T_);
            if (info == .@"struct" and info.@"struct".layout == .auto and (!info.@"struct".is_tuple or info.@"struct".field_names.len == 0)) {
                return .{ .h = callInit(Base_, v).h, .own = true };
            }
        }
        @compileError("expected " ++ @typeName(Base_) ++ ", got " ++ @typeName(T_));
    }

    pub fn objOpt(comptime Base_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| obj(Base_, x) else .{ .h = 0, .own = false };
        return obj(Base_, v);
    }

    pub fn array(v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (comptime (T_ == JsArray or hasDecl(T_, "_is_js_array") or hasDecl(T_, "_elem_type") or hasDecl(T_, "_tuple_types"))) return .{ .h = v.h, .own = false };
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (comptime isSeq(T_)) return .{ .h = JsArray.of(v).h, .own = true };
        @compileError("expected JsArray or sequence, got " ++ @typeName(T_));
    }

    pub fn arrayOpt(v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| array(x) else .{ .h = 0, .own = false };
        return array(v);
    }

    pub fn arrayOf(comptime E_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (comptime (T_ == JsArrayOf(E_) or T_ == JsArray or hasDecl(T_, "_elem_type") or hasDecl(T_, "_is_js_array"))) return .{ .h = v.h, .own = false };
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (comptime isSeq(T_)) return .{ .h = JsArrayOf(E_).of(v).h, .own = true };
        @compileError("expected array of " ++ @typeName(E_) ++ ", got " ++ @typeName(T_));
    }

    pub fn arrayOfOpt(comptime E_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| arrayOf(E_, x) else .{ .h = 0, .own = false };
        return arrayOf(E_, v);
    }

    pub fn tuple(comptime Types_: []const type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (comptime (T_ == JsTuple(Types_) or T_ == JsArray or hasDecl(T_, "_tuple_types") or hasDecl(T_, "_is_js_array"))) return .{ .h = v.h, .own = false };
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (comptime isSeq(T_)) return .{ .h = JsTuple(Types_).of(v).h, .own = true };
        @compileError("expected tuple, got " ++ @typeName(T_));
    }

    pub fn tupleOpt(comptime Types_: []const type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| tuple(Types_, x) else .{ .h = 0, .own = false };
        return tuple(Types_, v);
    }

    pub fn listener(comptime L_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == L_) return .{ .h = v.h, .own = false };
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        if (@typeInfo(T_) == .@"fn") return .{ .h = L_.fromFn(v).h, .own = true };
        if (comptime isTwoTupleFn(T_)) return .{ .h = L_.from(v[0], v[1]).h, .own = true };
        if (comptime isHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        @compileError("expected " ++ @typeName(L_) ++ ", got " ++ @typeName(T_));
    }

    pub fn listenerOpt(comptime L_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| listener(L_, x) else .{ .h = 0, .own = false };
        return listener(L_, v);
    }

    pub fn value(comptime Target_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (Target_ == []const u8) {
            if (comptime (isStr(T_) or hasDecl(T_, "_is_js_string"))) return of(v);
            @compileError("expected string, got " ++ @typeName(T_));
        }
        if (Target_ == f64) {
            if (T_ == f64 or T_ == f32 or T_ == comptime_float or T_ == comptime_int or @typeInfo(T_) == .int) return of(v);
            @compileError("expected number, got " ++ @typeName(T_));
        }
        if (Target_ == bool) {
            if (T_ == bool) return of(v);
            @compileError("expected bool, got " ++ @typeName(T_));
        }
        if (@typeInfo(Target_) == .optional) {
            if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
            if (@typeInfo(T_) == .optional) return if (v) |x| value(@typeInfo(Target_).optional.child, x) else .{ .h = 0, .own = false };
            return value(@typeInfo(Target_).optional.child, v);
        }
        if (@typeInfo(Target_) == .@"enum") {
            if (T_ == Target_) return of(v);
            if (@typeInfo(T_) == .enum_literal) return of(@as(Target_, v));
            @compileError("expected enum " ++ @typeName(Target_) ++ ", got " ++ @typeName(T_));
        }
        if (@typeInfo(Target_) == .@"union") return unionVal(Target_, v);
        if (comptime hasDecl(Target_, "_elem_type")) return arrayOf(Target_._elem_type, v);
        if (comptime hasDecl(Target_, "_tuple_types")) return tuple(Target_._tuple_types, v);
        if (comptime hasDecl(Target_, "_listener_event")) return listener(Target_, v);
        if (comptime hasDecl(Target_, "_is_js_array")) return array(v);
        if (comptime hasDecl(Target_, "_is_js_value")) return of(v);
        return obj(Target_, v);
    }

    pub fn valueOpt(comptime Target_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| value(Target_, x) else .{ .h = 0, .own = false };
        return value(Target_, v);
    }

    pub fn unionVal(comptime U_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == U_) return of(v);
        if (comptime isRawHandle(T_)) return .{ .h = @intCast(v.h), .own = false };
        const uinfo = @typeInfo(U_).@"union";
        const tinfo = @typeInfo(T_);
        if (tinfo == .@"struct" and !tinfo.@"struct".is_tuple and tinfo.@"struct".layout == .auto and tinfo.@"struct".field_names.len == 1) {
            inline for (uinfo.field_names, uinfo.field_types) |uf_name, uf_type| {
                if (comptime std.mem.eql(u8, tinfo.@"struct".field_names[0], uf_name)) {
                    return value(uf_type, @field(v, uf_name));
                }
            }
        }
        inline for (uinfo.field_types) |uf_type| {
            if (T_ == uf_type) return of(v);
        }
        if (comptime (isStr(T_) or hasDecl(T_, "_is_js_string"))) {
            inline for (uinfo.field_types) |uf_type| {
                if (comptime (uf_type == []const u8 or hasDecl(uf_type, "_is_js_string"))) return of(v);
            }
        }
        if (T_ == f64 or T_ == f32 or T_ == comptime_float or T_ == comptime_int or tinfo == .int) {
            inline for (uinfo.field_types) |uf_type| {
                if (uf_type == f64) return of(v);
            }
        }
        if (T_ == bool) {
            inline for (uinfo.field_types) |uf_type| {
                if (uf_type == bool) return of(v);
            }
        }
        if (tinfo == .enum_literal) {
            inline for (uinfo.field_types) |uf_type| {
                if (@typeInfo(uf_type) == .@"enum") {
                    if (comptime enumHasTag(uf_type, @tagName(v))) return of(@as(uf_type, v));
                }
            }
        }
        if (tinfo == .@"struct") {
            inline for (uinfo.field_types) |uf_type| {
                if (comptime canUpcast(T_, uf_type)) return .{ .h = v.h, .own = false };
            }
        }
        if (tinfo == .@"fn" or comptime isTwoTupleFn(T_)) {
            inline for (uinfo.field_types) |uf_type| {
                if (comptime isCallbackTy(uf_type)) return obj(uf_type, v);
            }
        }
        if (comptime isSeq(T_)) {
            inline for (uinfo.field_types) |uf_type| {
                if (comptime hasDecl(uf_type, "_tuple_types")) return tuple(uf_type._tuple_types, v);
            }
            inline for (uinfo.field_types) |uf_type| {
                if (comptime hasDecl(uf_type, "_elem_type")) return arrayOf(uf_type._elem_type, v);
            }
            inline for (uinfo.field_types) |uf_type| {
                if (comptime hasDecl(uf_type, "_is_js_array")) return array(v);
            }
        }
        if (tinfo == .@"struct" and !tinfo.@"struct".is_tuple and tinfo.@"struct".layout == .auto) {
            comptime var matched: ?type = null;
            comptime var match_count: usize = 0;
            inline for (uinfo.field_types) |uf_type| {
                if (comptime (hasDecl(uf_type, "_has_field_init") and hasAllFields(uf_type, T_))) {
                    matched = uf_type;
                    match_count += 1;
                }
            }
            if (match_count == 1) return .{ .h = callInit(matched.?, v).h, .own = true };
            if (match_count > 1) @compileError("ambiguous struct literal for union " ++ @typeName(U_));
        }
        inline for (uinfo.field_types) |uf_type| {
            if (comptime hasDecl(uf_type, "_is_js_value")) return of(v);
        }
        @compileError("cannot convert " ++ @typeName(T_) ++ " to union " ++ @typeName(U_));
    }

    pub fn unionOpt(comptime U_: type, v: anytype) Lent {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return .{ .h = 0, .own = false };
        if (@typeInfo(T_) == .optional) return if (v) |x| unionVal(U_, x) else .{ .h = 0, .own = false };
        return unionVal(U_, v);
    }

    pub fn singleOpt(comptime InnerTy_: type, comptime fname: []const u8, options: anytype) SingleOptType(InnerTy_, fname, @TypeOf(options)) {
        const Opts_ = @TypeOf(options);
        if (Opts_ == @TypeOf(null)) return null;
        const info = @typeInfo(Opts_);
        if (comptime (info == .@"struct" and info.@"struct".layout == .auto)) {
            if (info.@"struct".field_names.len == 0) return null;
            if (comptime (!info.@"struct".is_tuple and info.@"struct".field_names.len == 1 and std.mem.eql(u8, info.@"struct".field_names[0], fname))) {
                const V_ = info.@"struct".field_types[0];
                if (comptime shouldUnwrapSingle(InnerTy_, fname, V_)) return @field(options, fname);
            }
        }
        return options;
    }

    pub fn checkOpts(options: anytype, comptime allowed: []const []const u8) void {
        const Opts_ = @TypeOf(options);
        if (Opts_ == @TypeOf(null)) return;
        const info = @typeInfo(Opts_);
        if (info == .@"struct" and info.@"struct".layout == .auto and info.@"struct".field_names.len == 0) return;
        if (info != .@"struct" or info.@"struct".is_tuple or info.@"struct".layout != .auto) {
            @compileError("expected options struct literal or null, got " ++ @typeName(Opts_));
        }
        inline for (info.@"struct".field_names) |fname| {
            comptime var ok = false;
            inline for (allowed) |a| {
                if (comptime std.mem.eql(u8, fname, a)) ok = true;
            }
            if (!ok) @compileError("unknown option '" ++ fname ++ "'");
        }
    }

    pub fn optNum(v: anytype) ?f64 {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return null;
        if (@typeInfo(T_) == .optional) return if (v) |x| optNum(x) else null;
        if (T_ == f64 or T_ == f32 or T_ == comptime_float or T_ == comptime_int) return v;
        if (@typeInfo(T_) == .int) return @floatFromInt(v);
        @compileError("expected number or null, got " ++ @typeName(T_));
    }

    pub fn optBool(v: anytype) ?bool {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return null;
        if (@typeInfo(T_) == .optional) return if (v) |x| optBool(x) else null;
        if (T_ == bool) return v;
        @compileError("expected bool or null, got " ++ @typeName(T_));
    }

    pub fn optStr(v: anytype) ?[]const u8 {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return null;
        if (@typeInfo(T_) == .optional) return if (v) |x| optStr(x) else null;
        if (comptime isStr(T_)) return v;
        @compileError("expected string or null, got " ++ @typeName(T_));
    }

    pub fn optEnum(comptime E_: type, v: anytype) ?E_ {
        const T_ = @TypeOf(v);
        if (T_ == @TypeOf(null)) return null;
        if (@typeInfo(T_) == .optional) return if (v) |x| optEnum(E_, x) else null;
        if (T_ == E_ or @typeInfo(T_) == .enum_literal) return v;
        @compileError("expected " ++ @typeName(E_) ++ " or null, got " ++ @typeName(T_));
    }

    pub fn done(self: Lent) void { if (self.own) raw.js_release(self.h); }
};

// An enum's value from a handle, which stays as it was.
pub fn jsEnumOf(comptime T_: type, h: u32) T_ {
    return if (@hasDecl(T_, "fromJsHandle")) T_.fromJsHandle(h) else @enumFromInt(@as(i32, @intFromFloat(raw.js_to_number(h))));
}

// A T from a handle a function returned: takes the handle over.
pub fn jsTake(comptime T_: type, h: u32) T_ {
    if (@typeInfo(T_) == .optional) return if (h == 0) null else jsTake(@typeInfo(T_).optional.child, h);
    if (@typeInfo(T_) == .@"enum") { const x = jsEnumOf(T_, h); raw.js_release(h); return x; }
    if (T_ == f64) return JsValue.takeNumber(h);
    if (T_ == bool) return JsValue.takeBool(h);
    if (@typeInfo(T_) == .@"union") return T_.fromHandle(h);
    return .{ .h = h };
}

// A JavaScript iterator yielding T. Each value is the caller's; so is the iterator.
pub fn JsIteratorOf(comptime T_: type) type {
    return extern struct {
        h: u32,
        pub fn over(h: u32) @This() { return .{ .h = raw.js_iterate(h) }; }
        // The next value, or null at the end.
        pub fn next(self: @This()) ?T_ { const v = raw.js_iterator_next(self.h); return if (v == 0xFFFFFFFF) null else jsTake(T_, v); }
        pub fn deinit(self: @This()) void { jsRelease(self); }
        pub fn retain(self: @This()) @This() { return jsRetain(self); }
        pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }
    };
}

// A JavaScript array of fixed length and element types: `[number, string]` is
// `JsTuple(&.{ f64, JsString })`. `get(0)` reads an element, the caller's.
pub fn JsTuple(comptime Types_: []const type) type {
    return extern struct {
        h: u32,
        pub const _tuple_types = Types_;
        // From a Zig tuple of values: `.{ 1, "one" }`.
        pub fn of(values: anytype) @This() {
            const a: @This() = .{ .h = raw.js_array_new() };
            const V_ = @TypeOf(values);
            const vinfo = @typeInfo(V_);
            if (vinfo == .@"struct" and vinfo.@"struct".is_tuple) {
                inline for (values, 0..) |v, i| { const l = Lent.value(Types_[i], v); defer l.done(); raw.js_array_push(a.h, l.h); }
            } else if (vinfo == .pointer and vinfo.pointer.size == .one and @typeInfo(vinfo.pointer.child) == .@"struct" and @typeInfo(vinfo.pointer.child).@"struct".is_tuple) {
                inline for (values.*, 0..) |v, i| { const l = Lent.value(Types_[i], v); defer l.done(); raw.js_array_push(a.h, l.h); }
            } else {
                for (values) |v| { const l = Lent.of(v); defer l.done(); raw.js_array_push(a.h, l.h); }
            }
            return a;
        }
        pub fn get(self: @This(), comptime i: usize) Types_[i] { return jsTake(Types_[i], raw.js_array_get(self.h, i)); }
        pub fn deinit(self: @This()) void { jsRelease(self); }
        pub fn retain(self: @This()) @This() { return jsRetain(self); }
        pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }
    };
}

// A JavaScript array of T: f64, bool, JsString, a handle, or a union.
pub fn JsArrayOf(comptime T_: type) type {
    return extern struct {
        h: u32,
        pub const _elem_type = T_;
        pub fn new() @This() { return .{ .h = raw.js_array_new() }; }
        pub fn of(items: anytype) @This() {
            const a = new();
            const I_ = @TypeOf(items);
            const iinfo = @typeInfo(I_);
            if (iinfo == .@"struct" and iinfo.@"struct".is_tuple) {
                inline for (items) |x| a.push(x);
            } else if (iinfo == .pointer and iinfo.pointer.size == .one and @typeInfo(iinfo.pointer.child) == .@"struct" and @typeInfo(iinfo.pointer.child).@"struct".is_tuple) {
                inline for (items.*) |x| a.push(x);
            } else {
                for (items) |x| a.push(x);
            }
            return a;
        }
        pub fn len(self: @This()) usize { return raw.js_array_length(self.h); }
        // What it returns is the caller's, as from any function.
        pub fn get(self: @This(), i: usize) T_ { return jsTake(T_, raw.js_array_get(self.h, i)); }
        pub fn push(self: @This(), v: anytype) void { const l = Lent.value(T_, v); defer l.done(); raw.js_array_push(self.h, l.h); }
        pub fn pushString(self: @This(), s: []const u8) void { const l = Lent.of(s); defer l.done(); raw.js_array_push(self.h, l.h); }
        // The same array, untyped.
        pub fn any(self: @This()) JsArray { return .{ .h = self.h }; }
        pub fn deinit(self: @This()) void { jsRelease(self); }
        pub fn retain(self: @This()) @This() { return jsRetain(self); }
        pub fn as(self: @This(), comptime Target_: type) Target_ { return .{ .h = self.h }; }
    };
}

pub const JsArray = extern struct {
    h: u32,
    pub const _is_js_array = true;
    pub fn new() JsArray { return .{ .h = raw.js_array_new() }; }
    pub fn of(items: anytype) JsArray {
        const a = new();
        const I_ = @TypeOf(items);
        const iinfo = @typeInfo(I_);
        if (iinfo == .@"struct" and iinfo.@"struct".is_tuple) {
            inline for (items) |x| a.push(x);
        } else if (iinfo == .pointer and iinfo.pointer.size == .one and @typeInfo(iinfo.pointer.child) == .@"struct" and @typeInfo(iinfo.pointer.child).@"struct".is_tuple) {
            inline for (items.*) |x| a.push(x);
        } else {
            for (items) |x| a.push(x);
        }
        return a;
    }
    pub fn len(self: JsArray) usize { return raw.js_array_length(self.h); }
    pub fn get(self: JsArray, i: usize) JsValue { return .{ .h = raw.js_array_get(self.h, i) }; }
    pub fn push(self: JsArray, v: anytype) void { const l = Lent.of(v); defer l.done(); raw.js_array_push(self.h, l.h); }
    pub fn deinit(self: JsArray) void { jsRelease(self); }
    pub fn retain(self: JsArray) JsArray { return jsRetain(self); }
    pub fn as(self: JsArray, comptime Target_: type) Target_ { return .{ .h = self.h }; }
};

