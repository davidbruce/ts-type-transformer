#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = hex);
}

static void push_ind(JsArray_Echarts_RadarIndicatorOption arr, const char* name) {
    JsArray_Echarts_RadarIndicatorOption_push(arr, Echarts_RadarIndicatorOption_create(.name = name, .max = 100));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    ECharts radar = echarts_init(Document_getElementById(doc, "chart-radar"), NULL);
    ECharts gauge = echarts_init(Document_getElementById(doc, "chart-gauge"), NULL);

    /* ---- Radar ---- */
    // Its colors alternate.
    Echarts_AreaStyleOption sa_style = Echarts_AreaStyleOption_create(
        .color_array = js_values("#ffffff", "#f7f7f7"));

    JsArray_Echarts_RadarIndicatorOption inds = JsArray_Echarts_RadarIndicatorOption_new();
    push_ind(inds, "Throughput");
    push_ind(inds, "Memory");
    push_ind(inds, "Startup");
    push_ind(inds, "Type Safety");
    push_ind(inds, "Bundle");

    EChartsOption radar_opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend_plain = Echarts_PlainLegendComponentOption_create(
            .bottom = 6, .textStyle = Echarts_LabelOption_create(.color = js_string("#111111"))),
        .radar = Echarts_RadarComponentOption_create(
            .radius_string = "62%",
            .axisName = Echarts_RadarComponentOption_axisName_create(.color = "#111111"),
            .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#d4d4d4")),
            .splitArea = Echarts_SplitAreaOption_create(.areaStyle = sa_style),
            .axisLine = Echarts_AxisLineOption_create(.lineStyle = line_color("#b5b5b5")),
            .indicator = inds),
        .series_radar = Echarts_RadarSeriesOption_create(
            .data_items = Echarts_RadarSeriesDataItemOption_list(
                Echarts_RadarSeriesDataItemOption_create(
                    .name = "WASM Target", .value = js_value(js_numbers(92, 86, 88, 96, 84)),
                    .lineStyle = Echarts_LineStyleOption_create(.color = "#111111", .width = 2),
                    .itemStyle = Echarts_ItemStyleOption_create(.color = "#111111"),
                    .areaStyle = Echarts_AreaStyleOption_create(.color = "rgba(17,17,17,0.12)")),
                Echarts_RadarSeriesDataItemOption_create(
                    .name = "Baseline", .value = js_value(js_numbers(70, 68, 74, 72, 66)),
                    .lineStyle = Echarts_LineStyleOption_create(.color = "#737373", .width = 1.5, .type_string = "dashed"),
                    .itemStyle = Echarts_ItemStyleOption_create(.color = "#737373")))));
    ECharts_setOption(radar, radar_opt);

    /* ---- Gauge ---- */
    JsArray al_stops = js_values(js_values(1, "#e5e5e5"));

    EChartsOption gauge_opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .series_gauge = Echarts_GaugeSeriesOption_create(
            .min = 0, .max = 100,
            .progress = Echarts_ProgressOption_create(
                .show = true, .width = 14, .itemStyle = Echarts_ItemStyleOption_create(.color = "#111111")),
            .axisLine = Echarts_GaugeSeriesOption_axisLine_create(
                .lineStyle = Echarts_GaugeSeriesOption_axisLine_lineStyle_create(.width = 14, .color = al_stops)),
            .axisTick = Echarts_GaugeSeriesOption_axisTick_create(.lineStyle = line_color("#737373")),
            .splitLine = Echarts_GaugeSeriesOption_splitLine_create(.lineStyle = line_color("#111111")),
            .axisLabel = Echarts_GaugeSeriesOption_axisLabel_create(.color = "#555555"),
            .pointer = Echarts_PointerOption_create(.itemStyle = Echarts_ItemStyleOption_create(.color = "#111111")),
            .detail = Echarts_DetailOption_create(
                .color = js_string("#111111"), .valueAnimation = false, .formatter = "{value}%",
                .fontSize = 22),
            .title = Echarts_TitleOption_create(.color = js_string("#555555"), .fontSize = 12),
            .data_items = Echarts_GaugeDataItemOption_list(
                Echarts_GaugeDataItemOption_create(.value = 84, .name = "Score"))));
    ECharts_setOption(gauge, gauge_opt);

    char meta[128];
    snprintf(meta, sizeof meta, "Radar: %dx%d \xC2\xB7 Gauge: %dx%d \xC2\xB7 Score 84%%",
        (int)ECharts_getWidth(radar), (int)ECharts_getHeight(radar),
        (int)ECharts_getWidth(gauge), (int)ECharts_getHeight(gauge));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
