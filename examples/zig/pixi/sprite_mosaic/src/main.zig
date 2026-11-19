const dom = @import("dom");
const px = @import("pixi_js");

var g_app: px.Application = undefined;

fn onInit(_: ?*anyopaque, ok: bool, _: px.JsValue) void {
    if (!ok) return;

    const doc = dom.getDocument();
    if (doc.getElementById("stage")) |stage_el| {
        _ = stage_el.asNode().appendChild(.{ .h = g_app.getCanvas().h });
    }

    const stage = g_app.getStage();

    const frame = px.Graphics.new(null);
    _ = frame.rect(32.0, 20.0, 616.0, 360.0);
    const ss = px.StrokeStyle.create();
    ss.asStrokeAttributes().setWidth(1.5);
    ss.asFillStyle().setColor(.{ .number = 0xd4d4d4 });
    const sargs = px.JsArray.new();
    sargs.push(.{ .h = ss.h });
    _ = frame.stroke(sargs);
    _ = stage.asPixiMixins_Container().addChild(
        px.JsArrayOf(px.Container).of(&.{frame.asViewContainer().asContainer()}),
    );

    const shades = [_]f64{
        0x111111, 0x262626, 0x404040, 0x555555,
        0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5,
    };
    const white = px.Texture.getWHITE();

    var row: usize = 0;
    while (row < 6) : (row += 1) {
        var col: usize = 0;
        while (col < 10) : (col += 1) {
            const s = px.Sprite.new(.{ .Texture = white });
            const idx = (row * 3 + col * 5) % shades.len;
            const sc = s.asViewContainer().asContainer();
            sc.asPixiMixins_Container().setWidth(52.0);
            sc.asPixiMixins_Container().setHeight(48.0);
            _ = s.getAnchor().set(.{ .x = 0.5, .y = 0.5 });
            sc.setX(70.0 + @as(f64, @floatFromInt(col)) * 60.0);
            sc.setY(56.0 + @as(f64, @floatFromInt(row)) * 56.0);
            sc.setRotation((@as(f64, @floatFromInt(col)) - 4.5) * 0.04 + (@as(f64, @floatFromInt(row)) - 2.5) * 0.03);
            sc.setAlpha(0.65 + 0.35 * @as(f64, @floatFromInt((col + row) % 3)) / 2.0);
            sc.setTint(shades[idx]);
            _ = stage.asPixiMixins_Container().addChild(
                px.JsArrayOf(px.Container).of(&.{sc}),
            );
        }
    }

    g_app.render();

    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent("60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL");
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
