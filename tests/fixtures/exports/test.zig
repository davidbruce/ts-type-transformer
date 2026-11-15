// Calls a module library through the generated Zig bindings; every expect must hold.
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
    // A default export, reached as `default`.
    expect(b.origin().getX() == 0, @src());

    // Exported under other names: called and constructed by them.
    expect(b.area(3, 4) == 12, @src());
    const f = b.Figure.new(2);
    expect(f.grow() == 3 and f.getSize() == 3, @src());
    expect(strEq(b.Badge.new("x").getLabel(), "x"), @src());

    // Constants are read, never assigned: there is no setLIMIT.
    expect(b.getLIMIT() == 10, @src());
    expect(b.getCounter() == 0 and b.bump() == 1 and b.getCounter() == 1, @src());

    // A union told apart by the classes as exported.
    switch (b.pick(0)) {
        .Figure => |figure| expect(figure.getSize() == 2, @src()),
        else => expect(false, @src()),
    }
    expect(b.pick(1) == .Badge, @src());
}
