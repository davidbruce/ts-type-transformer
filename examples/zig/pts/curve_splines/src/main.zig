const std = @import("std");
const b = @import("pts");

const App = struct {
    space: b.CanvasSpace = undefined,
    form: b.CanvasForm = undefined,
};

var app: App = .{};

fn pt(x: f64, y: f64) b.Pt {
    return b.Pt.new(.{ x, y });
}

fn seg(a: b.Pt, c: b.Pt) b.Group {
    return b.Group.new(&.{ a, c });
}

fn onAnimate(state: *App, time_ms: f64, _: f64, _: b.Space) void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    const size = state.space.getSize();
    const c = state.space.getCenter();
    const pointer = state.space.getPointer();
    const w = @max(120.0, size.getX());
    const h = @max(120.0, size.getY());
    const cy = c.getY();
    const px = pointer.getX();
    const py = pointer.getY();
    const amp = h * 0.24;
    const count: usize = 11;

    _ = state.form
        .strokeOnlyWith("#e5e5e5", .{ .width = 1.0 })
        .line(seg(pt(w * 0.06, cy), pt(w * 0.94, cy)));

    const arr = b.JsArrayOf(b.Pt).new();
    for (0..count) |i| {
        const u = @as(f64, @floatFromInt(i)) / @as(f64, @floatFromInt(count - 1));
        const x = w * (0.08 + 0.84 * u);
        const fi = @as(f64, @floatFromInt(i));
        var y = cy +
            @sin(time_ms * 0.0014 + fi * 0.72) * amp +
            @cos(time_ms * 0.0009 - fi * 0.45) * (amp * 0.42);
        const dx = @abs(x - px);
        if (dx < w * 0.25 and (px > 1.0 or py > 1.0)) {
            const pull = 1.0 - dx / (w * 0.25);
            y += (py - y) * pull * 0.35;
        }
        arr.push(pt(x, y));
    }

    const ctrls = b.Group.new(arr);
    const bs = b.Curve.bsplineWith(ctrls, .{ .steps = 12.0 });
    const card = b.Curve.cardinalWith(ctrls, .{ .steps = 12.0, .tension = 0.35 });
    const cr = b.Curve.catmullRomWith(ctrls, .{ .steps = 12.0 });

    var it_cr = cr.iterate();
    var idx: usize = 0;
    while (it_cr.next()) |p| {
        if (idx % 3 == 0) {
            _ = state.form.strokeOnlyWith("#d4d4d4", .{ .width = 1.0 }).line(seg(pt(p.getX(), cy), p));
        }
        idx += 1;
    }

    _ = state.form.strokeOnlyWith("#d4d4d4", .{ .width = 1.0 }).line(ctrls);
    _ = state.form.strokeOnlyWith("#b5b5b5", .{ .width = 2.0 }).line(bs);
    _ = state.form.strokeOnlyWith("#555555", .{ .width = 1.8 }).line(card);
    _ = state.form.strokeOnlyWith("#111111", .{ .width = 2.6 }).line(cr);

    var it_c = ctrls.iterate();
    while (it_c.next()) |p| {
        _ = state.form
            .fill("#ffffff")
            .strokeWith("#111111", .{ .width = 1.5 })
            .pointWith(p, .{ .radius = 4.5, .shape = "circle" });
    }
}

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    app.space = b.CanvasSpace.new(.{ .elem = "#pt" }).setup(.{
        .bgcolor = "#ffffff",
        .resize = true,
        .retina = true,
    }).retain();
    app.form = app.space.getForm().retain();

    _ = app.space.add(.{ .animate = .{ onAnimate, &app } });
    _ = app.space.bindMouse(null).bindTouch(null).play(null);
}
