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

    const grid = px.Graphics.new(null);
    var x: f64 = 40.0;
    while (x <= 640.0) : (x += 40.0) {
        _ = grid.moveTo(x, 0.0).lineTo(x, 400.0);
    }
    var y: f64 = 40.0;
    while (y <= 360.0) : (y += 40.0) {
        _ = grid.moveTo(0.0, y).lineTo(680.0, y);
    }
    strokeG(grid, 1.0, 0xe5e5e5);
    addGfx(stage, grid);

    const rect_g = px.Graphics.new(null);
    _ = rect_g.rect(40.0, 46.0, 140.0, 88.0).fillWithColorAndAlpha(.{ .number = 0x111111 }, .{});
    addGfx(stage, rect_g);

    const round_g = px.Graphics.new(null);
    _ = round_g.roundRect(216.0, 46.0, 156.0, 88.0, .{ .radius = 16.0 }).fillWithColorAndAlpha(.{ .number = 0xf7f7f7 }, .{});
    strokeG(round_g, 2.0, 0x111111);
    addGfx(stage, round_g);

    const circle_g = px.Graphics.new(null);
    _ = circle_g.circle(458.0, 90.0, 44.0).fillWithColorAndAlpha(.{ .number = 0x404040 }, .{});
    _ = circle_g.circle(458.0, 90.0, 22.0);
    strokeG(circle_g, 2.0, 0xffffff);
    _ = circle_g.circle(458.0, 90.0, 52.0);
    strokeG(circle_g, 1.5, 0x111111);
    addGfx(stage, circle_g);

    const ellipse_g = px.Graphics.new(null);
    _ = ellipse_g.ellipse(588.0, 90.0, 56.0, 36.0).fillWithColorAndAlpha(.{ .number = 0xd4d4d4 }, .{});
    strokeG(ellipse_g, 1.5, 0x111111);
    addGfx(stage, ellipse_g);

    const poly_g = px.Graphics.new(null);
    _ = poly_g.regularPoly(130.0, 274.0, 58.0, 6.0, .{ .rotation = 0.25 }).fillWithColorAndAlpha(.{ .number = 0x737373 }, .{});
    strokeG(poly_g, 2.0, 0x111111);
    addGfx(stage, poly_g);

    const star_g = px.Graphics.new(null);
    _ = star_g.star(340.0, 274.0, 5.0, 62.0, .{ .innerRadius = 26.0, .rotation = -0.314 }).fillWithColorAndAlpha(.{ .number = 0x111111 }, .{});
    addGfx(stage, star_g);

    const diamond_g = px.Graphics.new(null);
    _ = diamond_g.moveTo(548.0, 212.0).lineTo(618.0, 274.0).lineTo(548.0, 336.0).lineTo(478.0, 274.0).closePath();
    _ = diamond_g.fillWithColorAndAlpha(.{ .number = 0xb5b5b5 }, .{});
    strokeG(diamond_g, 1.5, 0x111111);
    addGfx(stage, diamond_g);

    g_app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent("7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
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
