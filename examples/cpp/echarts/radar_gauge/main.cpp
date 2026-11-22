#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

using namespace echarts;

static RadarIndicatorOption indicator(const char* name) {
    return RadarIndicatorOption({.name = name, .max = 100});
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    ECharts radar = echarts::init(*doc.getElementById("chart-radar"));
    ECharts gauge = echarts::init(*doc.getElementById("chart-gauge"));

    /* ---- Radar ---- */
    auto item0 = RadarSeriesDataItemOption({
        .lineStyle = LineStyleOption<Color>({.width = 2, .color = "#111111"}),
        .areaStyle = AreaStyleOption<Color>({.color = "rgba(17,17,17,0.12)"}),
        .itemStyle = ItemStyleOption<DefaultLabelFormatterCallbackParams>({.color = "#111111"}),
        .name = "WASM Target",
        .value = ts::Array<double>::of(92, 86, 88, 96, 84),
    });

    auto item1 = RadarSeriesDataItemOption({
        .lineStyle = LineStyleOption<Color>({.width = 1.5, .color = "#737373", .type = "dashed"}),
        .itemStyle = ItemStyleOption<DefaultLabelFormatterCallbackParams>({.color = "#737373"}),
        .name = "Baseline",
        .value = ts::Array<double>::of(70, 68, 74, 72, 66),
    });

    EChartsOption radar_opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .radar = RadarComponentOption({
            .radius = "62%",
            .axisLine = AxisLineOption({.lineStyle = LineStyleOption<Color>({.color = "#b5b5b5"})}),
            .splitLine = SplitLineOption({.lineStyle = LineStyleOption<SplitLineOption_lineStyle>({.color = "#d4d4d4"})}),
            .splitArea = SplitAreaOption({.areaStyle = AreaStyleOption<ts::Array<>>({.color = ts::Array<std::string>::of("#ffffff", "#f7f7f7")})}),
            .axisName = RadarComponentOption_axisName({.color = "#111111"}),
            .indicator = ts::Array<RadarIndicatorOption>::of(
                indicator("Throughput"),
                indicator("Memory"),
                indicator("Startup"),
                indicator("Type Safety"),
                indicator("Bundle")
            ),
        }),
        .legend = PlainLegendComponentOption({.textStyle = LabelOption<DetailOption_object>({.color = "#111111"}), .bottom = 6}),
        .series = RadarSeriesOption({.data = ts::Array<RadarSeriesOption_data>::of(item0, item1)}),
    });
    radar.setOption(radar_opt);

    /* ---- Gauge ---- */
    EChartsOption gauge_opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .series = GaugeSeriesOption({
            .min = 0,
            .max = 100,
            .axisLine = GaugeSeriesOption_axisLine({.lineStyle = GaugeSeriesOption_axisLine_lineStyle({.width = 14, .color = ts::Array<ts::Array<>>::of(ts::Array<>::of(1.0, "#e5e5e5"))})}),
            .progress = ProgressOption({.show = true, .width = 14, .itemStyle = ItemStyleOption<ts::Value>({.color = "#111111"})}),
            .splitLine = GaugeSeriesOption_splitLine({.lineStyle = LineStyleOption<Color>({.color = "#111111"})}),
            .axisTick = GaugeSeriesOption_axisTick({.lineStyle = LineStyleOption<Color>({.color = "#737373"})}),
            .axisLabel = GaugeSeriesOption_axisLabel({.color = "#555555"}),
            .pointer = PointerOption({.itemStyle = ItemStyleOption<ts::Value>({.color = "#111111"})}),
            .title = TitleOption({.color = "#555555", .fontSize = 12}),
            .detail = DetailOption({.color = "#111111", .fontSize = 22, .formatter = "{value}%", .valueAnimation = false}),
            .data = ts::Array<number_or_string_or_GaugeDataItemOption>::of(GaugeDataItemOption({.name = "Score", .value = 84})),
        }),
    });
    gauge.setOption(gauge_opt);

    doc.getElementById("meta")->textContent(
        "Radar: " + std::to_string(static_cast<int>(radar.getWidth())) + "x" +
        std::to_string(static_cast<int>(radar.getHeight())) + " \xC2\xB7 Gauge: " +
        std::to_string(static_cast<int>(gauge.getWidth())) + "x" +
        std::to_string(static_cast<int>(gauge.getHeight())) + " \xC2\xB7 Score 84%"
    );
}
