const std = @import("std");
const b = @import("pts");

const App = struct {
    space: b.CanvasSpace = undefined,
    form: b.CanvasForm = undefined,
};

var app: App = .{};

const shades = [9][]const u8{
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626",
};

fn pt(x: f64, y: f64) b.Pt {
    return b.Pt.new(.{ x, y });
}

fn seg(a: b.Pt, c: b.Pt) b.Group {
    return b.Group.new(&.{ a, c });
}

fn onAnimate(state: *App, time_ms: f64, _: f64, _: b.Space) void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    const c = state.space.getCenter();
    const size = state.space.getSize();
    const pointer = state.space.getPointer();
    const sx = size.getX();
    const sy = size.getY();
    const radius = @max(80.0, @min(sx, sy) * 0.40);

    const px_norm = (pointer.getX() / @max(1.0, sx)) - 0.5;
    const t = b.Num.clamp(0.22 + 0.16 * @sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

    var poly = b.Polygon.fromCenter(c, radius, 6.0);
    _ = poly.rotate2D(time_ms * 0.00035, .{ .anchor = c });
    const centroid = b.Polygon.centroid(poly);

    for (shades, 0..) |shade, depth| {
        const sw: f64 = if (depth == 0) 1.6 else 1.0;
        _ = state.form
            .fill(shade)
            .stroke("#111111", .{ .width = sw })
            .polygon(poly);
        if (depth == 0) {
            var it = poly.iterate();
            while (it.next()) |p| {
                _ = state.form.strokeOnly("#d4d4d4", .{ .width = 1.0 }).line(seg(centroid, p));
                _ = state.form.fillOnly("#111111").point(p, .{ .radius = 3.5, .shape = "circle" });
            }
        }
        poly = b.Polygon.midpoints(poly, .{ .closePath = true, .t = t });
    }

    const cx = c.getX();
    const cy = c.getY();
    _ = state.form.strokeOnly("#ffffff", .{ .width = 1.5 }).line(seg(pt(cx - 10.0, cy), pt(cx + 10.0, cy)));
    _ = state.form.strokeOnly("#ffffff", .{ .width = 1.5 }).line(seg(pt(cx, cy - 10.0), pt(cx, cy + 10.0)));
    _ = state.form
        .fill("#ffffff")
        .stroke("#111111", .{ .width = 1.5 })
        .point(centroid, .{ .radius = 5.0, .shape = "circle" });

    const ring = b.Circle.fromCenter(pointer, 10.0);
    _ = state.form.strokeOnly("#737373", .{ .width = 1.5 }).circle(ring);
}

export fn start() void {
    const scope = b.JsScope.open();
    defer scope.deinit();

    app.space = b.CanvasSpace.new("#pt", null).setup(.{
        .bgcolor = "#ffffff",
        .resize = true,
        .retina = true,
    }).retain();
    app.form = app.space.getForm().retain();

    _ = app.space.add(.{ .animate = .{ onAnimate, &app } });
    _ = app.space.bindMouse(null).bindTouch(null).play(null);
}
