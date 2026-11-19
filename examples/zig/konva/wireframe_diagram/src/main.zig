const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

fn addStageBox(layer: kv.Layer, x: f64, title: []const u8, sub: []const u8, fill: []const u8, ink: []const u8) void {
    const gc = kv.GroupConfig.create();
    gc.setX(x);
    gc.setY(95);
    const g = kv.Group.new(gc);

    const rc = kv.RectConfig.create();
    rc.setWidth(126);
    rc.setHeight(68);
    rc.setCornerRadius(.{ .number = 3 });
    rc.setFill(.{ .string = kv.JsString.from(fill) });
    rc.setStroke(.{ .string = kv.JsString.from("#111111") });
    rc.setStrokeWidth(2);

    const t1 = kv.TextConfig.create();
    t1.setX(12);
    t1.setY(16);
    t1.setText(title);
    t1.setFontSize(12);
    t1.setFontStyle("bold");
    t1.setFontFamily("monospace");
    t1.setFill(.{ .string = kv.JsString.from(ink) });

    const t2 = kv.TextConfig.create();
    t2.setX(12);
    t2.setY(38);
    t2.setText(sub);
    t2.setFontSize(11);
    t2.setFontFamily("monospace");
    t2.setFill(.{ .string = kv.JsString.from(ink) });

    _ = g.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(rc).asShape() },
        .{ .Shape = kv.Text.new(t1).asShape() },
        .{ .Shape = kv.Text.new(t2).asShape() },
    }));
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Group = g }}));
}

fn addArrow(layer: kv.Layer, x1: f64, y1: f64, x2: f64, y2: f64) void {
    const ac = kv.ArrowConfig.create();
    ac.setPoints(.{ .array = kv.JsArrayOf(f64).of(&.{ x1, y1, x2, y2 }) });
    ac.setPointerLength(8);
    ac.setPointerWidth(8);
    ac.setFill(.{ .string = kv.JsString.from("#111111") });
    ac.setStroke(.{ .string = kv.JsString.from("#111111") });
    ac.setStrokeWidth(2);
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Arrow.new(ac).asLine().asShape() }}));
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
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Rect.new(bg).asShape() }}));

    addStageBox(layer, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    addStageBox(layer, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    addStageBox(layer, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    addStageBox(layer, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    addArrow(layer, 154, 129, 190, 129);
    addArrow(layer, 320, 129, 356, 129);
    addArrow(layer, 486, 129, 522, 129);
    addArrow(layer, 257, 163, 257, 251);
    addArrow(layer, 423, 163, 423, 251);

    const fbc = kv.LineConfig.create();
    fbc.setPoints(.{ .array = kv.JsArrayOf(f64).of(&.{ 589, 163, 589, 212, 91, 212, 91, 168 }) });
    fbc.setStroke(.{ .string = kv.JsString.from("#737373") });
    fbc.setStrokeWidth(1.5);
    fbc.setDash(kv.JsArrayOf(f64).of(&.{ 6, 4 }));

    const bar = kv.RectConfig.create();
    bar.setX(28);
    bar.setY(255);
    bar.setWidth(624);
    bar.setHeight(58);
    bar.setCornerRadius(.{ .number = 3 });
    bar.setFill(.{ .string = kv.JsString.from("#f7f7f7") });
    bar.setStroke(.{ .string = kv.JsString.from("#111111") });
    bar.setStrokeWidth(1.5);

    const btxt = kv.TextConfig.create();
    btxt.setX(180);
    btxt.setY(278);
    btxt.setText("Shared DOM & Package Handle Runtime");
    btxt.setFontSize(12);
    btxt.setFontFamily("monospace");
    btxt.setFill(.{ .string = kv.JsString.from("#111111") });

    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Line.new(fbc).asShape() },
        .{ .Shape = kv.Rect.new(bar).asShape() },
        .{ .Shape = kv.Text.new(btxt).asShape() },
    }));

    _ = layer.asContainer().asNode().draw();

    const rects = layer.asContainer().find(kv.JsValue.string("Rect")).len();
    const arrows = layer.asContainer().find(kv.JsValue.string("Arrow")).len();
    const jlen = layer.asContainer().asNode().toJSON().len();

    const doc = dom.getDocument();
    const meta_el = doc.getElementById("meta").?;
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Pipeline diagram \xC2\xB7 {d} rects \xC2\xB7 {d} arrows \xC2\xB7 JSON {d} bytes", .{ rects, arrows, jlen }) catch "",
    );
}
