const std = @import("std");
const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn mkRect(x: f64, y: f64, w: f64, h: f64) px.Rectangle {
    return px.Rectangle.new(.{ .x = x, .y = y, .width = w, .height = h });
}

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

    const query = mkRect(190.0, 105.0, 260.0, 170.0);
    const padded = query.clone().pad(.{ .paddingX = 10.0, .paddingY = 10.0 });
    const hull = query.clone();

    const cands = [_]px.Rectangle{
        mkRect(210.0, 125.0, 90.0, 60.0),
        mkRect(330.0, 165.0, 95.0, 80.0),
        mkRect(110.0, 80.0, 130.0, 85.0),
        mkRect(390.0, 75.0, 130.0, 90.0),
        mkRect(135.0, 225.0, 110.0, 90.0),
        mkRect(40.0, 40.0, 100.0, 110.0),
        mkRect(510.0, 48.0, 125.0, 115.0),
        mkRect(505.0, 245.0, 130.0, 110.0),
    };

    var contained: usize = 0;
    var intersecting: usize = 0;
    var disjoint: usize = 0;

    const g = px.Graphics.new(null);
    for (cands) |c| {
        _ = hull.enlarge(c);
        _ = g.rect(c.getX(), c.getY(), c.getWidth(), c.getHeight());
        if (query.containsRect(c)) {
            contained += 1;
            _ = g.fillWithColorAndAlpha(0x111111, null);
        } else if (query.intersects(c, null)) {
            intersecting += 1;
            _ = g.fillWithColorAndAlpha(0x737373, null);
            strokeG(g, 2.0, 0x111111);
        } else {
            disjoint += 1;
            _ = g.fillWithColorAndAlpha(0xe5e5e5, null);
            strokeG(g, 1.0, 0xa3a3a3);
        }
    }

    _ = g.rect(hull.getX(), hull.getY(), hull.getWidth(), hull.getHeight());
    strokeG(g, 1.5, 0xa3a3a3);
    _ = g.rect(padded.getX(), padded.getY(), padded.getWidth(), padded.getHeight());
    strokeG(g, 1.0, 0x737373);
    _ = g.rect(query.getX(), query.getY(), query.getWidth(), query.getHeight());
    strokeG(g, 2.5, 0x111111);

    _ = stage.addChild(.{g});
    app.render();

    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "query 260x170 · {d} contained · {d} intersecting · {d} disjoint · enlarged hull {d:.0}x{d:.0}",
        .{ contained, intersecting, disjoint, hull.getWidth(), hull.getHeight() },
    ) catch "query 260x170 · 8 candidates · 680x400 WebGL";

    if (doc.getElementById("meta")) |meta| {
        meta.setTextContent(msg);
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
