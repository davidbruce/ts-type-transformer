const std = @import("std");
const b = @import("pts");

const App = struct {
    space: b.CanvasSpace = undefined,
    form: b.CanvasForm = undefined,
};

var app: App = .{};

fn onAnimate(state: *App, time_ms: f64, _: f64, _: b.Space) void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    const pointer = state.space.getPointer();
    const grid = b.Create.gridPts(state.space.getInnerBound(), 18.0, 11.0);
    const size = state.space.getSize();
    const max_dist = @max(1.0, @max(size.getX(), size.getY()) * 0.55);

    var it = grid.iterate();
    while (it.next()) |pt| {
        const delta = pt._subtract(.{pointer});
        const dist = delta.magnitude();
        const t = 1.0 - b.Num.clamp(dist / max_dist, 0.0, 1.0);
        const angle = delta.angle(null) + @sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
        const len = 8.0 + 34.0 * t;
        const seg = b.Line.fromAngle(pt, angle, len);

        var buf: [48]u8 = undefined;
        const color = std.fmt.bufPrint(
            &buf,
            "hsla(0, 0%, {d:.0}%, {d:.2})",
            .{ 65.0 - 57.0 * t, 0.25 + 0.75 * t },
        ) catch continue;

        _ = state.form.strokeOnlyWith(color, .{ .width = 1.5 + 2.5 * t }).line(seg);
        const dot = b.Circle.fromCenter(pt, 1.5 + 4.5 * t * t);
        _ = state.form.fillOnly(color).circle(dot);
    }

    const ring_radius = 18.0 + 6.0 * @sin(time_ms * 0.004);
    const ring = b.Circle.fromCenter(pointer, ring_radius);
    _ = state.form.strokeOnlyWith("#11111199", .{ .width = 2.0 }).circle(ring);
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
