#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>


static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = hex);
}

static Echarts_AxisLabelOption label_color(const char* hex) {
    return Echarts_AxisLabelOption_create(.color = js_string(hex));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    Echarts_LineSeriesOption s0 = Echarts_LineSeriesOption_create(
        .name = "Throughput", .smooth_boolean = true, .data = js_values(28, 44, 39, 62, 58, 76, 81, 89),
        .lineStyle = Echarts_LineStyleOption_create(.color = "#111111", .width = 2),
        .itemStyle = Echarts_ItemStyleOption_create(.color = "#111111"),
        .areaStyle = Echarts_LineSeriesOption_areaStyle_create(.color = "rgba(17,17,17,0.12)"));

    Echarts_LineSeriesOption s1 = Echarts_LineSeriesOption_create(
        .name = "Baseline", .smooth_boolean = false, .data = js_values(35, 38, 42, 46, 50, 54, 58, 62),
        .lineStyle = Echarts_LineStyleOption_create(.color = "#737373", .width = 1.5, .type_string = "dashed"),
        .itemStyle = Echarts_ItemStyleOption_create(.color = "#737373"));

    JsArray series = js_values(js_value(s0), js_value(s1));

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend_plain = Echarts_PlainLegendComponentOption_create(
            .top = 8, .textStyle = Echarts_LabelOption_create(.color = js_string("#111111"))),
        .tooltip = Echarts_TooltipComponentOption_create(.trigger = Echarts_TooltipComponentOption_trigger_axis),
        .grid = Echarts_GridComponentOption_create(
            .left = 48, .right = 24, .top = 48, .bottom = 36),
        .xAxis_category = Echarts_XAXisComponentOption_category_create(
            .boundaryGap = false, .data_strings = js_strings("W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"),
            .axisLine = Echarts_AxisLineOption_create(.lineStyle = line_color("#111111")),
            .axisLabel = label_color("#555555")),
        .yAxis_value = Echarts_YAXisComponentOption_value_create(
            .min = 0, .max = 100,
            .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")),
            .axisLabel = label_color("#555555")),
        .series_list = series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 2 series (Throughput + Baseline)",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
