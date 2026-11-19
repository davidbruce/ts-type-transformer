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

fn addChildC(parent: px.Container, child: px.Container) void {
    _ = parent.asPixiMixins_Container().addChild(px.JsArrayOf(px.Container).of(&.{child}));
}

fn onInit(_: ?*anyopaque, ok: bool, _: px.JsValue) void {
    if (!ok) return;

    const doc = dom.getDocument();
    if (doc.getElementById("stage")) |stage_el| {
        _ = stage_el.asNode().appendChild(.{ .h = g_app.getCanvas().h });
    }

    const stage = g_app.getStage();

    const guide = px.Graphics.new(null);
    _ = guide.moveTo(40.0, 200.0).lineTo(640.0, 200.0).moveTo(340.0, 24.0).lineTo(340.0, 376.0);
    strokeG(guide, 1.0, 0xe5e5e5);
    _ = guide.circle(340.0, 200.0, 110.0);
    strokeG(guide, 1.0, 0xd4d4d4);
    addChildC(stage, guide.asViewContainer().asContainer());

    const root = px.Container.new(null);
    root.setX(340.0);
    root.setY(200.0);

    var i: usize = 0;
    while (i < 3) : (i += 1) {
        const arm = px.Container.new(null);
        arm.setRotation(@as(f64, @floatFromInt(i)) * 2.0 * std.math.pi / 3.0);

        const arm_bar = px.Graphics.new(null);
        _ = arm_bar.rect(0.0, -4.0, 110.0, 8.0).fillWithColorAndAlpha(.{ .number = 0x737373 }, .{});
        _ = arm_bar.circle(110.0, 0.0, 10.0).fillWithColorAndAlpha(.{ .number = 0x111111 }, .{});
        addChildC(arm, arm_bar.asViewContainer().asContainer());

        const sub = px.Container.new(null);
        sub.setX(110.0);
        sub.setRotation(-0.45);

        const sub_bar = px.Graphics.new(null);
        _ = sub_bar.moveTo(-32.0, 0.0).lineTo(48.0, 0.0);
        strokeG(sub_bar, 2.0, 0x404040);
        addChildC(sub, sub_bar.asViewContainer().asContainer());

        const sat_box = px.Graphics.new(null);
        _ = sat_box.rect(-12.0, -12.0, 24.0, 24.0).fillWithColorAndAlpha(.{ .number = 0x262626 }, .{});
        const sat_box_c = sat_box.asViewContainer().asContainer();
        sat_box_c.setX(48.0);
        addChildC(sub, sat_box_c);

        const sat_orb = px.Graphics.new(null);
        _ = sat_orb.circle(0.0, 0.0, 14.0).fillWithColorAndAlpha(.{ .number = 0xd4d4d4 }, .{});
        strokeG(sat_orb, 2.0, 0x111111);
        const sat_orb_c = sat_orb.asViewContainer().asContainer();
        sat_orb_c.setX(-32.0);
        addChildC(sub, sat_orb_c);

        addChildC(arm, sub);
        addChildC(root, arm);
    }

    const hub = px.Graphics.new(null);
    _ = hub.circle(0.0, 0.0, 18.0).fillWithColorAndAlpha(.{ .number = 0x111111 }, .{});
    _ = hub.circle(0.0, 0.0, 7.0).fillWithColorAndAlpha(.{ .number = 0xffffff }, .{});
    addChildC(root, hub.asViewContainer().asContainer());

    addChildC(stage, root);
    g_app.render();

    const rb = root.asPixiMixins_Container().getBounds(.{}).getRectangle();
    var buf: [160]u8 = undefined;
    const msg = std.fmt.bufPrint(
        &buf,
        "3 primary arms · 6 nested satellites · bounds {d:.1}x{d:.1} · 680x400 WebGL",
        .{ rb.getWidth(), rb.getHeight() },
    ) catch "3 primary arms · 6 nested satellites · 680x400 WebGL";

    if (doc.getElementById("meta")) |meta| {
        meta.asNode().setTextContent(msg);
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
