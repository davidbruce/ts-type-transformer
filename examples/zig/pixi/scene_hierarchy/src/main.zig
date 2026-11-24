const std = @import("std");
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

    const guide = px.Graphics.new(null);
    _ = guide.moveTo(40.0, 200.0).lineTo(640.0, 200.0).moveTo(340.0, 24.0).lineTo(340.0, 376.0);
    strokeG(guide, 1.0, 0xe5e5e5);
    _ = guide.circle(340.0, 200.0, 110.0);
    strokeG(guide, 1.0, 0xd4d4d4);
    _ = stage.addChild(.{guide});

    const root = px.Container.new(null);
    root.setX(340.0);
    root.setY(200.0);

    for (0..3) |i| {
        const arm = px.Container.new(null);
        arm.setRotation(@as(f64, @floatFromInt(i)) * 2.0 * std.math.pi / 3.0);

        const arm_bar = px.Graphics.new(null);
        _ = arm_bar.rect(0.0, -4.0, 110.0, 8.0).fillWithColorAndAlpha(0x737373, null);
        _ = arm_bar.circle(110.0, 0.0, 10.0).fillWithColorAndAlpha(0x111111, null);

        const sub = px.Container.new(null);
        sub.setX(110.0);
        sub.setRotation(-0.45);

        const sub_bar = px.Graphics.new(null);
        _ = sub_bar.moveTo(-32.0, 0.0).lineTo(48.0, 0.0);
        strokeG(sub_bar, 2.0, 0x404040);

        const sat_box = px.Graphics.new(null);
        _ = sat_box.rect(-12.0, -12.0, 24.0, 24.0).fillWithColorAndAlpha(0x262626, null);
        sat_box.setX(48.0);

        const sat_orb = px.Graphics.new(null);
        _ = sat_orb.circle(0.0, 0.0, 14.0).fillWithColorAndAlpha(0xd4d4d4, null);
        strokeG(sat_orb, 2.0, 0x111111);
        sat_orb.setX(-32.0);

        _ = sub.addChild(.{ sub_bar, sat_box, sat_orb });
        _ = arm.addChild(.{ arm_bar, sub });
        _ = root.addChild(.{arm});
    }

    const hub = px.Graphics.new(null);
    _ = hub.circle(0.0, 0.0, 18.0).fillWithColorAndAlpha(0x111111, null);
    _ = hub.circle(0.0, 0.0, 7.0).fillWithColorAndAlpha(0xffffff, null);
    _ = root.addChild(.{hub});

    _ = stage.addChild(.{root});
    app.render();

    const rb = root.getBounds(null).getRectangle();
    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "3 primary arms · 6 nested satellites · bounds {d:.1}x{d:.1} · 680x400 WebGL",
        .{ rb.getWidth(), rb.getHeight() },
    ) catch "3 primary arms · 6 nested satellites · 680x400 WebGL";

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
