#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

#define Position(x) Echarts_PositionSizeOption_of(x)
#define Bound(x) Echarts_AngleAxisComponentOption_category_min_of(x)

static Echarts_LineStyleOption line_color(const char* hex) {
    return Echarts_LineStyleOption_create(.color = Echarts_Color_of(hex));
}

#define VALUE_AXIS(T) T##_create( \
    .min = Bound(0), .max = Bound(100), \
    .splitLine = Echarts_SplitLineOption_create(.lineStyle = line_color("#e5e5e5")), \
    .axisLabel = Echarts_AxisLabelOption_create(.color = js_string("#555555")))

static JsArray make_pts(const double pts[][2], int count) {
    JsArray arr = JsArray_new();
    for (int i = 0; i < count; i++) {
        JsArray pair = JsArray_new();
        js_push_num(pair, pts[i][0]);
        js_push_num(pair, pts[i][1]);
        js_push_val(arr, pair);
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
        .name = Echarts_OptionName_of("Cluster A"), .symbol = Echarts_string_or_SymbolCallback_of("circle"), .symbolSize = Echarts_number_or_number_array_or_SymbolSizeCallback_of(10),
        .itemStyle = Echarts_ItemStyleOption_create(.color = Echarts_ItemStyleOption_color_of("#111111")),
        .data = Echarts_ScatterSeriesOption_data_of(make_pts(ca_pts, 10)));
    Echarts_ScatterSeriesOption s1 = Echarts_ScatterSeriesOption_create(
        .name = Echarts_OptionName_of("Cluster B"), .symbol = Echarts_string_or_SymbolCallback_of("rect"), .symbolSize = Echarts_number_or_number_array_or_SymbolSizeCallback_of(10),
        .itemStyle = Echarts_ItemStyleOption_create(
            .color = Echarts_ItemStyleOption_color_of("#ffffff"), .borderColor = Echarts_Color_of("#111111"), .borderWidth = 2),
        .data = Echarts_ScatterSeriesOption_data_of(make_pts(cb_pts, 10)));

    JsArray series = JsArray_new();
    js_push_val(series, s0);
    js_push_val(series, s1);

    EChartsOption opt = EChartsOption_create(
        .animation = false,
        .backgroundColor = Echarts_Color_of("#ffffff"),
        .legend = EChartsOption_legend_of(Echarts_PlainLegendComponentOption_create(
            .top = Position(8), .textStyle = Echarts_LabelOption_create(.color = js_string("#111111")))),
        .grid = EChartsOption_grid_of(Echarts_GridComponentOption_create(
            .left = Position(48), .right = Position(24), .top = Position(48), .bottom = Position(36))),
        .xAxis = EChartsOption_xAxis_of(VALUE_AXIS(Echarts_XAXisComponentOption_xAxis_value)),
        .yAxis = EChartsOption_yAxis_of(VALUE_AXIS(Echarts_YAXisComponentOption_yAxis_value)),
        .series = EChartsOption_series_of(series));
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 20 data points across 2 clusters",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
}
