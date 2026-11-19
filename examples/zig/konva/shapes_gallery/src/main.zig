const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

fn addCaption(layer: kv.Layer, x: f64, y: f64, label: []const u8) void {
    const tc = kv.TextConfig.create();
    tc.setX(x);
    tc.setY(y);
    tc.setText(label);
    tc.setFontSize(11);
    tc.setFontFamily("monospace");
    tc.setFill(.{ .string = kv.JsString.from("#555555") });
    const t = kv.Text.new(tc);
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = t.asShape() }}));
}

export fn start() void {
    const sc = kv.StageConfig.create();
    sc.setContainer(.{ .string = kv.JsString.from("stage") });
    sc.setWidth(680);
    sc.setHeight(400);
    const stage = kv.Stage.new(sc);

    const layer = kv.Layer.new(null);
    _ = stage.add(layer, kv.JsArray.new());

    const bg = kv.RectConfig.create();
    bg.setX(0);
    bg.setY(0);
    bg.setWidth(680);
    bg.setHeight(400);
    bg.setFill(.{ .string = kv.JsString.from("#ffffff") });
    const bg_rect = kv.Rect.new(bg);

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    const r1 = kv.RectConfig.create();
    r1.setX(40);
    r1.setY(55);
    r1.setWidth(110);
    r1.setHeight(76);
    r1.setCornerRadius(.{ .number = 8 });
    r1.setFill(.{ .string = kv.JsString.from("#111111") });
    const rect1 = kv.Rect.new(r1);

    const dash = kv.JsArrayOf(f64).of(&.{ 6, 4 });
    const r2 = kv.RectConfig.create();
    r2.setX(195);
    r2.setY(55);
    r2.setWidth(110);
    r2.setHeight(76);
    r2.setFill(.{ .string = kv.JsString.from("#ffffff") });
    r2.setStroke(.{ .string = kv.JsString.from("#111111") });
    r2.setStrokeWidth(2);
    r2.setDash(dash);
    const rect2 = kv.Rect.new(r2);

    const cc = kv.CircleConfig.create();
    cc.setX(415);
    cc.setY(93);
    cc.setRadius(40);
    cc.setFill(.{ .string = kv.JsString.from("#404040") });
    cc.setStroke(.{ .string = kv.JsString.from("#111111") });
    cc.setStrokeWidth(2);
    const circle = kv.Circle.new(cc);

    const pc = kv.RegularPolygonConfig.create();
    pc.setX(575);
    pc.setY(93);
    pc.setSides(6);
    pc.setRadius(42);
    pc.setFill(.{ .string = kv.JsString.from("#efefef") });
    pc.setStroke(.{ .string = kv.JsString.from("#111111") });
    pc.setStrokeWidth(2);
    const hex = kv.RegularPolygon.new(pc);

    // Row 2: Star, Ring, Wedge, Framed Text.
    const st = kv.StarConfig.create();
    st.setX(95);
    st.setY(268);
    st.setNumPoints(5);
    st.setInnerRadius(19);
    st.setOuterRadius(44);
    st.setFill(.{ .string = kv.JsString.from("#737373") });
    st.setStroke(.{ .string = kv.JsString.from("#111111") });
    st.setStrokeWidth(2);
    const star = kv.Star.new(st);

    const rc = kv.RingConfig.create();
    rc.setX(250);
    rc.setY(268);
    rc.setInnerRadius(22);
    rc.setOuterRadius(42);
    rc.setFill(.{ .string = kv.JsString.from("#d4d4d4") });
    rc.setStroke(.{ .string = kv.JsString.from("#111111") });
    rc.setStrokeWidth(2);
    const ring = kv.Ring.new(rc);

    const wc = kv.WedgeConfig.create();
    wc.setX(415);
    wc.setY(268);
    wc.setRadius(44);
    wc.setAngle(270);
    wc.setRotation(-90);
    wc.setFill(.{ .string = kv.JsString.from("#111111") });
    wc.setStroke(.{ .string = kv.JsString.from("#555555") });
    wc.setStrokeWidth(1);
    const wedge = kv.Wedge.new(wc);

    const tf = kv.RectConfig.create();
    tf.setX(500);
    tf.setY(224);
    tf.setWidth(150);
    tf.setHeight(88);
    tf.setFill(.{ .string = kv.JsString.from("#f7f7f7") });
    tf.setStroke(.{ .string = kv.JsString.from("#111111") });
    tf.setStrokeWidth(1.5);
    const tf_rect = kv.Rect.new(tf);

    const tc = kv.TextConfig.create();
    tc.setX(514);
    tc.setY(240);
    tc.setText("KONVA 2D\nMonochrome\nScene Graph");
    tc.setFontSize(13);
    tc.setFontFamily("monospace");
    tc.setFill(.{ .string = kv.JsString.from("#111111") });
    const txt = kv.Text.new(tc);

    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = bg_rect.asShape() },
        .{ .Shape = rect1.asShape() },
        .{ .Shape = rect2.asShape() },
        .{ .Shape = circle.asShape() },
        .{ .Shape = hex.asShape() },
        .{ .Shape = star.asShape() },
        .{ .Shape = ring.asShape() },
        .{ .Shape = wedge.asShape() },
        .{ .Shape = tf_rect.asShape() },
        .{ .Shape = txt.asShape() },
    }));

    addCaption(layer, 40, 142, "Rect (filled)");
    addCaption(layer, 195, 142, "Rect (dashed)");
    addCaption(layer, 395, 142, "Circle");
    addCaption(layer, 548, 142, "Hexagon");
    addCaption(layer, 62, 325, "Star (5pt)");
    addCaption(layer, 236, 325, "Ring");
    addCaption(layer, 382, 325, "Wedge 270\xC2\xB0");
    addCaption(layer, 558, 325, "Text");

    _ = layer.asContainer().asNode().draw();

    const doc = dom.getDocument();
    const meta_el = doc.getElementById("meta").?;
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 {d} layer children", .{layer.getChildren().len()}) catch "",
    );
}
