const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [160]u8 = undefined;

export fn start() void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    const cx: f64 = 340;
    const cy: f64 = 200;

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const layer = kv.Layer.new(null);
    _ = stage.add(layer, .{});

    _ = layer.add(.{kv.Rect.new(.{ .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff" })});

    const ring_r = [_]f64{ 160, 125, 90, 52 };
    const ring_stroke = [_][]const u8{ "#d4d4d4", "#a3a3a3", "#555555", "#111111" };
    const ring_sw = [_]f64{ 1, 1, 1, 1.5 };
    for (ring_r, ring_stroke, ring_sw, 0..) |r, col, sw, idx| {
        _ = layer.add(.{kv.Circle.new(.{
            .x = cx,
            .y = cy,
            .radius = r,
            .stroke = col,
            .strokeWidth = sw,
            .fill = if (idx == 3) "#f7f7f7" else null,
        })});
    }

    for (0..12) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 6.0;
        _ = layer.add(.{kv.Line.new(.{
            .points = &.{
                cx + 36.0 * @cos(a),  cy + 36.0 * @sin(a),
                cx + 160.0 * @cos(a), cy + 160.0 * @sin(a),
            },
            .stroke = "#d4d4d4",
            .strokeWidth = 1,
        })});
    }

    for (0..24) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 12.0;
        const r0: f64 = if (i % 2 == 0) 148.0 else 154.0;
        _ = layer.add(.{kv.Line.new(.{
            .points = &.{
                cx + r0 * @cos(a),    cy + r0 * @sin(a),
                cx + 166.0 * @cos(a), cy + 166.0 * @sin(a),
            },
            .stroke = "#111111",
            .strokeWidth = if (i % 2 == 0) @as(f64, 2) else 1,
        })});
    }

    for (0..8) |i| {
        _ = layer.add(.{kv.Rect.new(.{
            .x = cx,
            .y = cy,
            .width = 18,
            .height = 52,
            .offsetX = 9,
            .offsetY = 108,
            .rotation = i * 45,
            .fill = if (i % 2 == 0) "#111111" else "#efefef",
            .stroke = "#111111",
            .strokeWidth = 1.5,
        })});
    }

    for (0..12) |i| {
        const a: f64 = @as(f64, @floatFromInt(i)) * std.math.pi / 6.0;
        _ = layer.add(.{kv.Circle.new(.{
            .x = cx + 125.0 * @cos(a),
            .y = cy + 125.0 * @sin(a),
            .radius = 5,
            .fill = if (i % 2 == 0) "#111111" else "#ffffff",
            .stroke = "#111111",
            .strokeWidth = 1.5,
        })});
    }

    _ = layer.add(.{
        kv.RegularPolygon.new(.{ .x = cx, .y = cy, .sides = 8, .radius = 26, .fill = "#111111" }),
        kv.Circle.new(.{ .x = cx, .y = cy, .radius = 9, .fill = "#ffffff" }),
    });

    const lx = [_]f64{ 516, 332, 140, 328 };
    const ly = [_]f64{ 194, 376, 194, 14 };
    const labels = [_][]const u8{ "0\xC2\xB0", "90\xC2\xB0", "180\xC2\xB0", "270\xC2\xB0" };
    for (lx, ly, labels) |x, y, txt| {
        _ = layer.add(.{kv.Text.new(.{
            .x = x,
            .y = y,
            .text = txt,
            .fontSize = 11,
            .fontFamily = "monospace",
            .fill = "#555555",
        })});
    }

    _ = layer.draw();

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 Procedural rosette \xC2\xB7 {d} shapes on layer", .{layer.getChildren(null).len()}) catch "",
        );
    }
}
