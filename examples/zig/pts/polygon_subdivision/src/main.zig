const std = @import("std");
const b = @import("pts");

var g_space: b.CanvasSpace = undefined;
var g_form: b.CanvasForm = undefined;

const SHADES = [9][]const u8{
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626",
};

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
    const sx = size.getX();
    const sy = size.getY();
    const radius = @max(80.0, @min(sx, sy) * 0.40);

    const px_norm = (pointer.getX() / @max(1.0, sx)) - 0.5;
    const t = b.Num.clamp(0.22 + 0.16 * @sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

    var poly = b.Polygon.fromCenter(.{ .Pt = c }, radius, 6.0);
    _ = poly.rotate2D(time_ms * 0.00035, .{ .anchor = .{ .Pt = c } });
    const centroid = b.Polygon.centroid(.{ .Group = poly });

    for (0..9) |depth| {
        const sw: f64 = if (depth == 0) 1.6 else 1.0;
        _ = g_form
            .fill(.{ .string = b.JsString.from(SHADES[depth]) })
            .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = sw })
            .polygon2(.{ .Group = poly });
        if (depth == 0) {
            var it = poly.iterate();
            while (it.next()) |pt| {
                const ray = mkGroup2(centroid, pt);
                _ = g_form.strokeOnly(.{ .string = b.JsString.from("#d4d4d4") }, .{ .width = 1.0 }).line2(.{ .Group = ray });
                _ = g_form.fillOnly(.{ .string = b.JsString.from("#111111") }).point2(.{ .Pt = pt }, .{ .radius = 3.5, .shape = "circle" });
            }
        }
        poly = b.Polygon.midpoints(.{ .Group = poly }, .{ .closePath = true, .t = t });
    }

    const cx = c.getX();
    const cy = c.getY();
    const hbar = mkGroup2(mkPt(cx - 10.0, cy), mkPt(cx + 10.0, cy));
    const vbar = mkGroup2(mkPt(cx, cy - 10.0), mkPt(cx, cy + 10.0));
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#ffffff") }, .{ .width = 1.5 }).line2(.{ .Group = hbar });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#ffffff") }, .{ .width = 1.5 }).line2(.{ .Group = vbar });
    _ = g_form
        .fill(.{ .string = b.JsString.from("#ffffff") })
        .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.5 })
        .point2(.{ .Pt = centroid }, .{ .radius = 5.0, .shape = "circle" });

    const ring = b.Circle.fromCenter(.{ .Pt = pointer }, 10.0);
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#737373") }, .{ .width = 1.5 }).circle2(.{ .Group = ring });
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
