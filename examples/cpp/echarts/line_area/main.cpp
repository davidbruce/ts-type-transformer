#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

using namespace echarts;

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    ECharts chart = echarts::init(*doc.getElementById("chart"));

    auto s0 = LineSeriesOption({
        .lineStyle = LineStyleOption({.width = 2, .color = "#111111"}),
        .areaStyle = LineSeriesOption_areaStyle({.color = "rgba(17,17,17,0.12)"}),
        .smooth = true,
        .data = ts::Array<double>::of(28, 44, 39, 62, 58, 76, 81, 89),
        .name = "Throughput",
        .itemStyle = ItemStyleOption({.color = "#111111"}),
    });

    auto s1 = LineSeriesOption({
        .lineStyle = LineStyleOption({.width = 1.5, .color = "#737373", .type = "dashed"}),
        .smooth = false,
        .data = ts::Array<double>::of(35, 38, 42, 46, 50, 54, 58, 62),
        .name = "Baseline",
        .itemStyle = ItemStyleOption({.color = "#737373"}),
    });

    EChartsOption opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .grid = GridComponentOption({.top = 48, .right = 24, .bottom = 36, .left = 48}),
        .xAxis = XAXisComponentOption_category({
            .boundaryGap = false,
            .axisLabel = AxisLabelOption({.color = "#555555"}),
            .data = ts::Array<std::string>::of("W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"),
            .axisLine = AxisLineOption({.lineStyle = LineStyleOption({.color = "#111111"})}),
        }),
        .yAxis = YAXisComponentOption_value({
            .axisLabel = AxisLabelOption({.color = "#555555"}),
            .splitLine = SplitLineOption({.lineStyle = LineStyleOption({.color = "#e5e5e5"})}),
            .min = 0,
            .max = 100,
        }),
        .tooltip = TooltipComponentOption({.trigger = TooltipComponentOption_trigger::axis}),
        .legend = PlainLegendComponentOption({.textStyle = LabelOption({.color = "#111111"}), .top = 8}),
        .series = ts::Array<LineSeriesOption>::of(s0, s1),
    });
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 2 series (Throughput + Baseline)"
    );
}
