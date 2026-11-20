const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [192]u8 = undefined;
var id_buf: [64]u8 = undefined;

fn onCardClick(clicks: *u32, _: kv.KonvaEventObject) void {
    clicks.* += 1;
}

fn makeLine(pts: []const f64, col: []const u8, w: f64) kv.Line {
    return kv.Line.new(.{ .points = pts, .stroke = col, .strokeWidth = w });
}

fn makeCard(id: []const u8, x: f64, y: f64, fill: []const u8) kv.Rect {
    return kv.Rect.new(.{
        .id = id,
        .name = "card",
        .x = x,
        .y = y,
        .width = 200,
        .height = 130,
        .fill = fill,
        .stroke = "#111111",
        .strokeWidth = 2,
    });
}

export fn start() void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    var clicks: u32 = 0;

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const bg_layer = kv.Layer.new(null);
    const fg_layer = kv.Layer.new(null);
    _ = stage.add(bg_layer, &.{});
    _ = stage.add(fg_layer, &.{});

    // Layer 1: Blueprint Grid.
    _ = bg_layer.add(.{kv.Rect.new(.{ .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff" })});

    var x: i32 = 40;
    while (x <= 640) : (x += 40) {
        const xf: f64 = @floatFromInt(x);
        _ = bg_layer.add(.{makeLine(&.{ xf, 20, xf, 380 }, "#e5e5e5", 1)});
    }
    var y: i32 = 40;
    while (y <= 360) : (y += 40) {
        const yf: f64 = @floatFromInt(y);
        _ = bg_layer.add(.{makeLine(&.{ 20, yf, 660, yf }, "#e5e5e5", 1)});
    }

    _ = bg_layer.add(.{
        kv.Rect.new(.{ .x = 20, .y = 20, .width = 640, .height = 360, .stroke = "#b5b5b5", .strokeWidth = 1.5, .dash = &.{ 6, 4 } }),
        kv.Text.new(.{ .x = 32, .y = 32, .text = "Layer 1: Blueprint Grid (z=0)", .fontSize = 12, .fontFamily = "monospace", .fill = "#737373" }),
    });

    // Layer 2: Stacked cards & badge.
    const card_a = makeCard("card-a", 110, 80, "#efefef");
    const card_b = makeCard("card-b", 210, 125, "#404040");
    const card_c = makeCard("card-c", 310, 170, "#ffffff");
    const badge = kv.Circle.new(.{ .id = "badge", .x = 555, .y = 135, .radius = 38, .fill = "#111111" });
    _ = fg_layer.add(.{ card_a, card_b, card_c, badge });

    _ = card_a.moveToTop();
    const card_a_z: i32 = @intFromFloat(card_a.getZIndex());

    _ = card_a.on("click", .{ onCardClick, &clicks });
    _ = card_a.fire("click", null);
    _ = card_a.fire("click", null);

    _ = bg_layer.draw();
    _ = fg_layer.draw();

    const hit_id: []const u8 = if (stage.getIntersection(.{ .x = 250, .y = 160 })) |s| s.getId().invoke().toString().copy(&id_buf) else "none";

    _ = fg_layer.add(.{
        kv.Circle.new(.{ .x = 250, .y = 160, .radius = 7, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2 }),
        makeLine(&.{ 238, 160, 262, 160 }, "#111111", 1.5),
        makeLine(&.{ 250, 148, 250, 172 }, "#111111", 1.5),
        kv.Text.new(.{ .x = 32, .y = 354, .text = "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)", .fontSize = 12, .fontFamily = "monospace", .fill = "#111111" }),
    });
    _ = fg_layer.draw();

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Layers: {d} \xC2\xB7 card-a zIndex={d} \xC2\xB7 hit@(250,160)={s} \xC2\xB7 clicks={d}", .{
                stage.getLayers().len(), card_a_z, hit_id, clicks,
            }) catch "",
        );
    }
}
