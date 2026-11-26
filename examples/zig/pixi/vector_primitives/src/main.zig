const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn strokeG(g: px.Graphics, width: f64, color_hex: f64) void {
    _ = g.stroke(px.StrokeStyle.init(.{ .width = width, .color = color_hex }));
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

    const rect_g = px.Graphics.new(null);
    _ = rect_g.rect(40.0, 46.0, 140.0, 88.0).fillWithColorAndAlpha(0x111111, null);

    const round_g = px.Graphics.new(null);
    _ = round_g.roundRectWith(216.0, 46.0, 156.0, 88.0, .{ .radius = 16.0 }).fillWithColorAndAlpha(0xf7f7f7, null);
    strokeG(round_g, 2.0, 0x111111);

    const circle_g = px.Graphics.new(null);
    _ = circle_g.circle(458.0, 90.0, 44.0).fillWithColorAndAlpha(0x404040, null);
    _ = circle_g.circle(458.0, 90.0, 22.0);
    strokeG(circle_g, 2.0, 0xffffff);
    _ = circle_g.circle(458.0, 90.0, 52.0);
    strokeG(circle_g, 1.5, 0x111111);

    const ellipse_g = px.Graphics.new(null);
    _ = ellipse_g.ellipse(588.0, 90.0, 56.0, 36.0).fillWithColorAndAlpha(0xd4d4d4, null);
    strokeG(ellipse_g, 1.5, 0x111111);

    const poly_g = px.Graphics.new(null);
    _ = poly_g.regularPolyWith(130.0, 274.0, 58.0, 6.0, .{ .rotation = 0.25 }).fillWithColorAndAlpha(0x737373, null);
    strokeG(poly_g, 2.0, 0x111111);

    const star_g = px.Graphics.new(null);
    _ = star_g.starWith(340.0, 274.0, 5.0, 62.0, .{ .innerRadius = 26.0, .rotation = -0.314 }).fillWithColorAndAlpha(0x111111, null);

    const diamond_g = px.Graphics.new(null);
    _ = diamond_g.moveTo(548.0, 212.0).lineTo(618.0, 274.0).lineTo(548.0, 336.0).lineTo(478.0, 274.0).closePath();
    _ = diamond_g.fillWithColorAndAlpha(0xb5b5b5, null);
    strokeG(diamond_g, 1.5, 0x111111);

    _ = stage.addChild(.{ grid, rect_g, round_g, circle_g, ellipse_g, poly_g, star_g, diamond_g });
    app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.setTextContent("7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
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
