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
    const base_r = @max(70.0, @min(size.getX(), size.getY()) * 0.34);
    const cx = c.getX();
    const cy = c.getY();

    const radial = b.Create.radialPts(.{ .Pt = c }, base_r, 20.0, .{ .angleOffset = time_ms * 0.00025 });
    const cloud_arr = b.JsArrayOf(b.Pt).new();
    var it_r = radial.iterate();
    var idx: usize = 0;
    while (it_r.next()) |p| {
        const fi = @as(f64, @floatFromInt(idx));
        const scale = 0.48 + 0.52 * (0.5 + 0.5 * @sin(time_ms * 0.0011 + fi * 0.9) * @cos(time_ms * 0.0007 - fi * 0.5));
        cloud_arr.push(mkPt(cx + (p.getX() - cx) * scale, cy + (p.getY() - cy) * scale));
        idx += 1;
    }
    const px = pointer.getX();
    const py = pointer.getY();
    if (px > 1.0 or py > 1.0) {
        cloud_arr.push(mkPt(px, py));
    }

    const cloud = b.Group.new(cloud_arr);
    const hull = b.Polygon.convexHull(.{ .Group = cloud }, .{ .sorted = false });
    const bbox = b.Geom.boundingBox(.{ .Group = hull });
    const cent = b.Polygon.centroid(.{ .Group = hull });

    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#b5b5b5") }, .{ .width = 1.0 }).rect2(.{ .Group = bbox });
    _ = g_form
        .fill(.{ .string = b.JsString.from("#f7f7f7") })
        .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = 2.0 })
        .polygon2(.{ .Group = hull });

    const net0 = b.Polygon.network(.{ .Group = hull }, .{ .originIndex = 0.0 });
    for (0..net0.len()) |i| {
        _ = g_form.strokeOnly(.{ .string = b.JsString.from("#d4d4d4") }, .{ .width = 1.0 }).line2(.{ .Group = net0.get(i) });
    }
    const hull_len: usize = @intFromFloat(hull.getLength());
    if (hull_len > 3) {
        const net_mid = b.Polygon.network(.{ .Group = hull }, .{ .originIndex = @as(f64, @floatFromInt(hull_len / 2)) });
        for (0..net_mid.len()) |i| {
            _ = g_form.strokeOnly(.{ .string = b.JsString.from("#d4d4d4") }, .{ .width = 1.0 }).line2(.{ .Group = net_mid.get(i) });
        }
    }

    var it_cl = cloud.iterate();
    while (it_cl.next()) |p| {
        _ = g_form.fillOnly(.{ .string = b.JsString.from("#737373") }).point2(.{ .Pt = p }, .{ .radius = 3.0, .shape = "circle" });
    }
    var it_h = hull.iterate();
    while (it_h.next()) |h| {
        _ = g_form
            .fill(.{ .string = b.JsString.from("#111111") })
            .stroke(.{ .string = b.JsString.from("#ffffff") }, .{ .width = 1.5 })
            .point2(.{ .Pt = h }, .{ .radius = 5.0, .shape = "circle" });
    }

    const cent_x = cent.getX();
    const cent_y = cent.getY();
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.4 }).line2(.{ .Group = mkGroup2(mkPt(cent_x - 10.0, cent_y), mkPt(cent_x + 10.0, cent_y)) });
    _ = g_form.strokeOnly(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.4 }).line2(.{ .Group = mkGroup2(mkPt(cent_x, cent_y - 10.0), mkPt(cent_x, cent_y + 10.0)) });
    _ = g_form
        .fill(.{ .string = b.JsString.from("#ffffff") })
        .stroke(.{ .string = b.JsString.from("#111111") }, .{ .width = 1.6 })
        .point2(.{ .Pt = cent }, .{ .radius = 5.5, .shape = "circle" });
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
