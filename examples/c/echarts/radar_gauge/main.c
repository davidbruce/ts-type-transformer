#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = hex);
}

static void push_ind(JsArray_Echarts_RadarIndicatorOption arr, const char* name) {
    JsArray_Echarts_RadarIndicatorOption_push(arr, Echarts_RadarIndicatorOption_create(.name = name, .max = 100));
}

static JsArray num_arr(const double* vals, int n) {
    JsArray a = JsArray_new();
    for (int i = 0; i < n; i++) js_push_num(a, vals[i]);
    return a;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    ECharts radar = echarts_init(Document_getElementById(doc, "chart-radar"), NULL);
    ECharts gauge = echarts_init(Document_getElementById(doc, "chart-gauge"), NULL);

    /* ---- Radar ---- */
    // Its colors alternate.
    JsArray r_sa_cols = JsArray_new();
    js_push_str(r_sa_cols, "#ffffff");
    js_push_str(r_sa_cols, "#f7f7f7");
    Echarts_AreaStyleOption sa_style = Echarts_AreaStyleOption_create(.color_array = r_sa_cols);

    JsArray_Echarts_RadarIndicatorOption inds = JsArray_Echarts_RadarIndicatorOption_new();
    push_ind(inds, "Throughput");
    push_ind(inds, "Memory");
    push_ind(inds, "Startup");
    push_ind(inds, "Type Safety");
    push_ind(inds, "Bundle");

    const double v_wasm[5] = { 92, 86, 88, 96, 84 };
    const double v_base[5] = { 70, 68, 74, 72, 66 };

    JsArray_Echarts_RadarSeriesOption_data r_data = JsArray_Echarts_RadarSeriesOption_data_new();
    JsArray_Echarts_RadarSeriesOption_data_push(r_data, Echarts_RadarSeriesDataItemOption_create(
        .name = "WASM Target", .value = js_value(num_arr(v_wasm, 5)),
        .lineStyle = Echarts_LineStyleOption_create(.color = "#111111", .width = 2),
        .itemStyle = Echarts_ItemStyleOption_create(.color = "#111111"),
        .areaStyle = Echarts_AreaStyleOption_create(.color = "rgba(17,17,17,0.12)")));
    JsArray_Echarts_RadarSeriesOption_data_push(r_data, Echarts_RadarSeriesDataItemOption_create(
        .name = "Baseline", .value = js_value(num_arr(v_base, 5)),
        .lineStyle = Echarts_LineStyleOption_create(.color = "#737373", .width = 1.5, .type_string = "dashed"),
        .itemStyle = Echarts_ItemStyleOption_create(.color = "#737373")));

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
        .series_radar = Echarts_RadarSeriesOption_create(.data = r_data));
    ECharts_setOption(radar, radar_opt);

    /* ---- Gauge ---- */
    JsArray stop = JsArray_new();
    js_push_num(stop, 1);
    js_push_str(stop, "#e5e5e5");
    JsArray al_stops = JsArray_new();
    js_push_val(al_stops, stop);

    JsArray_Echarts_number_or_string_or_GaugeDataItemOption g_data = JsArray_Echarts_number_or_string_or_GaugeDataItemOption_new();
    JsArray_Echarts_number_or_string_or_GaugeDataItemOption_push(g_data, Echarts_GaugeDataItemOption_create(
        .value = 84, .name = "Score"));

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
            .data = g_data));
    ECharts_setOption(gauge, gauge_opt);

    char meta[128];
    snprintf(meta, sizeof meta, "Radar: %dx%d \xC2\xB7 Gauge: %dx%d \xC2\xB7 Score 84%%",
        (int)ECharts_getWidth(radar), (int)ECharts_getHeight(radar),
        (int)ECharts_getWidth(gauge), (int)ECharts_getHeight(gauge));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
