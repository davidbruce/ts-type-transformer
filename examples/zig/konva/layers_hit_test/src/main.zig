const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

const CardCtx = struct { idx: usize };

var meta_buf: [192]u8 = undefined;
var id_buf: [64]u8 = undefined;
var card_ctxs: [3]CardCtx = .{ .{ .idx = 0 }, .{ .idx = 1 }, .{ .idx = 2 } };
const card_ids: [3][]const u8 = .{ "card-a", "card-b", "card-c" };

const State = struct {
    stage: kv.Stage = .{ .h = 0 },
    fg_layer: kv.Layer = .{ .h = 0 },
    overlay: kv.Group = .{ .h = 0 },
    cards: [3]kv.Rect = .{ .{ .h = 0 }, .{ .h = 0 }, .{ .h = 0 } },
    clicks: u32 = 0,
};

var g_state: State = .{};

fn onCardClick(ctx: *CardCtx, _: kv.KonvaEventObject) void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    const card = g_state.cards[ctx.idx];
    _ = card.moveToTop();
    _ = g_state.overlay.moveToTop();
    _ = g_state.fg_layer.draw();
    g_state.clicks += 1;
    const z: i32 = @intFromFloat(card.getZIndex());

    const hit_id: []const u8 = if (g_state.stage.getIntersection(.{ .x = 250, .y = 160 })) |s|
        s.getId().invoke().toString().copy(&id_buf)
    else
        "none";

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Layers: {d} \xC2\xB7 top={s} (z={d}) \xC2\xB7 hit@(250,160)={s} \xC2\xB7 clicks={d}", .{
                g_state.stage.getLayers().len(), card_ids[ctx.idx], z, hit_id, g_state.clicks,
            }) catch "",
        );
    }
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

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const bg_layer = kv.Layer.new(null);
    const fg_layer = kv.Layer.new(null);
    _ = stage.add(bg_layer, .{});
    _ = stage.add(fg_layer, .{});

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

    // Layer 2: Stacked cards, badge, and non-listening crosshair overlay.
    const card_a = makeCard("card-a", 110, 80, "#efefef");
    const card_b = makeCard("card-b", 210, 125, "#404040");
    const card_c = makeCard("card-c", 310, 170, "#ffffff");
    const badge = kv.Circle.new(.{ .id = "badge", .x = 555, .y = 135, .radius = 38, .fill = "#111111" });
    const overlay = kv.Group.new(.{ .listening = false });
    _ = overlay.add(.{
        kv.Circle.new(.{ .x = 250, .y = 160, .radius = 7, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2 }),
        makeLine(&.{ 238, 160, 262, 160 }, "#111111", 1.5),
        makeLine(&.{ 250, 148, 250, 172 }, "#111111", 1.5),
        kv.Text.new(.{ .x = 32, .y = 354, .text = "Layer 2: Elevated Stack (z=1) \xC2\xB7 Probe (250,160)", .fontSize = 12, .fontFamily = "monospace", .fill = "#111111" }),
    });
    _ = fg_layer.add(.{ card_a, card_b, card_c, badge, overlay });

    _ = card_a.moveToTop();
    _ = overlay.moveToTop();
    _ = bg_layer.draw();
    _ = fg_layer.draw();

    g_state.stage = stage.retain();
    g_state.fg_layer = fg_layer.retain();
    g_state.overlay = overlay.retain();
    g_state.cards = .{ card_a.retain(), card_b.retain(), card_c.retain() };

    for (g_state.cards, 0..) |card, i| {
        _ = card.on("click", .{ onCardClick, &card_ctxs[i] });
    }
    _ = card_a.fire("click", null);
    _ = card_a.fire("click", null);
}
