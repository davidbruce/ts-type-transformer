#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>


static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = hex);
}

#define VALUE_AXIS(T) T##_create( \
    .min = 0, .max = 100, \
    .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")), \
    .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555")))

static JsArray make_pts(const double pts[][2], int count) {
    JsArray arr = JsArray_new();
    for (int i = 0; i < count; i++) {
        js_push_val(arr, js_numbers(pts[i][0], pts[i][1]));
    }
    return arr;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };

    Echarts_ScatterSeriesOption s0 = Echarts_ScatterSeriesOption_create(
        .name = "Cluster A", .symbol = "circle", .symbolSize = 10,
        .itemStyle = Echarts_ItemStyleOption_create(.color = "#111111"),
        .data_list = make_pts(ca_pts, 10));
    Echarts_ScatterSeriesOption s1 = Echarts_ScatterSeriesOption_create(
        .name = "Cluster B", .symbol = "rect", .symbolSize = 10,
        .itemStyle = Echarts_ItemStyleOption_create(
            .color = "#ffffff", .borderColor = "#111111", .borderWidth = 2),
        .data_list = make_pts(cb_pts, 10));

    JsArray series = js_values(js_value(s0), js_value(s1));

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = "#ffffff",
        .legend_plain = Echarts_PlainLegendComponentOption_create(
            .top = 8, .textStyle = Echarts_LabelOption_create(.color = js_string("#111111"))),
        .grid = Echarts_GridComponentOption_create(
            .left = 48, .right = 24, .top = 48, .bottom = 36),
        .xAxis_value = VALUE_AXIS(Echarts_XAXisComponentOption_value),
        .yAxis_value = VALUE_AXIS(Echarts_YAXisComponentOption_value),
        .series_list = series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 20 data points across 2 clusters",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
