#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

#define Position(x) Echarts_PositionSizeOption_of(x)
#define Bound(x) Echarts_AngleAxisComponentOption_category_min_of(x)

static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = Echarts_Color_of(hex));
}

static Echarts_AxisLabelOption label_color(const char* hex) {
    return Echarts_AxisLabelOption_create(.color = js_string(hex));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    JsArray_Echarts_CategoryAxisBase_data_or_string_or_number weeks = JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_new();
    const char* wk[] = { "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" };
    for (int i = 0; i < 8; i++) JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_push(weeks, wk[i]);

    JsArray vals0 = JsArray_new();
    double d0[] = { 28, 44, 39, 62, 58, 76, 81, 89 };
    for (int i = 0; i < 8; i++) js_push_num(vals0, d0[i]);
    Echarts_LineSeriesOption s0 = Echarts_LineSeriesOption_create(
        .name = Echarts_OptionName_of("Throughput"), .smooth = Echarts_IncrementalIdCompat_of(true), .data = vals0,
        .lineStyle = Echarts_LineStyleOption_create(.color = Echarts_Color_of("#111111"), .width = 2),
        .itemStyle = Echarts_ItemStyleOption_create(.color = Echarts_ItemStyleOption_color_of("#111111")),
        .areaStyle = Echarts_LineSeriesOption_areaStyle_create(.color = Echarts_PathStyleProps_fill_of("rgba(17,17,17,0.12)")));

    JsArray vals1 = JsArray_new();
    double d1[] = { 35, 38, 42, 46, 50, 54, 58, 62 };
    for (int i = 0; i < 8; i++) js_push_num(vals1, d1[i]);
    Echarts_LineSeriesOption s1 = Echarts_LineSeriesOption_create(
        .name = Echarts_OptionName_of("Baseline"), .smooth = Echarts_IncrementalIdCompat_of(false), .data = vals1,
        .lineStyle = Echarts_LineStyleOption_create(.color = Echarts_Color_of("#737373"), .width = 1.5, .type = Echarts_ZRLineType_of("dashed")),
        .itemStyle = Echarts_ItemStyleOption_create(.color = Echarts_ItemStyleOption_color_of("#737373")));

    JsArray series = JsArray_new();
    js_push_val(series, s0);
    js_push_val(series, s1);

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = Echarts_Color_of("#ffffff"),
        .legend = EChartsOption_legend_of(Echarts_PlainLegendComponentOption_create(
            .top = Position(8), .textStyle = Echarts_LabelOption_create(.color = js_string("#111111")))),
        .tooltip = EChartsOption_tooltip_of(Echarts_TooltipComponentOption_create(.trigger = Echarts_TooltipComponentOption_trigger_axis)),
        .grid = EChartsOption_grid_of(Echarts_GridComponentOption_create(
            .left = Position(48), .right = Position(24), .top = Position(48), .bottom = Position(36))),
        .xAxis = EChartsOption_xAxis_of(Echarts_XAXisComponentOption_xAxis_create(
            .boundaryGap = false, .data = weeks,
            .axisLine = Echarts_AxisLineOption_create(.lineStyle = line_color("#111111")),
            .axisLabel = label_color("#555555"))),
        .yAxis = EChartsOption_yAxis_of(Echarts_YAXisComponentOption_yAxis_value_create(
            .min = Bound(0), .max = Bound(100),
            .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")),
            .axisLabel = label_color("#555555"))),
        .series = EChartsOption_series_of(series));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 2 series (Throughput + Baseline)",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
