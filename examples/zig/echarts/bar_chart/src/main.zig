const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const box = doc.getElementById("chart") orelse return;
    const chart = ec.init(.{ .dom = box });

    const opt = ec.EChartsOption.init(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .title = .{
            .text = "Monthly Output",
            .left = 16,
            .top = 12,
            .textStyle = .{ .color = "#111111", .fontSize = 13 },
        },
        .grid = .{ .left = 48, .right = 24, .top = 48, .bottom = 36 },
        .xAxis = .{
            .type = "category",
            .data = .{ "Jan", "Feb", "Mar", "Apr", "May", "Jun" },
            .axisLine = .{ .lineStyle = .{ .color = "#111111" } },
            .axisLabel = .{ .color = "#555555" },
        },
        .yAxis = .{
            .type = "value",
            .min = 0,
            .max = 100,
            .splitLine = .{ .lineStyle = .{ .color = "#e5e5e5" } },
            .axisLabel = .{ .color = "#555555" },
        },
        .series = .{.{
            .type = "bar",
            .name = "Units",
            .barWidth = "46%",
            .data = &.{ 42, 68, 55, 84, 71, 93 },
            .itemStyle = .{ .color = "#111111" },
        }},
    });
    chart.setOption(opt);

    const w: i32 = @intFromFloat(chart.getWidth());
    const h: i32 = @intFromFloat(chart.getHeight());
    if (doc.getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111", .{ w, h }) catch "",
        );
    }
}
