const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn onInit(app: *px.Application, ok: bool, _: px.JsValue) void {
    if (!ok) return;
    const scope = px.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    if (doc.getElementById("stage")) |stage_el| {
        _ = stage_el.appendChild(app.getCanvas());
    }

    const stage = app.getStage();

    const frame = px.Graphics.new(null);
    _ = frame.rect(32.0, 20.0, 616.0, 360.0);
    _ = frame.stroke(.{px.StrokeStyle.init(.{ .width = 1.5, .color = 0xd4d4d4 })});
    _ = stage.addChild(.{frame});

    const shades = [_]f64{
        0x111111, 0x262626, 0x404040, 0x555555,
        0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5,
    };
    const white = px.Texture.getWHITE();

    for (0..6) |row| {
        for (0..10) |col| {
            const s = px.Sprite.new(white);
            const idx = (row * 3 + col * 5) % shades.len;
            s.setWidth(52.0);
            s.setHeight(48.0);
            _ = s.getAnchor().set(.{ .x = 0.5, .y = 0.5 });
            s.setX(70 + col * 60);
            s.setY(56 + row * 56);
            s.setRotation((@as(f64, @floatFromInt(col)) - 4.5) * 0.04 + (@as(f64, @floatFromInt(row)) - 2.5) * 0.03);
            s.setAlpha(0.65 + 0.35 * @as(f64, @floatFromInt((col + row) % 3)) / 2.0);
            s.setTint(shades[idx]);
            _ = stage.addChild(.{s});
        }
    }

    app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.setTextContent("60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL");
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
