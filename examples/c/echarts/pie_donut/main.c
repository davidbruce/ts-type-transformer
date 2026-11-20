#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static void push_slice(JsArray data, const char* name, double value) {
    Dictionary d = Dictionary_create();
    js_set_str(d, "name", name);
    js_set_num(d, "value", value);
    js_push_val(data, d);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, NULL);

    JsArray palette = JsArray_new();
    const char* cols[] = { "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" };
    for (int i = 0; i < 5; i++) js_push_str(palette, cols[i]);

    Dictionary leg_ts = Dictionary_create();
    js_set_str(leg_ts, "color", "#111111");
    Dictionary legend = Dictionary_create();
    js_set_num(legend, "bottom", 8);
    js_set_val(legend, "textStyle", leg_ts);

    Dictionary tooltip = Dictionary_create();
    js_set_str(tooltip, "trigger", "item");

    JsArray radius = JsArray_new();
    js_push_str(radius, "44%");
    js_push_str(radius, "72%");

    JsArray center = JsArray_new();
    js_push_str(center, "50%");
    js_push_str(center, "46%");

    Dictionary item_style = Dictionary_create();
    js_set_str(item_style, "borderColor", "#ffffff");
    js_set_num(item_style, "borderWidth", 2);

    Dictionary lbl = Dictionary_create();
    js_set_str(lbl, "color", "#111111");

    JsArray data = JsArray_new();
    push_slice(data, "Runtime", 34);
    push_slice(data, "Bindings", 26);
    push_slice(data, "Glue", 18);
    push_slice(data, "Parser", 14);
    push_slice(data, "Tests", 8);

    Dictionary s0 = Dictionary_create();
    js_set_str(s0, "type", "pie");
    js_set_str(s0, "name", "Subsystems");
    js_set_val(s0, "radius", radius);
    js_set_val(s0, "center", center);
    js_set_val(s0, "itemStyle", item_style);
    js_set_val(s0, "label", lbl);
    js_set_val(s0, "data", data);

    JsArray series = JsArray_new();
    js_push_val(series, s0);

    EChartsCoreOption opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(opt, false);
    js_set_str(opt, "backgroundColor", "#ffffff");
    js_set_val(opt, "color", palette);
    js_set_val(opt, "legend", legend);
    js_set_val(opt, "tooltip", tooltip);
    js_set_val(opt, "series", series);
    ECharts_setOption(chart, opt);

    char meta[96];
    snprintf(meta, sizeof meta, "Canvas: %dx%d \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100",
        (int)ECharts_getWidth(chart), (int)ECharts_getHeight(chart));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
    js_scope_close();
}
