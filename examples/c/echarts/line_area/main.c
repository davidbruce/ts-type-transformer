#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_Dictionary make_color_obj(const char* hex) {
    Echarts_Dictionary d = Echarts_Dictionary_create();
    js_set_str(d, "color", hex);
    return d;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = echarts_init(box, NULL);

    Echarts_Dictionary legend = Echarts_Dictionary_create();
    js_set_num(legend, "top", 8);
    js_set_val(legend, "textStyle", make_color_obj("#111111"));

    Echarts_Dictionary tooltip = Echarts_Dictionary_create();
    js_set_str(tooltip, "trigger", "axis");

    Echarts_Dictionary grid = Echarts_Dictionary_create();
    js_set_num(grid, "left", 48);
    js_set_num(grid, "right", 24);
    js_set_num(grid, "top", 48);
    js_set_num(grid, "bottom", 36);

    JsArray weeks = JsArray_new();
    const char* wk[] = { "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" };
    for (int i = 0; i < 8; i++) js_push_str(weeks, wk[i]);

    Echarts_Dictionary x_line = Echarts_Dictionary_create();
    js_set_val(x_line, "lineStyle", make_color_obj("#111111"));
    Echarts_Dictionary x_axis = Echarts_Dictionary_create();
    js_set_str(x_axis, "type", "category");
    js_set_bool(x_axis, "boundaryGap", false);
    js_set_val(x_axis, "data", weeks);
    js_set_val(x_axis, "axisLine", x_line);
    js_set_val(x_axis, "axisLabel", make_color_obj("#555555"));

    Echarts_Dictionary y_split = Echarts_Dictionary_create();
    js_set_val(y_split, "lineStyle", make_color_obj("#e5e5e5"));
    Echarts_Dictionary y_axis = Echarts_Dictionary_create();
    js_set_str(y_axis, "type", "value");
    js_set_num(y_axis, "min", 0);
    js_set_num(y_axis, "max", 100);
    js_set_val(y_axis, "splitLine", y_split);
    js_set_val(y_axis, "axisLabel", make_color_obj("#555555"));

    JsArray vals0 = JsArray_new();
    double d0[] = { 28, 44, 39, 62, 58, 76, 81, 89 };
    for (int i = 0; i < 8; i++) js_push_num(vals0, d0[i]);
    Echarts_Dictionary ls0 = make_color_obj("#111111");
    js_set_num(ls0, "width", 2);
    Echarts_Dictionary s0 = Echarts_Dictionary_create();
    js_set_str(s0, "type", "line");
    js_set_str(s0, "name", "Throughput");
    js_set_bool(s0, "smooth", true);
    js_set_val(s0, "data", vals0);
    js_set_val(s0, "lineStyle", ls0);
    js_set_val(s0, "itemStyle", make_color_obj("#111111"));
    js_set_val(s0, "areaStyle", make_color_obj("rgba(17,17,17,0.12)"));

    JsArray vals1 = JsArray_new();
    double d1[] = { 35, 38, 42, 46, 50, 54, 58, 62 };
    for (int i = 0; i < 8; i++) js_push_num(vals1, d1[i]);
    Echarts_Dictionary ls1 = make_color_obj("#737373");
    js_set_num(ls1, "width", 1.5);
    js_set_str(ls1, "type", "dashed");
    Echarts_Dictionary s1 = Echarts_Dictionary_create();
    js_set_str(s1, "type", "line");
    js_set_str(s1, "name", "Baseline");
    js_set_bool(s1, "smooth", false);
    js_set_val(s1, "data", vals1);
    js_set_val(s1, "lineStyle", ls1);
    js_set_val(s1, "itemStyle", make_color_obj("#737373"));

    JsArray series = JsArray_new();
    js_push_val(series, s0);
    js_push_val(series, s1);

    EChartsCoreOption opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(opt, false);
    js_set_str(opt, "backgroundColor", "#ffffff");
    js_set_val(opt, "legend", legend);
    js_set_val(opt, "tooltip", tooltip);
    js_set_val(opt, "grid", grid);
    js_set_val(opt, "xAxis", x_axis);
    js_set_val(opt, "yAxis", y_axis);
    js_set_val(opt, "series", series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 2 series (Throughput + Baseline)",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
    js_scope_close();
}
