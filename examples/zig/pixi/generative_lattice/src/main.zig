const std = @import("std");
const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn strokeG(g: px.Graphics, width: f64, color_hex: f64) void {
    const ss = px.StrokeStyle.create();
    ss.asStrokeAttributes().setWidth(width);
    ss.asFillStyle().setColor(.{ .number = color_hex });
    const args = px.JsArray.new();
    args.push(.{ .h = ss.h });
    _ = g.stroke(args);
}

fn addGfx(stage: px.Container, g: px.Graphics) void {
    _ = stage.asPixiMixins_Container().addChild(
        px.JsArrayOf(px.Container).of(&.{g.asViewContainer().asContainer()}),
    );
}

fn onInit(_: ?*anyopaque, ok: bool, _: px.JsValue) void {
    if (!ok) return;

    const doc = dom.getDocument();
    if (doc.getElementById("stage")) |stage_el| {
        _ = stage_el.asNode().appendChild(.{ .h = g_app.getCanvas().h });
    }

    const stage = g_app.getStage();

    const rings = [_]f64{ 42.0, 82.0, 122.0, 162.0 };
    const ring_colors = [_]f64{ 0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5 };

    const guides = px.Graphics.new(null);
    for (rings) |r| {
        _ = guides.circle(340.0, 200.0, r);
        strokeG(guides, 1.0, 0xe5e5e5);
    }

    const unit_x = px.Point.new(.{ .x = 1.0, .y = 0.0 });
    var step: usize = 0;
    while (step < 12) : (step += 1) {
        const theta = @as(f64, @floatFromInt(step)) * (2.0 * std.math.pi / 12.0);
        const dir_pd = px.Matrix.new(.{}).rotate(theta).apply(.{ .h = unit_x.h }, .{});
        const dir: px.Point = .{ .h = dir_pd.h };
        _ = guides.moveTo(340.0, 200.0).lineTo(340.0 + dir.getX() * 182.0, 200.0 + dir.getY() * 182.0);
    }
    strokeG(guides, 1.0, 0xd4d4d4);
    addGfx(stage, guides);

    const tiles = px.Graphics.new(null);
    for (rings, 0..) |radius, r_idx| {
        const s = 8.0 + @as(f64, @floatFromInt(r_idx)) * 2.5;
        const cx = [_]f64{ 0.0, s * 0.72, 0.0, -s * 0.72 };
        const cy = [_]f64{ -s, 0.0, s, 0.0 };

        var t_step: usize = 0;
        while (t_step < 12) : (t_step += 1) {
            const theta = @as(f64, @floatFromInt(t_step)) * (2.0 * std.math.pi / 12.0) + @as(f64, @floatFromInt(r_idx)) * 0.13;
            const dir_pd = px.Matrix.new(.{}).rotate(theta).apply(.{ .h = unit_x.h }, .{});
            const dir: px.Point = .{ .h = dir_pd.h };
            const tile_m = px.Matrix.new(.{})
                .rotate(theta + @as(f64, @floatFromInt(r_idx)) * 0.2)
                .translate(340.0 + radius * dir.getX(), 200.0 + radius * dir.getY());

            const p0: px.Point = .{ .h = tile_m.apply(.{ .h = px.Point.new(.{ .x = cx[0], .y = cy[0] }).h }, .{}).h };
            _ = tiles.moveTo(p0.getX(), p0.getY());
            var k: usize = 1;
            while (k < 4) : (k += 1) {
                const pk: px.Point = .{ .h = tile_m.apply(.{ .h = px.Point.new(.{ .x = cx[k], .y = cy[k] }).h }, .{}).h };
                _ = tiles.lineTo(pk.getX(), pk.getY());
            }
            _ = tiles.closePath().fillWithColorAndAlpha(.{ .number = ring_colors[r_idx] }, .{});
            strokeG(tiles, 1.2, 0x111111);
        }
    }
    addGfx(stage, tiles);

    const center_dot = px.Graphics.new(null);
    _ = center_dot.circle(340.0, 200.0, 8.0).fillWithColorAndAlpha(.{ .number = 0x111111 }, .{});
    addGfx(stage, center_dot);

    g_app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent("48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL");
    }
}

export fn start() void {
    g_app = px.Application.new();
    const opts = px.Application_init_options.create();
    opts.setWidth(680.0);
    opts.setHeight(400.0);
    opts.setBackgroundColor(.{ .number = 0xffffff });
    opts.setPreference(.{ .string = px.JsString.from("webgl") });
    opts.setPreserveDrawingBuffer(true);
    opts.setAntialias(true);
    opts.setAutoStart(false);
    const promise = g_app.init(opts);
    promise.then(onInit, null);
}
