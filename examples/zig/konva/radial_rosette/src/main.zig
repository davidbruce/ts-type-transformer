const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

export fn start() void {
    const cx: f64 = 340;
    const cy: f64 = 200;

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
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Rect.new(bg).asShape() }}));

    const ring_r = [_]f64{ 160, 125, 90, 52 };
    const ring_stroke = [_][]const u8{ "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const ring_sw = [_]f64{ 1, 1, 1, 1.5 };
    for (ring_r, ring_stroke, ring_sw, 0..) |r, col, sw, idx| {
        const cc = kv.CircleConfig.create();
        cc.setX(cx);
        cc.setY(cy);
        cc.setRadius(r);
        cc.setStroke(.{ .string = kv.JsString.from(col) });
        cc.setStrokeWidth(sw);
        if (idx == 3) cc.setFill(.{ .string = kv.JsString.from("#f7f7f7") });
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Circle.new(cc).asShape() }}));
    }

    for (0..12) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 6.0;
        const lc = kv.LineConfig.create();
        lc.setPoints(.{ .array = kv.JsArrayOf(f64).of(&.{
            cx + 36.0 * @cos(a),  cy + 36.0 * @sin(a),
            cx + 160.0 * @cos(a), cy + 160.0 * @sin(a),
        }) });
        lc.setStroke(.{ .string = kv.JsString.from("#d4d4d4") });
        lc.setStrokeWidth(1);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Line.new(lc).asShape() }}));
    }

    for (0..24) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 12.0;
        const r0: f64 = if (i % 2 == 0) 148.0 else 154.0;
        const lc = kv.LineConfig.create();
        lc.setPoints(.{ .array = kv.JsArrayOf(f64).of(&.{
            cx + r0 * @cos(a),    cy + r0 * @sin(a),
            cx + 166.0 * @cos(a), cy + 166.0 * @sin(a),
        }) });
        lc.setStroke(.{ .string = kv.JsString.from("#111111") });
        lc.setStrokeWidth(if (i % 2 == 0) 2 else 1);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Line.new(lc).asShape() }}));
    }

    for (0..8) |i| {
        const pc = kv.RectConfig.create();
        pc.setX(cx);
        pc.setY(cy);
        pc.setWidth(18);
        pc.setHeight(52);
        pc.setOffsetX(9);
        pc.setOffsetY(108);
        pc.setRotation(@as(f64, @floatFromInt(i)) * 45.0);
        pc.setFill(.{ .string = kv.JsString.from(if (i % 2 == 0) "#111111" else "#efefef") });
        pc.setStroke(.{ .string = kv.JsString.from("#111111") });
        pc.setStrokeWidth(1.5);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Rect.new(pc).asShape() }}));
    }

    for (0..12) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 6.0;
        const oc = kv.CircleConfig.create();
        oc.setX(cx + 125.0 * @cos(a));
        oc.setY(cy + 125.0 * @sin(a));
        oc.setRadius(5);
        oc.setFill(.{ .string = kv.JsString.from(if (i % 2 == 0) "#111111" else "#ffffff") });
        oc.setStroke(.{ .string = kv.JsString.from("#111111") });
        oc.setStrokeWidth(1.5);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Circle.new(oc).asShape() }}));
    }

    const cp = kv.RegularPolygonConfig.create();
    cp.setX(cx);
    cp.setY(cy);
    cp.setSides(8);
    cp.setRadius(26);
    cp.setFill(.{ .string = kv.JsString.from("#111111") });
    const cd = kv.CircleConfig.create();
    cd.setX(cx);
    cd.setY(cy);
    cd.setRadius(9);
    cd.setFill(.{ .string = kv.JsString.from("#ffffff") });
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.RegularPolygon.new(cp).asShape() },
        .{ .Shape = kv.Circle.new(cd).asShape() },
    }));

    const lx = [_]f64{ 516, 332, 140, 328 };
    const ly = [_]f64{ 194, 376, 194, 14 };
    const labels = [_][]const u8{ "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (lx, ly, labels) |x, y, txt| {
        const tc = kv.TextConfig.create();
        tc.setX(x);
        tc.setY(y);
        tc.setText(txt);
        tc.setFontSize(11);
        tc.setFontFamily("monospace");
        tc.setFill(.{ .string = kv.JsString.from("#555555") });
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Text.new(tc).asShape() }}));
    }

    _ = layer.asContainer().asNode().draw();

    const doc = dom.getDocument();
    const meta_el = doc.getElementById("meta").?;
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 {d} shapes on layer", .{layer.getChildren().len()}) catch "",
    );
}
