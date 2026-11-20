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
    const base_r = @max(70.0, @min(size.getX(), size.getY()) * 0.34);
    const cx = c.getX();
    const cy = c.getY();

    const radial = b.Create.radialPts(c, base_r, 20.0, .{ .angleOffset = time_ms * 0.00025 });
    const cloud_arr = b.JsArrayOf(b.Pt).new();
    var it_r = radial.iterate();
    var idx: usize = 0;
    while (it_r.next()) |p| {
        const fi = @as(f64, @floatFromInt(idx));
        const scale = 0.48 + 0.52 * (0.5 + 0.5 * @sin(time_ms * 0.0011 + fi * 0.9) * @cos(time_ms * 0.0007 - fi * 0.5));
        cloud_arr.push(pt(cx + (p.getX() - cx) * scale, cy + (p.getY() - cy) * scale));
        idx += 1;
    }
    const px = pointer.getX();
    const py = pointer.getY();
    if (px > 1.0 or py > 1.0) {
        cloud_arr.push(pt(px, py));
    }

    const cloud = b.Group.new(cloud_arr);
    const hull = b.Polygon.convexHull(cloud, .{ .sorted = false });
    const bbox = b.Geom.boundingBox(hull);
    const cent = b.Polygon.centroid(hull);

    _ = state.form.strokeOnly("#b5b5b5", .{ .width = 1.0 }).rect2(bbox);
    _ = state.form
        .fill("#f7f7f7")
        .stroke("#111111", .{ .width = 2.0 })
        .polygon2(hull);

    const net0 = b.Polygon.network(hull, .{ .originIndex = 0.0 });
    for (0..net0.len()) |i| {
        _ = state.form.strokeOnly("#d4d4d4", .{ .width = 1.0 }).line2(net0.get(i));
    }
    const hull_len: usize = @intFromFloat(hull.getLength());
    if (hull_len > 3) {
        const net_mid = b.Polygon.network(hull, .{ .originIndex = @as(f64, @floatFromInt(hull_len / 2)) });
        for (0..net_mid.len()) |i| {
            _ = state.form.strokeOnly("#d4d4d4", .{ .width = 1.0 }).line2(net_mid.get(i));
        }
    }

    var it_cl = cloud.iterate();
    while (it_cl.next()) |p| {
        _ = state.form.fillOnly("#737373").point2(p, .{ .radius = 3.0, .shape = "circle" });
    }
    var it_h = hull.iterate();
    while (it_h.next()) |h| {
        _ = state.form
            .fill("#111111")
            .stroke("#ffffff", .{ .width = 1.5 })
            .point2(h, .{ .radius = 5.0, .shape = "circle" });
    }

    const cent_x = cent.getX();
    const cent_y = cent.getY();
    _ = state.form.strokeOnly("#111111", .{ .width = 1.4 }).line2(seg(pt(cent_x - 10.0, cent_y), pt(cent_x + 10.0, cent_y)));
    _ = state.form.strokeOnly("#111111", .{ .width = 1.4 }).line2(seg(pt(cent_x, cent_y - 10.0), pt(cent_x, cent_y + 10.0)));
    _ = state.form
        .fill("#ffffff")
        .stroke("#111111", .{ .width = 1.6 })
        .point2(cent, .{ .radius = 5.5, .shape = "circle" });
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
