const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const box = doc.getElementById("chart") orelse return;
    const chart = ec.init(box, null);

    const opt = ec.JsValue.obj(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend = .{ .top = 8, .textStyle = .{ .color = "#111111" } },
        .tooltip = .{ .trigger = "axis" },
        .grid = .{ .left = 48, .right = 24, .top = 48, .bottom = 36 },
        .xAxis = .{
            .type = "category",
            .boundaryGap = false,
            .data = .{ "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" },
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
        .series = .{
            .{
                .type = "line",
                .name = "Throughput",
                .smooth = true,
                .data = &.{ 28, 44, 39, 62, 58, 76, 81, 89 },
                .lineStyle = .{ .color = "#111111", .width = 2 },
                .itemStyle = .{ .color = "#111111" },
                .areaStyle = .{ .color = "rgba(17,17,17,0.12)" },
            },
            .{
                .type = "line",
                .name = "Baseline",
                .smooth = false,
                .data = &.{ 35, 38, 42, 46, 50, 54, 58, 62 },
                .lineStyle = .{ .color = "#737373", .width = 1.5, .type = "dashed" },
                .itemStyle = .{ .color = "#737373" },
            },
        },
    }).as(ec.EChartsCoreOption);
    chart.setOption(opt, null);

    const w: i32 = @intFromFloat(chart.getWidth());
    const h: i32 = @intFromFloat(chart.getHeight());
    if (doc.getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 2 series (Throughput + Baseline)", .{ w, h }) catch "",
        );
    }
}
