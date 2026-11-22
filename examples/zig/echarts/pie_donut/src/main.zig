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

    const opt = ec.EChartsOption.init(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .color = .{ "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" },
        .legend = ec.PlainLegendComponentOption.init(.{ .bottom = 8, .textStyle = .{ .color = "#111111" } }),
        .tooltip = .{ .trigger = .item },
        .series = .{.{
            .type = "pie",
            .name = "Subsystems",
            .radius = .{ "44%", "72%" },
            .center = .{ "50%", "46%" },
            .itemStyle = .{ .borderColor = "#ffffff", .borderWidth = 2 },
            .label = .{ .color = "#111111" },
            .data = .{
                .{ .name = "Runtime", .value = 34 },
                .{ .name = "Bindings", .value = 26 },
                .{ .name = "Glue", .value = 18 },
                .{ .name = "Parser", .value = 14 },
                .{ .name = "Tests", .value = 8 },
            },
        }},
    });
    chart.setOption(opt, null);

    const w: i32 = @intFromFloat(chart.getWidth());
    const h: i32 = @intFromFloat(chart.getHeight());
    if (doc.getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Canvas: {d}x{d} \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100", .{ w, h }) catch "",
        );
    }
}
