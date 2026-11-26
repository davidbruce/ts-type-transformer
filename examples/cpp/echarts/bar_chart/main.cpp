#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

using namespace echarts;

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    ECharts chart = echarts::init(*doc.getElementById("chart"));

    EChartsOption opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .title = TitleComponentOption({.top = 12, .left = 16, .text = "Monthly Output", .textStyle = TitleTextStyleOption({.color = "#111111", .fontSize = 13})}),
        .grid = GridComponentOption({.top = 48, .right = 24, .bottom = 36, .left = 48}),
        .xAxis = XAXisComponentOption_category({
            .axisLabel = AxisLabelOption({.color = "#555555"}),
            .data = {"Jan", "Feb", "Mar", "Apr", "May", "Jun"},
            .axisLine = AxisLineOption({.lineStyle = LineStyleOption({.color = "#111111"})}),
        }),
        .yAxis = YAXisComponentOption_value({
            .axisLabel = AxisLabelOption({.color = "#555555"}),
            .splitLine = SplitLineOption({.lineStyle = LineStyleOption({.color = "#e5e5e5"})}),
            .min = 0,
            .max = 100,
        }),
        .series = BarSeriesOption({
            .data = {42, 68, 55, 84, 71, 93},
            .name = "Units",
            .barWidth = "46%",
            .itemStyle = BarItemStyleOption({.color = "#111111"}),
        }),
    });
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111"
    );
}
