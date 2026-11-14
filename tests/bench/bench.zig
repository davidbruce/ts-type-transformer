// The benchmark through the Zig bindings.
const b = @import("bindings.zig");

export fn calls(n: i32) f64 {
    var s: f64 = 0;
    var i: i32 = 0;
    while (i < n) : (i += 1) s = b.add(s, @floatFromInt(i));
    return s;
}

export fn objects(n: i32) f64 {
    var s: f64 = 0;
    var i: i32 = 0;
    while (i < n) : (i += 1) {
        const v = b.Vec2.new(@floatFromInt(i), 1);
        defer b.jsRelease(v);
        s += v.length();
    }
    return s;
}

export fn properties(n: i32) f64 {
    const v = b.Vec2.new(0, 0);
    defer b.jsRelease(v);
    var s: f64 = 0;
    var i: i32 = 0;
    while (i < n) : (i += 1) {
        v.set_x(@floatFromInt(i));
        s += v.get_x();
    }
    return s;
}

export fn strings_in(n: i32) f64 {
    var s: f64 = 0;
    var i: i32 = 0;
    while (i < n) : (i += 1) s += b.count("hello world");
    return s;
}

export fn strings_fresh(n: i32) f64 {
    var s: f64 = 0;
    var text = "hello world".*;
    var i: i32 = 0;
    while (i < n) : (i += 1) {
        text[0] = 'a' + @as(u8, @intCast(@mod(i, 26)));
        s += b.count(&text);
    }
    return s;
}

export fn strings_out(n: i32) f64 {
    var s: f64 = 0;
    var buf: [32]u8 = undefined;
    var i: i32 = 0;
    while (i < n) : (i += 1) {
        const g = b.greet("you");
        defer b.jsRelease(g);
        const text = g.copy(&buf);
        s += @floatFromInt(text.len + text[0]);
    }
    return s;
}

fn onI(ctx: ?*anyopaque, i: f64) void {
    const s: *f64 = @ptrCast(@alignCast(ctx));
    s.* += i;
}

export fn callbacks(n: i32) f64 {
    var s: f64 = 0;
    const f = b.each_f.from(onI, &s);
    defer b.jsRelease(f);
    b.each(@floatFromInt(n), f);
    _ = b.jsForgetCallback(f);
    return s;
}

export fn compute(n: i32) f64 {
    var x: u32 = 1;
    var s: f64 = 0;
    var i: i32 = 0;
    while (i < n) : (i += 1) {
        x = x *% 1664525 +% 1013904223;
        s += @floatFromInt(x >> 24);
    }
    return s;
}
