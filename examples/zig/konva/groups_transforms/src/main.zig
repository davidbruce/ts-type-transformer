const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [192]u8 = undefined;
var strip_buf: [160]u8 = undefined;

fn makeLine(pts: []const f64) kv.Shape {
    const lc = kv.LineConfig.create();
    lc.setPoints(.{ .array = kv.JsArrayOf(f64).of(pts) });
    lc.setStroke(.{ .string = kv.JsString.from("#e5e5e5") });
    lc.setStrokeWidth(1);
    return kv.Line.new(lc).asShape();
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

    var x: i32 = 40;
    while (x <= 640) : (x += 40) {
        const xf: f64 = @floatFromInt(x);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = makeLine(&.{ xf, 0, xf, 400 }) }}));
    }
    var y: i32 = 40;
    while (y <= 360) : (y += 40) {
        const yf: f64 = @floatFromInt(y);
        _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = makeLine(&.{ 0, yf, 680, yf }) }}));
    }

    // Group A.
    const gac = kv.GroupConfig.create();
    gac.setX(40);
    gac.setY(55);
    const group_a = kv.Group.new(gac);
    const gab = kv.RectConfig.create();
    gab.setWidth(160);
    gab.setHeight(120);
    gab.setFill(.{ .string = kv.JsString.from("#f7f7f7") });
    gab.setStroke(.{ .string = kv.JsString.from("#111111") });
    gab.setStrokeWidth(1.5);
    const gad = kv.CircleConfig.create();
    gad.setX(80);
    gad.setY(60);
    gad.setRadius(24);
    gad.setFill(.{ .string = kv.JsString.from("#111111") });
    const gal = kv.TextConfig.create();
    gal.setX(12);
    gal.setY(10);
    gal.setText("Group A \xC2\xB7 0\xC2\xB0");
    gal.setFontSize(11);
    gal.setFontFamily("monospace");
    gal.setFill(.{ .string = kv.JsString.from("#111111") });
    _ = group_a.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(gab).asShape() },
        .{ .Shape = kv.Circle.new(gad).asShape() },
        .{ .Shape = kv.Text.new(gal).asShape() },
    }));
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Group = group_a }}));

    // Group B.
    const gbc = kv.GroupConfig.create();
    gbc.setX(340);
    gbc.setY(75);
    gbc.setRotation(24);
    const group_b = kv.Group.new(gbc);
    const gbo = kv.RectConfig.create();
    gbo.setWidth(150);
    gbo.setHeight(110);
    gbo.setFill(.{ .string = kv.JsString.from("#efefef") });
    gbo.setStroke(.{ .string = kv.JsString.from("#111111") });
    gbo.setStrokeWidth(2);
    const gbi = kv.RectConfig.create();
    gbi.setX(22);
    gbi.setY(22);
    gbi.setWidth(106);
    gbi.setHeight(66);
    gbi.setFill(.{ .string = kv.JsString.from("#ffffff") });
    gbi.setStroke(.{ .string = kv.JsString.from("#555555") });
    gbi.setStrokeWidth(1.5);
    gbi.setDash(kv.JsArrayOf(f64).of(&.{ 4, 3 }));
    const gbd = kv.CircleConfig.create();
    gbd.setX(75);
    gbd.setY(55);
    gbd.setRadius(14);
    gbd.setFill(.{ .string = kv.JsString.from("#404040") });
    const gbl = kv.TextConfig.create();
    gbl.setX(10);
    gbl.setY(8);
    gbl.setText("Group B \xC2\xB7 24\xC2\xB0");
    gbl.setFontSize(11);
    gbl.setFontFamily("monospace");
    gbl.setFill(.{ .string = kv.JsString.from("#111111") });
    _ = group_b.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(gbo).asShape() },
        .{ .Shape = kv.Rect.new(gbi).asShape() },
        .{ .Shape = kv.Circle.new(gbd).asShape() },
        .{ .Shape = kv.Text.new(gbl).asShape() },
    }));
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Group = group_b }}));

    // Group C + nested inner group.
    const gcc = kv.GroupConfig.create();
    gcc.setX(495);
    gcc.setY(55);
    gcc.setScaleX(1.15);
    gcc.setScaleY(1.15);
    const group_c = kv.Group.new(gcc);
    const gco = kv.RectConfig.create();
    gco.setWidth(130);
    gco.setHeight(110);
    gco.setFill(.{ .string = kv.JsString.from("#ffffff") });
    gco.setStroke(.{ .string = kv.JsString.from("#111111") });
    gco.setStrokeWidth(2);
    const gcl = kv.TextConfig.create();
    gcl.setX(8);
    gcl.setY(8);
    gcl.setText("Group C \xC2\xB7 1.15x");
    gcl.setFontSize(10);
    gcl.setFontFamily("monospace");
    gcl.setFill(.{ .string = kv.JsString.from("#111111") });

    const icc = kv.GroupConfig.create();
    icc.setX(25);
    icc.setY(25);
    icc.setRotation(15);
    const inner_group = kv.Group.new(icc);
    const mc = kv.CircleConfig.create();
    mc.setX(40);
    mc.setY(30);
    mc.setRadius(16);
    mc.setFill(.{ .string = kv.JsString.from("#111111") });
    const marker = kv.Circle.new(mc);
    _ = inner_group.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = marker.asShape() }}));
    _ = group_c.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(gco).asShape() },
        .{ .Shape = kv.Text.new(gcl).asShape() },
        .{ .Group = inner_group },
    }));
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Group = group_c }}));

    const abs = marker.asShape().asNode().getAbsolutePosition(null);
    const bbox = group_b.asContainer().asNode().getClientRect(null);
    const ax: i32 = @intFromFloat(@round(abs.getX()));
    const ay: i32 = @intFromFloat(@round(abs.getY()));
    const bw: i32 = @intFromFloat(@round(bbox.getWidth()));
    const bh: i32 = @intFromFloat(@round(bbox.getHeight()));
    const strip_txt = std.fmt.bufPrint(&strip_buf, "Marker abs=({d}, {d}) \xC2\xB7 Group B bbox={d}x{d}", .{ ax, ay, bw, bh }) catch "";

    const strip = kv.RectConfig.create();
    strip.setX(28);
    strip.setY(308);
    strip.setWidth(624);
    strip.setHeight(60);
    strip.setFill(.{ .string = kv.JsString.from("#f7f7f7") });
    strip.setStroke(.{ .string = kv.JsString.from("#111111") });
    strip.setStrokeWidth(1.5);
    const sum = kv.TextConfig.create();
    sum.setX(44);
    sum.setY(331);
    sum.setText(strip_txt);
    sum.setFontSize(12);
    sum.setFontFamily("monospace");
    sum.setFill(.{ .string = kv.JsString.from("#111111") });
    _ = layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(strip).asShape() },
        .{ .Shape = kv.Text.new(sum).asShape() },
    }));

    _ = layer.asContainer().asNode().draw();

    const doc = dom.getDocument();
    const meta_el = doc.getElementById("meta").?;
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 Nested marker abs=({d}, {d}) \xC2\xB7 Rotated Group B rect={d}x{d}", .{ ax, ay, bw, bh }) catch "",
    );
}
