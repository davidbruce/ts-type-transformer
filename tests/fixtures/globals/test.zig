// Calls the generated Zig bindings; every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn strEq(s: b.JsString, expected: []const u8) bool {
    var buf: [128]u8 = undefined;
    if (s.len() > buf.len) return false;
    return std.mem.eql(u8, s.read(&buf), expected);
}

export fn run() void {
    // Global values.
    const s = b.get_settings();
    expect(s.get_level() == 2, @src());
    expect(!s.get_debug(), @src());
    s.set_debug(true);
    expect(b.get_settings().get_debug(), @src());
    expect(strEq(b.get_VERSION(), "1.2.3"), @src());

    // A global with members of its own.
    expect(b.Clock.now() == 1000, @src());
    expect(b.Clock.ZERO == 0, @src());

    // Namespaces, nested ones included.
    const p = b.Geometry_point(0, 0);
    const q = b.Geometry_point(3, 4);
    expect(b.Geometry_distance(p, q) == 5, @src());
    expect(q.get_x() == 3, @src());
    expect(b.get_Geometry_UNIT() == 1, @src());
    expect(b.Geometry_Deep_twice(4) == 8, @src());

    // Rest parameters take an array.
    const parts = b.JsArray.new();
    parts.push(b.JsValue.string("a"));
    parts.push(b.JsValue.string("b"));
    parts.push(b.JsValue.string("c"));
    expect(strEq(b.join_with_parts("-", parts), "a-b-c"), @src());
    expect(strEq(b.join("-"), ""), @src());

    // Numeric enums.
    expect(b.bump(.Low) == .High, @src());
    expect(b.bump(.High) == .High, @src());
}
