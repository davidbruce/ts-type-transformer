const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

fn addStageBox(layer: kv.Layer, x: f64, title: []const u8, sub: []const u8, fill: []const u8, ink: []const u8) void {
    const g = kv.Group.new(.{ .x = x, .y = 95 });
    _ = g.add(.{
        kv.Rect.new(.{ .width = 126, .height = 68, .cornerRadius = 3, .fill = fill, .stroke = "#111111", .strokeWidth = 2 }),
        kv.Text.new(.{ .x = 12, .y = 16, .text = title, .fontSize = 12, .fontStyle = "bold", .fontFamily = "monospace", .fill = ink }),
        kv.Text.new(.{ .x = 12, .y = 38, .text = sub, .fontSize = 11, .fontFamily = "monospace", .fill = ink }),
    });
    _ = layer.add(.{g});
}

fn addArrow(layer: kv.Layer, x1: f64, y1: f64, x2: f64, y2: f64) void {
    _ = layer.add(.{kv.Arrow.new(.{
        .points = &.{ x1, y1, x2, y2 },
        .pointerLength = 8,
        .pointerWidth = 8,
        .fill = "#111111",
        .stroke = "#111111",
        .strokeWidth = 2,
    })});
}

export fn start() void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const layer = kv.Layer.new(null);
    _ = stage.add(layer);

    _ = layer.add(.{kv.Rect.new(.{ .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff" })});

    addStageBox(layer, 28, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    addStageBox(layer, 194, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    addStageBox(layer, 360, "03 EMIT", "6 Targets", "#efefef", "#111111");
    addStageBox(layer, 526, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    addArrow(layer, 154, 129, 190, 129);
    addArrow(layer, 320, 129, 356, 129);
    addArrow(layer, 486, 129, 522, 129);
    addArrow(layer, 257, 163, 257, 251);
    addArrow(layer, 423, 163, 423, 251);

    _ = layer.add(.{
        kv.Line.new(.{
            .points = &.{ 589, 163, 589, 212, 91, 212, 91, 168 },
            .stroke = "#737373",
            .strokeWidth = 1.5,
            .dash = &.{ 6, 4 },
        }),
        kv.Rect.new(.{
            .x = 28,
            .y = 255,
            .width = 624,
            .height = 58,
            .cornerRadius = 3,
            .fill = "#f7f7f7",
            .stroke = "#111111",
            .strokeWidth = 1.5,
        }),
        kv.Text.new(.{
            .x = 180,
            .y = 278,
            .text = "Shared DOM & Package Handle Runtime",
            .fontSize = 12,
            .fontFamily = "monospace",
            .fill = "#111111",
        }),
    });

    _ = layer.draw();

    const rects = layer.find("Rect").len();
    const arrows = layer.find("Arrow").len();
    const jlen = layer.toJSON().len();

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Pipeline diagram \xC2\xB7 {d} rects \xC2\xB7 {d} arrows \xC2\xB7 JSON {d} bytes", .{ rects, arrows, jlen }) catch "",
        );
    }
}
