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

    const c = state.space.getCenter();
    const size = state.space.getSize();
    const pointer = state.space.getPointer();
    const min_dim = @max(120.0, @min(size.getX(), size.getY()));
    const cx = c.getX();
    const cy = c.getY();

    const ang = time_ms * 0.00035;
    const half_len = min_dim * 0.44;
    const ca = @cos(ang);
    const sa = @sin(ang);
    const chord = seg(pt(cx - ca * half_len, cy - sa * half_len), pt(cx + ca * half_len, cy + sa * half_len));
    _ = state.form.strokeOnlyWith("#999999", .{ .width = 1.2 }).line(chord);

    const px = pointer.getX();
    const py = pointer.getY();
    const probe_center = if (px > 1.0 or py > 1.0) pointer else c;
    const probe = b.Circle.fromCenter(probe_center, min_dim * 0.20);

    var targets: [5]b.Group = undefined;
    for (0..5) |i| {
        const fi = @as(f64, @floatFromInt(i));
        const orbit_r = min_dim * (0.14 + 0.04 * fi);
        const theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
        const tc = pt(cx + @cos(theta) * orbit_r, cy + @sin(theta) * orbit_r);
        const tr = min_dim * (0.11 + 0.025 * @as(f64, @floatFromInt(i % 3)));
        targets[i] = b.Circle.fromCenter(tc, tr);

        _ = state.form
            .fill("#f7f7f7")
            .strokeWith("#737373", .{ .width = 1.4 })
            .circle(targets[i]);
        if (b.Line.perpendicularFromPtWith(chord, tc, .{ .asProjection = false })) |foot| {
            _ = state.form.strokeOnlyWith("#b5b5b5", .{ .width = 1.0 }).line(seg(tc, foot));
            _ = state.form.fillOnly("#555555").pointWith(foot, .{ .radius = 2.5, .shape = "circle" });
        }
    }

    _ = state.form.strokeOnlyWith("#111111", .{ .width = 2.0 }).circle(probe);
    const pcx = probe_center.getX();
    const pcy = probe_center.getY();
    _ = state.form.strokeOnlyWith("#111111", .{ .width = 1.4 }).line(seg(pt(pcx - 6.0, pcy), pt(pcx + 6.0, pcy)));
    _ = state.form.strokeOnlyWith("#111111", .{ .width = 1.4 }).line(seg(pt(pcx, pcy - 6.0), pt(pcx, pcy + 6.0)));

    for (targets) |target| {
        const hits = b.Circle.intersectCircle2D(probe, target);
        if (hits.getLength() == 2.0) {
            _ = state.form.strokeOnlyWith("#111111", .{ .width = 1.4 }).line(hits);
        }
        var it = hits.iterate();
        while (it.next()) |h| {
            _ = state.form
                .fill("#ffffff")
                .strokeWith("#111111", .{ .width = 2.0 })
                .pointWith(h, .{ .radius = 5.0, .shape = "circle" });
        }
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
