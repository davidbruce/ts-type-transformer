#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_Dictionary make_value_axis(void) {
    Echarts_Dictionary split_ls = Echarts_Dictionary_create();
    js_set_str(split_ls, "color", "#e5e5e5");
    Echarts_Dictionary split = Echarts_Dictionary_create();
    js_set_val(split, "lineStyle", split_ls);
    Echarts_Dictionary lbl = Echarts_Dictionary_create();
    js_set_str(lbl, "color", "#555555");
    Echarts_Dictionary ax = Echarts_Dictionary_create();
    js_set_str(ax, "type", "value");
    js_set_num(ax, "min", 0);
    js_set_num(ax, "max", 100);
    js_set_val(ax, "splitLine", split);
    js_set_val(ax, "axisLabel", lbl);
    return ax;
}

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
    js_scope_open();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    Echarts_Dictionary leg_ts = Echarts_Dictionary_create();
    js_set_str(leg_ts, "color", "#111111");
    Echarts_Dictionary legend = Echarts_Dictionary_create();
    js_set_num(legend, "top", 8);
    js_set_val(legend, "textStyle", leg_ts);

    Echarts_Dictionary grid = Echarts_Dictionary_create();
    js_set_num(grid, "left", 48);
    js_set_num(grid, "right", 24);
    js_set_num(grid, "top", 48);
    js_set_num(grid, "bottom", 36);

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };

    Echarts_Dictionary is_a = Echarts_Dictionary_create();
    js_set_str(is_a, "color", "#111111");
    Echarts_Dictionary s0 = Echarts_Dictionary_create();
    js_set_str(s0, "type", "scatter");
    js_set_str(s0, "name", "Cluster A");
    js_set_str(s0, "symbol", "circle");
    js_set_num(s0, "symbolSize", 10);
    js_set_val(s0, "itemStyle", is_a);
    js_set_val(s0, "data", make_pts(ca_pts, 10));

    Echarts_Dictionary is_b = Echarts_Dictionary_create();
    js_set_str(is_b, "color", "#ffffff");
    js_set_str(is_b, "borderColor", "#111111");
    js_set_num(is_b, "borderWidth", 2);
    Echarts_Dictionary s1 = Echarts_Dictionary_create();
    js_set_str(s1, "type", "scatter");
    js_set_str(s1, "name", "Cluster B");
    js_set_str(s1, "symbol", "rect");
    js_set_num(s1, "symbolSize", 10);
    js_set_val(s1, "itemStyle", is_b);
    js_set_val(s1, "data", make_pts(cb_pts, 10));

    JsArray series = JsArray_new();
    js_push_val(series, s0);
    js_push_val(series, s1);

    EChartsCoreOption opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(opt, false);
    js_set_str(opt, "backgroundColor", "#ffffff");
    js_set_val(opt, "legend", legend);
    js_set_val(opt, "grid", grid);
    js_set_val(opt, "xAxis", make_value_axis());
    js_set_val(opt, "yAxis", make_value_axis());
    js_set_val(opt, "series", series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 20 data points across 2 clusters",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
    js_scope_close();
}
