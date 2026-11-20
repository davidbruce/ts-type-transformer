#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Dictionary line_style_color(const char* hex) {
    Dictionary inner = Dictionary_create();
    js_set_str(inner, "color", hex);
    Dictionary outer = Dictionary_create();
    js_set_val(outer, "lineStyle", inner);
    return outer;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, NULL);

    Dictionary title_style = Dictionary_create();
    js_set_str(title_style, "color", "#111111");
    js_set_num(title_style, "fontSize", 13);
    Dictionary title = Dictionary_create();
    js_set_str(title, "text", "Monthly Output");
    js_set_num(title, "left", 16);
    js_set_num(title, "top", 12);
    js_set_val(title, "textStyle", title_style);

    Dictionary grid = Dictionary_create();
    js_set_num(grid, "left", 48);
    js_set_num(grid, "right", 24);
    js_set_num(grid, "top", 48);
    js_set_num(grid, "bottom", 36);

    JsArray cats = JsArray_new();
    const char* months[] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun" };
    for (int i = 0; i < 6; i++) js_push_str(cats, months[i]);

    Dictionary x_lbl = Dictionary_create();
    js_set_str(x_lbl, "color", "#555555");
    Dictionary x_axis = Dictionary_create();
    js_set_str(x_axis, "type", "category");
    js_set_val(x_axis, "data", cats);
    js_set_val(x_axis, "axisLine", line_style_color("#111111"));
    js_set_val(x_axis, "axisLabel", x_lbl);

    Dictionary y_lbl = Dictionary_create();
    js_set_str(y_lbl, "color", "#555555");
    Dictionary y_axis = Dictionary_create();
    js_set_str(y_axis, "type", "value");
    js_set_num(y_axis, "min", 0);
    js_set_num(y_axis, "max", 100);
    js_set_val(y_axis, "splitLine", line_style_color("#e5e5e5"));
    js_set_val(y_axis, "axisLabel", y_lbl);

    JsArray vals = JsArray_new();
    double nums[] = { 42, 68, 55, 84, 71, 93 };
    for (int i = 0; i < 6; i++) js_push_num(vals, nums[i]);

    Dictionary item_style = Dictionary_create();
    js_set_str(item_style, "color", "#111111");
    Dictionary s0 = Dictionary_create();
    js_set_str(s0, "type", "bar");
    js_set_str(s0, "name", "Units");
    js_set_str(s0, "barWidth", "46%");
    js_set_val(s0, "data", vals);
    js_set_val(s0, "itemStyle", item_style);
    JsArray series = JsArray_new();
    js_push_val(series, s0);

    EChartsCoreOption opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(opt, false);
    js_set_str(opt, "backgroundColor", "#ffffff");
    js_set_val(opt, "title", title);
    js_set_val(opt, "grid", grid);
    js_set_val(opt, "xAxis", x_axis);
    js_set_val(opt, "yAxis", y_axis);
    js_set_val(opt, "series", series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
    js_scope_close();
}
