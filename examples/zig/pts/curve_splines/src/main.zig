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

    const size = g_space.getSize();
    const c = g_space.getCenter();
    const pointer = g_space.getPointer();
    const w = @max(120.0, size.getX());
    const h = @max(120.0, size.getY());
    const cy = c.getY();
    const px = pointer.getX();
    const py = pointer.getY();
    const amp = h * 0.24;
    const count: usize = 11;

    _ = g_form
        .strokeOnly(.{ .string = b.JsString.from("#e5e5e5") }, .{ .width = 1.0 })
        .line2(.{ .Group = mkGroup2(mkPt(w * 0.06, cy), mkPt(w * 0.94, cy)) });

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
        arr.push(mkPt(x, y));
    }

    const ctrls = b.Group.new(arr);
    const bs = b.Curve.bspline(.{ .Group = ctrls }, .{ .steps = 12.0 });
    const card = b.Curve.cardinal(.{ .Group = ctrls }, .{ .steps = 12.0, .tension = 0.35 });
    const cr = b.Curve.catmullRom(.{ .Group = ctrls }, .{ .steps = 12.0 });

    var it_cr = cr.iterate();
    var idx: usize = 0;
    while (it_cr.next()) |p| {
        if (idx % 3 == 0) {
            const stem = mkGroup2(mkPt(p.getX(), cy), p);
            _ = g_form.strokeOnly(.{ .string = b.JsString.from("#d4d4d4") }, .{ .width = 1.0 }).line2(.{ .Group = stem });
        }
        idx += 1;
    }

    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#d4d4d4") }, .{ .width = 1.0 }).line2(.{ .Group = ctrls });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#b5b5b5") }, .{ .width = 2.0 }).line2(.{ .Group = bs });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#555555") }, .{ .width = 1.8 }).line2(.{ .Group = card });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 2.6 }).line2(.{ .Group = cr });

    var it_c = ctrls.iterate();
    while (it_c.next()) |p| {
        _ = g_form
            .fill(.{ .string = b.JsString.from("#ffffff") })
            .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.5 })
            .point2(.{ .Pt = p }, .{ .radius = 4.5, .shape = "circle" });
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
