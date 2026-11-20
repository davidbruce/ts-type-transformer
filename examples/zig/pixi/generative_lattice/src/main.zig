const std = @import("std");
const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn strokeG(g: px.Graphics, width: f64, color_hex: f64) void {
    _ = g.stroke(.{px.StrokeStyle.init(.{ .width = width, .color = color_hex })});
}

fn onInit(app: *px.Application, ok: bool, _: px.JsValue) void {
    if (!ok) return;
    const scope = px.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    if (doc.getElementById("stage")) |stage_el| {
        _ = stage_el.appendChild(app.getCanvas());
    }

    const stage = app.getStage();

    const rings = [_]f64{ 42.0, 82.0, 122.0, 162.0 };
    const ring_colors = [_]f64{ 0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5 };

    const guides = px.Graphics.new(null);
    for (rings) |r| {
        _ = guides.circle(340.0, 200.0, r);
        strokeG(guides, 1.0, 0xe5e5e5);
    }

    for (0..12) |step| {
        const theta = @as(f64, @floatFromInt(step)) * (2.0 * std.math.pi / 12.0);
        const dir = px.Matrix.new(.{}).rotate(theta).apply(.{ .x = 1.0, .y = 0.0 }, null);
        _ = guides.moveTo(340.0, 200.0).lineTo(340.0 + dir.getX() * 182.0, 200.0 + dir.getY() * 182.0);
    }
    strokeG(guides, 1.0, 0xd4d4d4);
    _ = stage.addChild(.{guides});

    const tiles = px.Graphics.new(null);
    for (rings, 0..) |radius, r_idx| {
        const s = 8.0 + @as(f64, @floatFromInt(r_idx)) * 2.5;
        const cx = [_]f64{ 0.0, s * 0.72, 0.0, -s * 0.72 };
        const cy = [_]f64{ -s, 0.0, s, 0.0 };

        for (0..12) |t_step| {
            const theta = @as(f64, @floatFromInt(t_step)) * (2.0 * std.math.pi / 12.0) + @as(f64, @floatFromInt(r_idx)) * 0.13;
            const dir = px.Matrix.new(.{}).rotate(theta).apply(.{ .x = 1.0, .y = 0.0 }, null);
            const tile_m = px.Matrix.new(.{})
                .rotate(theta + @as(f64, @floatFromInt(r_idx)) * 0.2)
                .translate(340.0 + radius * dir.getX(), 200.0 + radius * dir.getY());

            const p0 = tile_m.apply(.{ .x = cx[0], .y = cy[0] }, null);
            _ = tiles.moveTo(p0.getX(), p0.getY());
            for (1..4) |k| {
                const pk = tile_m.apply(.{ .x = cx[k], .y = cy[k] }, null);
                _ = tiles.lineTo(pk.getX(), pk.getY());
            }
            _ = tiles.closePath().fillWithColorAndAlpha(ring_colors[r_idx], null);
            strokeG(tiles, 1.2, 0x111111);
        }
    }
    _ = stage.addChild(.{tiles});

    const center_dot = px.Graphics.new(null);
    _ = center_dot.circle(340.0, 200.0, 8.0).fillWithColorAndAlpha(0x111111, null);
    _ = stage.addChild(.{center_dot});

    app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.setTextContent("48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL");
    }
}

export fn start() void {
    const scope = px.JsScope.open();
    defer scope.deinit();

    g_app = px.Application.new().retain();
    g_app.init(.{
        .width = 680.0,
        .height = 400.0,
        .backgroundColor = 0xffffff,
        .preference = "webgl",
        .preserveDrawingBuffer = true,
        .antialias = true,
        .autoStart = false,
    }).then(onInit, &g_app);
}
