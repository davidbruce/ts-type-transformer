const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [192]u8 = undefined;
var id_buf: [64]u8 = undefined;
var clicks: u32 = 0;

fn onCardClick(_: ?*anyopaque, _: kv.KonvaEventObject) void {
    clicks += 1;
}

fn makeLine(pts: []const f64, col: []const u8, w: f64) kv.Shape {
    const lc = kv.LineConfig.create();
    lc.setPoints(.{ .array = kv.JsArrayOf(f64).of(pts) });
    lc.setStroke(.{ .string = kv.JsString.from(col) });
    lc.setStrokeWidth(w);
    return kv.Line.new(lc).asShape();
}

fn makeCard(id: []const u8, x: f64, y: f64, fill: []const u8) kv.Rect {
    const rc = kv.RectConfig.create();
    rc.setId(id);
    rc.setName("card");
    rc.setX(x);
    rc.setY(y);
    rc.setWidth(200);
    rc.setHeight(130);
    rc.setFill(.{ .string = kv.JsString.from(fill) });
    rc.setStroke(.{ .string = kv.JsString.from("#111111") });
    rc.setStrokeWidth(2);
    return kv.Rect.new(rc);
}

export fn start() void {
    const sc = kv.StageConfig.create();
    sc.setContainer(.{ .string = kv.JsString.from("stage") });
    sc.setWidth(680);
    sc.setHeight(400);
    const stage = kv.Stage.new(sc);

    const bg_layer = kv.Layer.new(null);
    const fg_layer = kv.Layer.new(null);
    _ = stage.add(bg_layer, kv.JsArray.new());
    _ = stage.add(fg_layer, kv.JsArray.new());

    // Layer 1: Blueprint Grid.
    const bgr = kv.RectConfig.create();
    bgr.setX(0);
    bgr.setY(0);
    bgr.setWidth(680);
    bgr.setHeight(400);
    bgr.setFill(.{ .string = kv.JsString.from("#ffffff") });
    _ = bg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = kv.Rect.new(bgr).asShape() }}));

    var x: i32 = 40;
    while (x <= 640) : (x += 40) {
        const xf: f64 = @floatFromInt(x);
        _ = bg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = makeLine(&.{ xf, 20, xf, 380 }, "#e5e5e5", 1) }}));
    }
    var y: i32 = 40;
    while (y <= 360) : (y += 40) {
        const yf: f64 = @floatFromInt(y);
        _ = bg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{.{ .Shape = makeLine(&.{ 20, yf, 660, yf }, "#e5e5e5", 1) }}));
    }

    const frame = kv.RectConfig.create();
    frame.setX(20);
    frame.setY(20);
    frame.setWidth(640);
    frame.setHeight(360);
    frame.setStroke(.{ .string = kv.JsString.from("#b5b5b5") });
    frame.setStrokeWidth(1.5);
    frame.setDash(kv.JsArrayOf(f64).of(&.{ 6, 4 }));
    const l1 = kv.TextConfig.create();
    l1.setX(32);
    l1.setY(32);
    l1.setText("Layer 1: Blueprint Grid (z=0)");
    l1.setFontSize(12);
    l1.setFontFamily("monospace");
    l1.setFill(.{ .string = kv.JsString.from("#737373") });
    _ = bg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Rect.new(frame).asShape() },
        .{ .Shape = kv.Text.new(l1).asShape() },
    }));

    // Layer 2: Stacked cards & badge.
    const card_a = makeCard("card-a", 110, 80, "#efefef");
    const card_b = makeCard("card-b", 210, 125, "#404040");
    const card_c = makeCard("card-c", 310, 170, "#ffffff");
    const bc = kv.CircleConfig.create();
    bc.setId("badge");
    bc.setX(555);
    bc.setY(135);
    bc.setRadius(38);
    bc.setFill(.{ .string = kv.JsString.from("#111111") });
    const badge = kv.Circle.new(bc);
    _ = fg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = card_a.asShape() },
        .{ .Shape = card_b.asShape() },
        .{ .Shape = card_c.asShape() },
        .{ .Shape = badge.asShape() },
    }));

    const card_a_node = card_a.asShape().asNode();
    _ = card_a_node.moveToTop();
    const card_a_z: i32 = @intFromFloat(card_a_node.getZIndex());

    const listener = kv.KonvaEventListener.from(onCardClick, null);
    _ = card_a_node.on(.{ .string = kv.JsString.from("click") }, listener);
    _ = card_a_node.fire("click", .{});
    _ = card_a_node.fire("click", .{});

    _ = bg_layer.asContainer().asNode().draw();
    _ = fg_layer.asContainer().asNode().draw();

    const pt = kv.Vector2d.create();
    pt.setX(250);
    pt.setY(160);
    const hit = stage.getIntersection(pt);
    const hit_id: []const u8 = if (hit) |s| s.asNode().getId().invoke().toString().copy(&id_buf) else "none";

    const dc = kv.CircleConfig.create();
    dc.setX(250);
    dc.setY(160);
    dc.setRadius(7);
    dc.setFill(.{ .string = kv.JsString.from("#ffffff") });
    dc.setStroke(.{ .string = kv.JsString.from("#111111") });
    dc.setStrokeWidth(2);
    const l2 = kv.TextConfig.create();
    l2.setX(32);
    l2.setY(354);
    l2.setText("Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)");
    l2.setFontSize(12);
    l2.setFontFamily("monospace");
    l2.setFill(.{ .string = kv.JsString.from("#111111") });
    _ = fg_layer.add(kv.JsArrayOf(kv.Group_or_Shape).of(&.{
        .{ .Shape = kv.Circle.new(dc).asShape() },
        .{ .Shape = makeLine(&.{ 238, 160, 262, 160 }, "#111111", 1.5) },
        .{ .Shape = makeLine(&.{ 250, 148, 250, 172 }, "#111111", 1.5) },
        .{ .Shape = kv.Text.new(l2).asShape() },
    }));
    _ = fg_layer.asContainer().asNode().draw();

    const doc = dom.getDocument();
    const meta_el = doc.getElementById("meta").?;
    meta_el.asNode().setTextContent(
        std.fmt.bufPrint(&meta_buf, "Layers: {d} \xC2\xB7 card-a zIndex={d} \xC2\xB7 hit@(250,160)={s} \xC2\xB7 clicks={d}", .{
            stage.getLayers().len(), card_a_z, hit_id, clicks,
        }) catch "",
    );
}
