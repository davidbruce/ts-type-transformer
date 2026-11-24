#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

#define Position(x) Echarts_PositionSizeOption_of(x)
#define Bound(x) Echarts_AngleAxisComponentOption_category_min_of(x)

static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = Echarts_LineStyle_or_string_or_Color_array_of(hex));
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    ECharts chart = echarts_init(Document_getElementById(doc, "chart"), NULL);

    JsArray_Echarts_CategoryAxisBase_data_or_string_or_number cats = JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_new();
    const char* months[] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun" };
    for (int i = 0; i < 6; i++) JsArray_Echarts_CategoryAxisBase_data_or_string_or_number_push(cats, months[i]);

    JsArray vals = JsArray_new();
    double nums[] = { 42, 68, 55, 84, 71, 93 };
    for (int i = 0; i < 6; i++) js_push_num(vals, nums[i]);

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = Echarts_Color_of("#ffffff"),
        .title = EChartsOption_title_of(Echarts_TitleComponentOption_create(
            .text = "Monthly Output", .left = Position(16), .top = Position(12),
            .textStyle = Echarts_TitleTextStyleOption_create(.color = js_string("#111111"), .fontSize = Echarts_string_or_number_of(13)))),
        .grid = EChartsOption_grid_of(Echarts_GridComponentOption_create(
            .left = Position(48), .right = Position(24), .top = Position(48), .bottom = Position(36))),
        .xAxis = EChartsOption_xAxis_of(Echarts_XAXisComponentOption_xAxis_create(
            .data = cats,
            .axisLine = Echarts_AxisLineOption_create(.lineStyle = line_color("#111111")),
            .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555")))),
        .yAxis = EChartsOption_yAxis_of(Echarts_YAXisComponentOption_yAxis_value_create(
            .min = Bound(0), .max = Bound(100),
            .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")),
            .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555")))),
        .series = EChartsOption_series_of(Echarts_BarSeriesOption_create(
            .name = Echarts_OptionName_of("Units"), .barWidth = Position("46%"), .data = vals,
            .itemStyle = Echarts_BarItemStyleOption_create(.color = Echarts_ItemStyle_or_SVGPatternObject_of("#111111")))));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
