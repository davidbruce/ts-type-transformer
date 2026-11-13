// Calls the generated Zig bindings; every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

var calls: u32 = 0;

fn onValue(ctx: ?*anyopaque, value: f64) callconv(.c) void {
    const out: *f64 = @ptrCast(@alignCast(ctx));
    out.* = value;
    calls += 1;
}
fn fold(_: ?*anyopaque, total: f64, label: b.JsString) callconv(.c) f64 {
    return total * 2 + @as(f64, @floatFromInt(label.len()));
}
fn isFive(_: ?*anyopaque, c: b.Counter) callconv(.c) bool {
    return c.get_count() == 5;
}
// A union argument arrives decoded; the function is known at compile time.
var number: f64 = 0;
var text_len: usize = 0;
fn onId(_: ?*anyopaque, id: b.Id) void {
    switch (id) {
        .number => |n| number = n,
        .string => |s| text_len = s.len(),
    }
}
fn bump(ctx: ?*anyopaque) callconv(.c) void {
    const n: *u32 = @ptrCast(@alignCast(ctx));
    n.* += 1;
}

export fn run() void {
    const c = b.Counter.new();
    var last: f64 = 0;

    // A callback is called with its context.
    c.subscribe(b.Listener.from(onValue, &last));
    expect(c.increment() == 1, @src());
    expect(last == 1, @src());
    _ = c.increment_with_by(4);
    expect(last == 5 and calls == 2, @src());

    // A callback that takes an object and returns a value.
    expect(c.visit(b.Counter_visit_f.from(isFive, null)), @src());

    // The same function and context is the same JavaScript function.
    c.unsubscribe(b.Listener.from(onValue, &last));
    _ = c.increment();
    expect(last == 5 and calls == 2, @src());

    // A nullable callback property.
    expect(c.get_onchange() == null, @src());
    var prop: f64 = 0;
    c.set_onchange(b.Listener.from(onValue, &prop));
    expect(c.get_onchange() != null, @src());
    _ = c.increment();
    expect(prop == 7, @src());
    c.set_onchange(null);
    _ = c.increment();
    expect(prop == 7, @src());

    // Several arguments, a string among them, and a result.
    expect(c.fold(b.Counter_fold_f.from(fold, null)) == 8 * 2 + 5, @src());

    // A callback kept and called later.
    var ran: u32 = 0;
    b.later(b.later_f.from(bump, &ran));
    b.later(b.later_f.from(bump, &ran));
    expect(ran == 0, @src());
    expect(b.flush() == 2, @src());
    expect(ran == 2, @src());

    expect(b.eachId(b.eachId_f.from(onId, null)) == 2, @src());
    expect(number == 1 and text_len == 3, @src());
}
