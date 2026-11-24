const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

fn makeValueAxis() ec.JsValue {
    return ec.JsValue.obj(.{
        .type = "value",
        .min = 0,
        .max = 100,
        .splitLine = .{ .lineStyle = .{ .color = "#e5e5e5" } },
        .axisLabel = .{ .color = "#555555" },
    });
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const box = doc.getElementById("chart") orelse return;
    const chart = ec.init(.{ .dom = box });

    const ca_pts = &[_][2]f64{ .{ 14, 22 }, .{ 21, 31 }, .{ 28, 27 }, .{ 33, 42 }, .{ 40, 48 }, .{ 46, 53 }, .{ 54, 61 }, .{ 62, 59 }, .{ 71, 74 }, .{ 82, 85 } };
    const cb_pts = &[_][2]f64{ .{ 18, 64 }, .{ 25, 72 }, .{ 31, 68 }, .{ 39, 79 }, .{ 47, 65 }, .{ 55, 81 }, .{ 64, 77 }, .{ 72, 88 }, .{ 79, 82 }, .{ 88, 92 } };

    chart.setOption(ec.EChartsOption.init(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend = ec.PlainLegendComponentOption.init(.{ .top = 8, .textStyle = .{ .color = "#111111" } }),
        .grid = .{ .left = 48, .right = 24, .top = 48, .bottom = 36 },
        .xAxis = makeValueAxis(),
        .yAxis = makeValueAxis(),
        .series = .{
            .{
                .type = "scatter",
                .name = "Cluster A",
                .symbol = "circle",
                .symbolSize = 10,
                .itemStyle = .{ .color = "#111111" },
                .data = ca_pts,
            },
            .{
                .type = "scatter",
                .name = "Cluster B",
                .symbol = "rect",
                .symbolSize = 10,
                .itemStyle = .{ .color = "#ffffff", .borderColor = "#111111", .borderWidth = 2 },
                .data = cb_pts,
            },
        },
    }), null);

    const w: i32 = @intFromFloat(chart.getWidth());
    const h: i32 = @intFromFloat(chart.getHeight());
    if (doc.getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 20 data points across 2 clusters", .{ w, h }) catch "",
        );
    }
}
