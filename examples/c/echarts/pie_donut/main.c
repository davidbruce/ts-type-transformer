#include "dom/dom.h"
#include "echarts/echarts.h"

static uint32_t append_cstr(char* dst, uint32_t pos, uint32_t cap, const char* src) {
    for (uint32_t i = 0; src[i] && pos + 1 < cap; i++) dst[pos++] = src[i];
    dst[pos] = '\0';
    return pos;
}

static uint32_t append_i32(char* dst, uint32_t pos, uint32_t cap, int32_t v) {
    char tmp[16];
    uint32_t len = 0;
    if (v == 0) tmp[len++] = '0';
    else while (v > 0 && len < sizeof(tmp)) { tmp[len++] = (char)('0' + (v % 10)); v /= 10; }
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) dst[pos++] = tmp[len - 1 - i];
    dst[pos] = '\0';
    return pos;
}

static void push_slice(JsArray data, const char* name, double value) {
    Dictionary d = Dictionary_create();
    Dictionary_set_at(d, "name", js_string(name));
    Dictionary_set_at(d, "value", js_number(value));
    JsArray_push(data, (JsValue){ d.h });
    js_release(d);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, (const char*)0, (EChartsInitOpts){ 0 });

    JsArray palette = JsArray_new();
    const char* cols[] = { "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" };
    for (int i = 0; i < 5; i++) JsArray_push(palette, js_string(cols[i]));

    Dictionary leg_ts = Dictionary_create();
    Dictionary_set_at(leg_ts, "color", js_string("#111111"));
    Dictionary legend = Dictionary_create();
    Dictionary_set_at(legend, "bottom", js_number(8));
    Dictionary_set_at(legend, "textStyle", (JsValue){ leg_ts.h });

    Dictionary tooltip = Dictionary_create();
    Dictionary_set_at(tooltip, "trigger", js_string("item"));

    JsArray radius = JsArray_new();
    JsArray_push(radius, js_string("44%"));
    JsArray_push(radius, js_string("72%"));

    JsArray center = JsArray_new();
    JsArray_push(center, js_string("50%"));
    JsArray_push(center, js_string("46%"));

    Dictionary item_style = Dictionary_create();
    Dictionary_set_at(item_style, "borderColor", js_string("#ffffff"));
    Dictionary_set_at(item_style, "borderWidth", js_number(2));

    Dictionary lbl = Dictionary_create();
    Dictionary_set_at(lbl, "color", js_string("#111111"));

    JsArray data = JsArray_new();
    push_slice(data, "Runtime", 34);
    push_slice(data, "Bindings", 26);
    push_slice(data, "Glue", 18);
    push_slice(data, "Parser", 14);
    push_slice(data, "Tests", 8);

    Dictionary s0 = Dictionary_create();
    Dictionary_set_at(s0, "type", js_string("pie"));
    Dictionary_set_at(s0, "name", js_string("Subsystems"));
    Dictionary_set_at(s0, "radius", (JsValue){ radius.h });
    Dictionary_set_at(s0, "center", (JsValue){ center.h });
    Dictionary_set_at(s0, "itemStyle", (JsValue){ item_style.h });
    Dictionary_set_at(s0, "label", (JsValue){ lbl.h });
    Dictionary_set_at(s0, "data", (JsValue){ data.h });

    JsArray series = JsArray_new();
    JsArray_push(series, (JsValue){ s0.h });

    EChartsCoreOption opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(opt, js_some_bool(false));
    ECUnitOption_set_at(opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(opt, "color", (JsValue){ palette.h });
    ECUnitOption_set_at(opt, "legend", (JsValue){ legend.h });
    ECUnitOption_set_at(opt, "tooltip", (JsValue){ tooltip.h });
    ECUnitOption_set_at(opt, "series", (JsValue){ series.h });
    EChartsType_setOption(chart, opt);

    int32_t w = (int32_t)EChartsType_getWidth(chart);
    int32_t h = (int32_t)EChartsType_getHeight(chart);
    char meta[96];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Canvas: ");
    p = append_i32(meta, p, sizeof(meta), w);
    p = append_cstr(meta, p, sizeof(meta), "x");
    p = append_i32(meta, p, sizeof(meta), h);
    append_cstr(meta, p, sizeof(meta), " \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100");
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);

    js_release(meta_el);
    js_release(opt);
    js_release(series); js_release(s0); js_release(data); js_release(lbl);
    js_release(item_style); js_release(center); js_release(radius);
    js_release(tooltip); js_release(legend); js_release(leg_ts); js_release(palette);
    js_release(chart);
    js_release(box);
    js_release(doc);
}
