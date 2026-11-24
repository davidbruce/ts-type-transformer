#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

using namespace echarts;

static PieSeriesOption_data slice(const char* name, double val) {
    return PieDataItemOption({.name = name, .value = val});
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    ECharts chart = echarts::init(*doc.getElementById("chart"));

    EChartsOption opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .color = ts::Array<std::string>::of("#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4"),
        .tooltip = TooltipComponentOption({.trigger = TooltipComponentOption_trigger::item}),
        .legend = PlainLegendComponentOption({.textStyle = LabelOption({.color = "#111111"}), .bottom = 8}),
        .series = PieSeriesOption({
            .data = ts::Array<PieSeriesOption_data>::of(slice("Runtime", 34), slice("Bindings", 26), slice("Glue", 18), slice("Parser", 14), slice("Tests", 8)),
            .center = ts::Array<PositionSizeOption>::of("50%", "46%"),
            .radius = ts::Array<PositionSizeOption>::of("44%", "72%"),
            .name = "Subsystems",
            .itemStyle = PieItemStyleOption({.borderColor = "#ffffff", .borderWidth = 2}),
            .label = PieLabelOption({.color = "#111111"}),
        }),
    });
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100"
    );
}
