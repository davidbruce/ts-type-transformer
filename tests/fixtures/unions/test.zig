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

// Values coming back, told apart by a switch.
fn kind(v: b.Value) []const u8 {
    return switch (v) {
        .string => "string",
        .number => "number",
        .Widget => "widget",
        .Options => "options",
    };
}

export fn run() void {
    // Values made here, one per member.
    const widget = b.Widget.new("w");
    const options = b.Options.create();
    options.setVerbose(true);
    expect(strEq(b.describe(.{ .string = b.JsString.from("text") }), "string text"), @src());
    expect(strEq(b.describe(.{ .number = 42 }), "number 42"), @src());
    expect(strEq(b.describe(.{ .Widget = widget }), "widget w"), @src());
    expect(strEq(b.describe(.{ .Options = options }), "options verbose=true"), @src());

    // Values made by JavaScript: typeof, instanceof, and a property only one member has.
    expect(std.mem.eql(u8, kind(b.pick(0)), "string"), @src());
    expect(std.mem.eql(u8, kind(b.pick(1)), "number"), @src());
    expect(std.mem.eql(u8, kind(b.pick(2)), "widget"), @src());
    expect(std.mem.eql(u8, kind(b.pick(3)), "options"), @src());
    expect(strEq(b.pick(0).string, "text"), @src());
    expect(b.pick(1).number == 42, @src());
    expect(strEq(b.pick(2).Widget.getName(), "w"), @src());
    expect(b.pick(3).Options.getVerbose(), @src());

    // A tagged union: told apart by its `kind` property.
    switch (b.makeShape("circle", 2)) {
        .Circle => |c| expect(c.getRadius() == 2, @src()),
        .Square => expect(false, @src()),
    }
    switch (b.makeShape("square", 3)) {
        .Circle => expect(false, @src()),
        .Square => |q| expect(q.getSide() == 3, @src()),
    }
    expect(b.area(b.makeShape("square", 3)) == 9, @src());

    // Unions written inline are named after their members.
    expect(strEq(b.setup(.{ .boolean = true }), "bool true"), @src());
    expect(strEq(b.setup(.{ .Options = options }), "options verbose=true"), @src());
    expect(strEq(b.lookup("w").Widget.getName(), "w"), @src());
    expect(b.lookup("abc").number == 3, @src());
}
