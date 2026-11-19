const std = @import("std");
const b = @import("pts");

var g_space: b.CanvasSpace = undefined;
var g_form: b.CanvasForm = undefined;

fn onAnimate(_: ?*anyopaque, time_ms: f64, _: f64, _: b.Space) void {
    const scope = b.JsScope.open();
    defer scope.close();

    const pointer = g_space.getPointer();
    const grid = b.Create.gridPts(g_space.getInnerBound(), 18.0, 11.0, .{});
    const size = g_space.getSize();
    const max_dist = @max(1.0, @max(size.getX(), size.getY()) * 0.55);

    var it = grid.iterate();
    while (it.next()) |pt| {
        const args = b.JsArray.new();
        args.push(.{ .h = pointer.h });
        const delta = pt._subtract(args);
        const dist = delta.magnitude();
        const t = 1.0 - b.Num.clamp(dist / max_dist, 0.0, 1.0);
        const angle = delta.angle(null) + @sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
        const len = 8.0 + 34.0 * t;
        const seg = b.Line.fromAngle(.{ .Pt = pt }, angle, len);

        var buf: [48]u8 = undefined;
        const color = std.fmt.bufPrint(
            &buf,
            "hsla({d:.0}, 85%, 62%, {d:.2})",
            .{ 155.0 + 185.0 * t, 0.25 + 0.75 * t },
        ) catch continue;
        const color_str = b.JsString.from(color);

        _ = g_form.strokeOnly(.{ .string = color_str }, .{ .width = 1.5 + 2.5 * t }).line2(.{ .Group = seg });
        const dot = b.Circle.fromCenter(.{ .Pt = pt }, 1.5 + 4.5 * t * t);
        _ = g_form.fillOnly(.{ .string = color_str }).circle2(.{ .Group = dot });
    }

    const ring_radius = 18.0 + 6.0 * @sin(time_ms * 0.004);
    const ring = b.Circle.fromCenter(.{ .Pt = pointer }, ring_radius);
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#ffffffaa") }, .{ .width = 2.0 }).circle2(.{ .Group = ring });
}

export fn start() void {
    const opts = b.CanvasSpaceOptions.create();
    defer b.jsRelease(opts);
    opts.setBgcolor("#101622");
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
