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
    const s = b.getSettings();
    expect(s.getLevel() == 2, @src());
    expect(!s.getDebug(), @src());
    s.setDebug(true);
    expect(b.getSettings().getDebug(), @src());
    expect(strEq(b.getVERSION(), "1.2.3"), @src());

    // A global with members of its own.
    expect(b.Clock.now() == 1000, @src());
    expect(b.Clock.ZERO == 0, @src());

    // Namespaces, nested ones included.
    const p = b.GeometryPoint(0, 0);
    const q = b.GeometryPoint(3, 4);
    expect(b.GeometryDistance(p, q) == 5, @src());
    expect(q.getX() == 3, @src());
    expect(b.getGeometryUNIT() == 1, @src());
    expect(b.GeometryDeepTwice(4) == 8, @src());

    // Rest parameters take an array.
    const parts = b.JsArrayOf(b.JsString).new();
    defer b.jsRelease(parts);
    parts.pushString("a");
    parts.pushString("b");
    parts.pushString("c");
    expect(strEq(b.join("-", parts), "a-b-c"), @src());
    expect(strEq(b.join("-", .{ .h = 0 }), ""), @src());

    // Numeric enums.
    expect(b.bump(.Low) == .High, @src());
    expect(b.bump(.High) == .High, @src());
}
