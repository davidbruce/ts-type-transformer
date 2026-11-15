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
    // Class pairing: constructor, static method, constant.
    const s = b.Shape.new(.square, 3);
    expect(s.area() == 9, @src());
    expect(b.Shape.unit().area() == 1, @src());
    expect(b.Shape.SIDES == 4, @src());

    // Literal union -> enum, in both directions.
    expect(s.getKind() == .square, @src());
    expect(b.Shape.new(.circle, 1).getKind() == .circle, @src());

    // Properties.
    s.setSize(4);
    expect(s.getSize() == 4, @src());

    // Flattened inheritance, and passing a Shape where a Named is wanted.
    expect(strEq(s.getName(), "square-3"), @src());
    expect(strEq(s.describe(), "square-3 (16)"), @src());
    expect(strEq(b.nameOf(s.asNamed()), "square-3"), @src());

    // Overloads.
    const big = s.scale(2);
    expect(big.getSize() == 8, @src());
    expect(strEq(s.scaleWithName(2, "big").getName(), "big"), @src());

    // Optional parameter.
    s.resize(2.6, .{});
    expect(s.getSize() == 2.6, @src());
    s.resize(2.6, .{ .round = true });
    expect(s.getSize() == 3, @src());

    // Union parameter: a tagged union.
    expect(strEq(s.label(.{ .string = b.JsString.from("a") }), "string:a"), @src());
    expect(strEq(s.label(.{ .number = 7 }), "number:7"), @src());

    // Nullable: an optional.
    expect(s.getParent() == null, @src());
    expect(b.jsSame(big.getParent().?, s), @src());
    big.setParent(null);
    expect(big.getParent() == null, @src());

    // Global functions.
    expect(b.totalArea(s, b.Shape.unit()) == 10, @src());
    expect(b.jsSame(b.largest(s, null).?, s), @src());
    expect(b.jsSame(b.largest(b.Shape.unit(), s).?, s), @src());

    // `undefined` rather than `null`.
    expect(b.orUnit(null).area() == 1, @src());
    expect(b.jsSame(b.orUnit(s), s), @src());
    expect(b.find("nothing") == null, @src());
    expect(b.find("unit").?.area() == 1, @src());
}
