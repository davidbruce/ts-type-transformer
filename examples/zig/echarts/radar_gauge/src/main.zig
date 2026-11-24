const std = @import("std");
const dom = @import("dom");
const ec = @import("echarts");

var meta_buf: [128]u8 = undefined;

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const radar_box = doc.getElementById("chart-radar") orelse return;
    const gauge_box = doc.getElementById("chart-gauge") orelse return;

    const radar = ec.init(radar_box, null);
    const gauge = ec.init(gauge_box, null);

    // Its colors alternate: an array, which areaStyle's type does not say.
    const split_area = ec.AreaStyleOption.init(.{});
    ec.JsValue.of(split_area).set("color", .{ "#ffffff", "#f7f7f7" });
    radar.setOption(ec.EChartsOption.init(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend = ec.PlainLegendComponentOption.init(.{ .bottom = 6, .textStyle = .{ .color = "#111111" } }),
        .radar = .{
            .radius = "62%",
            .axisName = .{ .color = "#111111" },
            .splitLine = .{ .lineStyle = .{ .color = "#d4d4d4" } },
            .splitArea = .{ .areaStyle = split_area },
            .axisLine = .{ .lineStyle = .{ .color = "#b5b5b5" } },
            .indicator = .{
                .{ .name = "Throughput", .max = 100 },
                .{ .name = "Memory", .max = 100 },
                .{ .name = "Startup", .max = 100 },
                .{ .name = "Type Safety", .max = 100 },
                .{ .name = "Bundle", .max = 100 },
            },
        },
        .series = .{.{
            .type = "radar",
            .data = .{
                .{
                    .name = "WASM Target",
                    .value = &.{ 92, 86, 88, 96, 84 },
                    .lineStyle = .{ .color = "#111111", .width = 2 },
                    .itemStyle = .{ .color = "#111111" },
                    .areaStyle = .{ .color = "rgba(17,17,17,0.12)" },
                },
                .{
                    .name = "Baseline",
                    .value = &.{ 70, 68, 74, 72, 66 },
                    .lineStyle = .{ .color = "#737373", .width = 1.5, .type = "dashed" },
                    .itemStyle = .{ .color = "#737373" },
                },
            },
        }},
    }), null);

    gauge.setOption(ec.EChartsOption.init(.{
        .animation = false,
        .backgroundColor = "#ffffff",
        .series = .{.{
            .type = "gauge",
            .min = 0,
            .max = 100,
            .progress = .{ .show = true, .width = 14, .itemStyle = .{ .color = "#111111" } },
            .axisLine = .{ .lineStyle = .{ .width = 14, .color = .{.{ 1, "#e5e5e5" }} } },
            .axisTick = .{ .lineStyle = .{ .color = "#737373" } },
            .splitLine = .{ .lineStyle = .{ .color = "#111111" } },
            .axisLabel = .{ .color = "#555555" },
            .pointer = .{ .itemStyle = .{ .color = "#111111" } },
            .detail = .{ .color = "#111111", .valueAnimation = false, .formatter = "{value}%", .fontSize = 22 },
            .title = .{ .color = "#555555", .fontSize = 12 },
            .data = .{.{ .value = 84, .name = "Score" }},
        }},
    }), null);

    const w1: i32 = @intFromFloat(radar.getWidth());
    const h1: i32 = @intFromFloat(radar.getHeight());
    const w2: i32 = @intFromFloat(gauge.getWidth());
    const h2: i32 = @intFromFloat(gauge.getHeight());
    if (doc.getElementById("meta")) |meta_el| {
        meta_el.setTextContent(
            std.fmt.bufPrint(&meta_buf, "Radar: {d}x{d} \xC2\xB7 Gauge: {d}x{d} \xC2\xB7 Score 84%", .{ w1, h1, w2, h2 }) catch "",
        );
    }
}
