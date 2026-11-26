const std = @import("std");
const dom = @import("dom");
const kv = @import("konva");

var meta_buf: [192]u8 = undefined;
var strip_buf: [160]u8 = undefined;

fn makeLine(pts: []const f64) kv.Line {
    return kv.Line.new(.{ .points = pts, .stroke = "#e5e5e5", .strokeWidth = 1 });
}

export fn start() void {
    const scope = kv.JsScope.open();
    defer scope.deinit();

    const stage = kv.Stage.new(.{ .container = "stage", .width = 680, .height = 400 });
    const layer = kv.Layer.new(null);
    _ = stage.add(layer);

    _ = layer.add(.{kv.Rect.new(.{ .x = 0, .y = 0, .width = 680, .height = 400, .fill = "#ffffff" })});

    var x: i32 = 40;
    while (x <= 640) : (x += 40) {
        const xf: f64 = @floatFromInt(x);
        _ = layer.add(.{makeLine(&.{ xf, 0, xf, 400 })});
    }
    var y: i32 = 40;
    while (y <= 360) : (y += 40) {
        const yf: f64 = @floatFromInt(y);
        _ = layer.add(.{makeLine(&.{ 0, yf, 680, yf })});
    }

    // Group A.
    const group_a = kv.Group.new(.{ .x = 40, .y = 55 });
    _ = group_a.add(.{
        kv.Rect.new(.{ .width = 160, .height = 120, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5 }),
        kv.Circle.new(.{ .x = 80, .y = 60, .radius = 24, .fill = "#111111" }),
        kv.Text.new(.{ .x = 12, .y = 10, .text = "Group A \xC2\xB7 0\xC2\xB0", .fontSize = 11, .fontFamily = "monospace", .fill = "#111111" }),
    });

    // Group B.
    const group_b = kv.Group.new(.{ .x = 340, .y = 75, .rotation = 24 });
    _ = group_b.add(.{
        kv.Rect.new(.{ .width = 150, .height = 110, .fill = "#efefef", .stroke = "#111111", .strokeWidth = 2 }),
        kv.Rect.new(.{ .x = 22, .y = 22, .width = 106, .height = 66, .fill = "#ffffff", .stroke = "#555555", .strokeWidth = 1.5, .dash = &.{ 4, 3 } }),
        kv.Circle.new(.{ .x = 75, .y = 55, .radius = 14, .fill = "#404040" }),
        kv.Text.new(.{ .x = 10, .y = 8, .text = "Group B \xC2\xB7 24\xC2\xB0", .fontSize = 11, .fontFamily = "monospace", .fill = "#111111" }),
    });

    // Group C + nested inner group.
    const group_c = kv.Group.new(.{ .x = 495, .y = 55, .scaleX = 1.15, .scaleY = 1.15 });
    const inner_group = kv.Group.new(.{ .x = 25, .y = 25, .rotation = 15 });
    const marker = kv.Circle.new(.{ .x = 40, .y = 30, .radius = 16, .fill = "#111111" });
    _ = inner_group.add(.{marker});
    _ = group_c.add(.{
        kv.Rect.new(.{ .width = 130, .height = 110, .fill = "#ffffff", .stroke = "#111111", .strokeWidth = 2 }),
        kv.Text.new(.{ .x = 8, .y = 8, .text = "Group C \xC2\xB7 1.15x", .fontSize = 10, .fontFamily = "monospace", .fill = "#111111" }),
        inner_group,
    });
    _ = layer.add(.{ group_a, group_b, group_c });

    const abs = marker.getAbsolutePosition(null);
    const bbox = group_b.getClientRect(null);
    const ax: i32 = @intFromFloat(@round(abs.getX()));
    const ay: i32 = @intFromFloat(@round(abs.getY()));
    const bw: i32 = @intFromFloat(@round(bbox.getWidth()));
    const bh: i32 = @intFromFloat(@round(bbox.getHeight()));
    const strip_txt = std.fmt.bufPrint(&strip_buf, "Marker abs=({d}, {d}) \xC2\xB7 Group B bbox={d}x{d}", .{ ax, ay, bw, bh }) catch "";

    _ = layer.add(.{
        kv.Rect.new(.{ .x = 28, .y = 308, .width = 624, .height = 60, .fill = "#f7f7f7", .stroke = "#111111", .strokeWidth = 1.5 }),
        kv.Text.new(.{ .x = 44, .y = 331, .text = strip_txt, .fontSize = 12, .fontFamily = "monospace", .fill = "#111111" }),
    });

    _ = layer.draw();

    if (dom.getDocument().getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Stage: 680x400 \xC2\xB7 Nested marker abs=({d}, {d}) \xC2\xB7 Rotated Group B rect={d}x{d}", .{ ax, ay, bw, bh }) catch "",
        );
    }
}
