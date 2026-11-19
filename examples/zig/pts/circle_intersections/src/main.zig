const std = @import("std");
const b = @import("pts");

var g_space: b.CanvasSpace = undefined;
var g_form: b.CanvasForm = undefined;

fn mkPt(x: f64, y: f64) b.Pt {
    const p = b.Pt.make(2.0, .{});
    p.setX(x);
    p.setY(y);
    return p;
}

fn mkGroup2(a: b.Pt, p_b: b.Pt) b.Group {
    const arr = b.JsArrayOf(b.Pt).new();
    arr.push(a);
    arr.push(p_b);
    return b.Group.new(arr);
}

fn onAnimate(_: ?*anyopaque, time_ms: f64, _: f64, _: b.Space) void {
    const scope = b.JsScope.open();
    defer scope.close();

    const c = g_space.getCenter();
    const size = g_space.getSize();
    const pointer = g_space.getPointer();
    const min_dim = @max(120.0, @min(size.getX(), size.getY()));
    const cx = c.getX();
    const cy = c.getY();

    const ang = time_ms * 0.00035;
    const half_len = min_dim * 0.44;
    const ca = @cos(ang);
    const sa = @sin(ang);
    const chord = mkGroup2(mkPt(cx - ca * half_len, cy - sa * half_len), mkPt(cx + ca * half_len, cy + sa * half_len));
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#999999") }, .{ .width = 1.2 }).line2(.{ .Group = chord });

    const px = pointer.getX();
    const py = pointer.getY();
    const probe_center = if (px > 1.0 or py > 1.0) pointer else c;
    const probe = b.Circle.fromCenter(.{ .Pt = probe_center }, min_dim * 0.20);

    var targets: [5]b.Group = undefined;
    for (0..5) |i| {
        const fi = @as(f64, @floatFromInt(i));
        const orbit_r = min_dim * (0.14 + 0.04 * fi);
        const theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
        const tc = mkPt(cx + @cos(theta) * orbit_r, cy + @sin(theta) * orbit_r);
        const tr = min_dim * (0.11 + 0.025 * @as(f64, @floatFromInt(i % 3)));
        targets[i] = b.Circle.fromCenter(.{ .Pt = tc }, tr);

        _ = g_form
            .fill(.{ .string = b.JsString.from("#f7f7f7") })
            .stroke(.{ .string = b.JsString.from("#737373") }, .{ .width = 1.4 })
            .circle2(.{ .Group = targets[i] });
        if (b.Line.perpendicularFromPt(.{ .Group = chord }, .{ .Pt = tc }, .{ .asProjection = false })) |foot| {
            _ = g_form.strokeOnly(.{ .string = b.JsString.from("#b5b5b5") }, .{ .width = 1.0 }).line2(.{ .Group = mkGroup2(tc, foot) });
            _ = g_form.fillOnly(.{ .string = b.JsString.from("#555555") }).point2(.{ .Pt = foot }, .{ .radius = 2.5, .shape = "circle" });
        }
    }

    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 2.0 }).circle2(.{ .Group = probe });
    const pcx = probe_center.getX();
    const pcy = probe_center.getY();
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.4 }).line2(.{ .Group = mkGroup2(mkPt(pcx - 6.0, pcy), mkPt(pcx + 6.0, pcy)) });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.4 }).line2(.{ .Group = mkGroup2(mkPt(pcx, pcy - 6.0), mkPt(pcx, pcy + 6.0)) });

    for (0..5) |i| {
        const hits = b.Circle.intersectCircle2D(.{ .Group = probe }, .{ .Group = targets[i] });
        if (hits.getLength() == 2.0) {
            _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.4 }).line2(.{ .Group = hits });
        }
        var it = hits.iterate();
        while (it.next()) |h| {
            _ = g_form
                .fill(.{ .string = b.JsString.from("#ffffff") })
                .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = 2.0 })
                .point2(.{ .Pt = h }, .{ .radius = 5.0, .shape = "circle" });
        }
    }
}

export fn start() void {
    const opts = b.CanvasSpaceOptions.create();
    defer b.jsRelease(opts);
    opts.setBgcolor("#ffffff");
    opts.setResize(true);
    opts.setRetina(true);

    const id = b.JsString.from("#pt");
    defer b.jsRelease(id);
    const raw_space = b.CanvasSpace.new(.{ .string = id }, .{});
    defer b.jsRelease(raw_space);
    g_space = raw_space.setup(opts);
    g_form = g_space.getForm();

    const player = b.IPlayer.create();
    defer b.jsRelease(player);
    const cb = b.AnimateCallbackFn.from(onAnimate, null);
    defer b.jsRelease(cb);
    player.setAnimate(cb);

    const added = g_space.add(.{ .IPlayer = player });
    defer b.jsRelease(added);
    const mouse_bound = g_space.bindMouse(.{});
    defer b.jsRelease(mouse_bound);
    const touch_bound = mouse_bound.bindTouch(.{});
    defer b.jsRelease(touch_bound);
    const played = touch_bound.play(null);
    defer b.jsRelease(played);
}
