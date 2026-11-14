// Calls Pts, a real library, through its generated Zig bindings; every expect must hold.
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

// Coordinates: a Pt's constructor, and its arithmetic, take any number of them.
fn xy(x: f64, y: f64) b.JsArray {
    const a = b.JsArray.new();
    a.push(b.JsValue.number(x));
    a.push(b.JsValue.number(y));
    return a;
}

fn pt(x: f64, y: f64) b.Pt {
    return b.Pt.new(xy(x, y));
}

export fn run() void {
    const live = b.jsLiveHandles();
    {
        // Everything that comes back while the scope is open is released with it.
        const scope = b.JsScope.open();
        defer scope.close();

        // A Pt is an array of coordinates, with names for the first four.
        const p = pt(3, 4);
        expect(p.get_x() == 3 and p.get_y() == 4, @src());
        expect(p.magnitude() == 5, @src());
        expect(strEq(p.toString(), "Pt(3, 4)"), @src());
        p.set_id("corner");
        expect(strEq(p.get_id(), "corner"), @src());

        // A method that changes a Pt returns it; its `$` twin returns another.
        const moved = p._add(xy(1, 1));
        expect(!b.jsSame(moved, p) and moved.get_x() == 4 and p.get_x() == 3, @src());
        expect(b.jsSame(p.add(xy(1, 1)), p) and p.get_x() == 4, @src());
        // A union is a tagged union; an optional argument is null.
        expect(p.equals(.{ .Pt = moved }, null), @src());
        expect(p.clone().equals(.{ .Pt = p }, 0.001) and !b.jsSame(p.clone(), p), @src());

        // Statics.
        expect(b.Num.lerp(0, 10, 0.5) == 5, @src());
        expect(b.Num.clamp(12, 0, 10) == 10, @src());
        expect(b.Geom.toDegree(std.math.pi) == 180, @src());
        expect(strEq(b.Pt.make(3, 2, null).toString(), "Pt(2, 2, 2)"), @src());

        // A Group is an array of Pt: indexed and iterated.
        const square = b.Group.new(b.JsArrayOf(b.Pt).of(&.{ pt(0, 0), pt(4, 0), pt(4, 4), pt(0, 4) }));
        expect(square.get_at(2).?.get_y() == 4, @src());
        var sum: f64 = 0;
        var corners = square.iterate();
        while (corners.next()) |corner| sum += corner.get_x();
        expect(sum == 8, @src());
        expect(square.get_p1().get_x() == 0, @src());
        expect(strEq(square.centroid().toString(), "Pt(2, 2)"), @src());
        expect(square.boundingBox().get_p2().get_x() == 4, @src());

        // A union parameter takes any of its members: a Group where points are wanted.
        expect(b.Geom.centroid(.{ .Group = square }).get_x() == 2, @src());
        const line = b.Group.new(b.JsArrayOf(b.Pt).of(&.{ pt(0, 0), pt(3, 4) }));
        expect(b.Line.magnitude(.{ .Group = line }) == 5, @src());
        const ray = b.Line.fromAngle(.{ .Pt = pt(1, 1) }, 0, 5);
        expect(ray.get_p2().get_x() == 6 and ray.get_p2().get_y() == 1, @src());
    }
    expect(b.jsLiveHandles() == live, @src());
}
