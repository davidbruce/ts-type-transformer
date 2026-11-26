const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

fn addCaption(layer: kv.Layer, x: f64, y: f64, label: []const u8) void {
    _ = layer.add(.{kv.Text.new(.{
        .x = x,
        .y = y,
        .text = label,
        .fontSize = 11,
        .fontFamily = "monospace",
        .fill = "#555555",
    })});
}

export fn start() void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const layer = kv.Layer.new(null);
    _ = stage.add(layer);

    _ = layer.add(.{
        kv.Rect.new(.{ .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff" }),
        // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
        kv.Rect.new(.{ .x = 40, .y = 55, .width = 110, .height = 76, .cornerRadius = 8, .fill = "#111111" }),
        kv.Rect.new(.{ .x = 195, .y = 55, .width = 110, .height = 76, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2, .dash = &.{ 6, 4 } }),
        kv.Circle.new(.{ .x = 415, .y = 93, .radius = 40, .fill = "#404040", .stroke = "#111111", .strokeWidth = 2 }),
        kv.RegularPolygon.new(.{ .x = 575, .y = 93, .sides = 6, .radius = 42, .fill = "#efefef", .stroke = "#111111", .strokeWidth = 2 }),
        // Row 2: Star, Ring, Wedge, Framed Text.
        kv.Star.new(.{ .x = 95, .y = 268, .numPoints = 5, .innerRadius = 19, .outerRadius = 44, .fill = "#737373", .stroke = "#111111", .strokeWidth = 2 }),
        kv.Ring.new(.{ .x = 250, .y = 268, .innerRadius = 22, .outerRadius = 42, .fill = "#d4d4d4", .stroke = "#111111", .strokeWidth = 2 }),
        kv.Wedge.new(.{ .x = 415, .y = 268, .radius = 44, .angle = 270, .rotation = -90, .fill = "#111111", .stroke = "#555555", .strokeWidth = 1 }),
        kv.Rect.new(.{ .x = 500, .y = 224, .width = 150, .height = 88, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5 }),
        kv.Text.new(.{ .x = 514, .y = 240, .text = "KONVA 2D\nMonochrome\nScene Graph", .fontSize = 13, .fontFamily = "monospace", .fill = "#111111" }),
    });

    addCaption(layer, 40, 142, "Rect (filled)");
    addCaption(layer, 195, 142, "Rect (dashed)");
    addCaption(layer, 395, 142, "Circle");
    addCaption(layer, 548, 142, "Hexagon");
    addCaption(layer, 62, 325, "Star (5pt)");
    addCaption(layer, 236, 325, "Ring");
    addCaption(layer, 382, 325, "Wedge 270\xC2\xB0");
    addCaption(layer, 558, 325, "Text");

    _ = layer.draw();

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 7 primitive types \xC2\xB7 {d} layer children", .{layer.getChildren(null).len()}) catch "",
        );
    }
}
