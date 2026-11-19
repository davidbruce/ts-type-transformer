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

static Dictionary make_color_obj(const char* hex) {
    Dictionary d = Dictionary_create();
    Dictionary_set_at(d, "color", js_string(hex));
    return d;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, (const char*)0, (EChartsInitOpts){ 0 });

    Dictionary leg_ts = make_color_obj("#111111");
    Dictionary legend = Dictionary_create();
    Dictionary_set_at(legend, "top", js_number(8));
    Dictionary_set_at(legend, "textStyle", (JsValue){ leg_ts.h });

    Dictionary tooltip = Dictionary_create();
    Dictionary_set_at(tooltip, "trigger", js_string("axis"));

    Dictionary grid = Dictionary_create();
    Dictionary_set_at(grid, "left", js_number(48));
    Dictionary_set_at(grid, "right", js_number(24));
    Dictionary_set_at(grid, "top", js_number(48));
    Dictionary_set_at(grid, "bottom", js_number(36));

    JsArray weeks = JsArray_new();
    const char* wk[] = { "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" };
    for (int i = 0; i < 8; i++) JsArray_push(weeks, js_string(wk[i]));

    Dictionary x_ls = make_color_obj("#111111");
    Dictionary x_line = Dictionary_create();
    Dictionary_set_at(x_line, "lineStyle", (JsValue){ x_ls.h });
    Dictionary x_lbl = make_color_obj("#555555");
    Dictionary x_axis = Dictionary_create();
    Dictionary_set_at(x_axis, "type", js_string("category"));
    Dictionary_set_at(x_axis, "boundaryGap", js_bool(false));
    Dictionary_set_at(x_axis, "data", (JsValue){ weeks.h });
    Dictionary_set_at(x_axis, "axisLine", (JsValue){ x_line.h });
    Dictionary_set_at(x_axis, "axisLabel", (JsValue){ x_lbl.h });

    Dictionary y_ls = make_color_obj("#e5e5e5");
    Dictionary y_split = Dictionary_create();
    Dictionary_set_at(y_split, "lineStyle", (JsValue){ y_ls.h });
    Dictionary y_lbl = make_color_obj("#555555");
    Dictionary y_axis = Dictionary_create();
    Dictionary_set_at(y_axis, "type", js_string("value"));
    Dictionary_set_at(y_axis, "min", js_number(0));
    Dictionary_set_at(y_axis, "max", js_number(100));
    Dictionary_set_at(y_axis, "splitLine", (JsValue){ y_split.h });
    Dictionary_set_at(y_axis, "axisLabel", (JsValue){ y_lbl.h });

    JsArray vals0 = JsArray_new();
    double d0[] = { 28, 44, 39, 62, 58, 76, 81, 89 };
    for (int i = 0; i < 8; i++) JsArray_push(vals0, js_number(d0[i]));
    Dictionary ls0 = make_color_obj("#111111");
    Dictionary_set_at(ls0, "width", js_number(2));
    Dictionary is0 = make_color_obj("#111111");
    Dictionary as0 = make_color_obj("rgba(17,17,17,0.12)");
    Dictionary s0 = Dictionary_create();
    Dictionary_set_at(s0, "type", js_string("line"));
    Dictionary_set_at(s0, "name", js_string("Throughput"));
    Dictionary_set_at(s0, "smooth", js_bool(true));
    Dictionary_set_at(s0, "data", (JsValue){ vals0.h });
    Dictionary_set_at(s0, "lineStyle", (JsValue){ ls0.h });
    Dictionary_set_at(s0, "itemStyle", (JsValue){ is0.h });
    Dictionary_set_at(s0, "areaStyle", (JsValue){ as0.h });

    JsArray vals1 = JsArray_new();
    double d1[] = { 35, 38, 42, 46, 50, 54, 58, 62 };
    for (int i = 0; i < 8; i++) JsArray_push(vals1, js_number(d1[i]));
    Dictionary ls1 = make_color_obj("#737373");
    Dictionary_set_at(ls1, "width", js_number(1.5));
    Dictionary_set_at(ls1, "type", js_string("dashed"));
    Dictionary is1 = make_color_obj("#737373");
    Dictionary s1 = Dictionary_create();
    Dictionary_set_at(s1, "type", js_string("line"));
    Dictionary_set_at(s1, "name", js_string("Baseline"));
    Dictionary_set_at(s1, "smooth", js_bool(false));
    Dictionary_set_at(s1, "data", (JsValue){ vals1.h });
    Dictionary_set_at(s1, "lineStyle", (JsValue){ ls1.h });
    Dictionary_set_at(s1, "itemStyle", (JsValue){ is1.h });

    JsArray series = JsArray_new();
    JsArray_push(series, (JsValue){ s0.h });
    JsArray_push(series, (JsValue){ s1.h });

    EChartsCoreOption opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(opt, js_some_bool(false));
    ECUnitOption_set_at(opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(opt, "legend", (JsValue){ legend.h });
    ECUnitOption_set_at(opt, "tooltip", (JsValue){ tooltip.h });
    ECUnitOption_set_at(opt, "grid", (JsValue){ grid.h });
    ECUnitOption_set_at(opt, "xAxis", (JsValue){ x_axis.h });
    ECUnitOption_set_at(opt, "yAxis", (JsValue){ y_axis.h });
    ECUnitOption_set_at(opt, "series", (JsValue){ series.h });
    EChartsType_setOption(chart, opt);

    int32_t w = (int32_t)EChartsType_getWidth(chart);
    int32_t h = (int32_t)EChartsType_getHeight(chart);
    char meta[96];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Canvas: ");
    p = append_i32(meta, p, sizeof(meta), w);
    p = append_cstr(meta, p, sizeof(meta), "x");
    p = append_i32(meta, p, sizeof(meta), h);
    append_cstr(meta, p, sizeof(meta), " \xC2\xB7 2 series (Throughput + Baseline)");
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);

    js_release(meta_el);
    js_release(opt);
    js_release(series);
    js_release(s1); js_release(is1); js_release(ls1); js_release(vals1);
    js_release(s0); js_release(as0); js_release(is0); js_release(ls0); js_release(vals0);
    js_release(y_axis); js_release(y_lbl); js_release(y_split); js_release(y_ls);
    js_release(x_axis); js_release(x_lbl); js_release(x_line); js_release(x_ls); js_release(weeks);
    js_release(grid); js_release(tooltip); js_release(legend); js_release(leg_ts);
    js_release(chart);
    js_release(box);
    js_release(doc);
}
