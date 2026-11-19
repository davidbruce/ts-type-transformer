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
    if (v == 0) {
        tmp[len++] = '0';
    } else {
        while (v > 0 && len < sizeof(tmp)) {
            tmp[len++] = (char)('0' + (v % 10));
            v /= 10;
        }
    }
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) dst[pos++] = tmp[len - 1 - i];
    dst[pos] = '\0';
    return pos;
}

static Dictionary line_style_color(const char* hex) {
    Dictionary inner = Dictionary_create();
    Dictionary_set_at(inner, "color", js_string(hex));
    Dictionary outer = Dictionary_create();
    Dictionary_set_at(outer, "lineStyle", (JsValue){ inner.h });
    js_release(inner);
    return outer;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, (const char*)0, (EChartsInitOpts){ 0 });

    Dictionary title_style = Dictionary_create();
    Dictionary_set_at(title_style, "color", js_string("#111111"));
    Dictionary_set_at(title_style, "fontSize", js_number(13));
    Dictionary title = Dictionary_create();
    Dictionary_set_at(title, "text", js_string("Monthly Output"));
    Dictionary_set_at(title, "left", js_number(16));
    Dictionary_set_at(title, "top", js_number(12));
    Dictionary_set_at(title, "textStyle", (JsValue){ title_style.h });

    Dictionary grid = Dictionary_create();
    Dictionary_set_at(grid, "left", js_number(48));
    Dictionary_set_at(grid, "right", js_number(24));
    Dictionary_set_at(grid, "top", js_number(48));
    Dictionary_set_at(grid, "bottom", js_number(36));

    JsArray cats = JsArray_new();
    const char* months[] = { "Jan", "Feb", "Mar", "Apr", "May", "Jun" };
    for (int i = 0; i < 6; i++) JsArray_push(cats, js_string(months[i]));

    Dictionary x_line = line_style_color("#111111");
    Dictionary x_lbl = Dictionary_create();
    Dictionary_set_at(x_lbl, "color", js_string("#555555"));
    Dictionary x_axis = Dictionary_create();
    Dictionary_set_at(x_axis, "type", js_string("category"));
    Dictionary_set_at(x_axis, "data", (JsValue){ cats.h });
    Dictionary_set_at(x_axis, "axisLine", (JsValue){ x_line.h });
    Dictionary_set_at(x_axis, "axisLabel", (JsValue){ x_lbl.h });

    Dictionary y_split = line_style_color("#e5e5e5");
    Dictionary y_lbl = Dictionary_create();
    Dictionary_set_at(y_lbl, "color", js_string("#555555"));
    Dictionary y_axis = Dictionary_create();
    Dictionary_set_at(y_axis, "type", js_string("value"));
    Dictionary_set_at(y_axis, "min", js_number(0));
    Dictionary_set_at(y_axis, "max", js_number(100));
    Dictionary_set_at(y_axis, "splitLine", (JsValue){ y_split.h });
    Dictionary_set_at(y_axis, "axisLabel", (JsValue){ y_lbl.h });

    JsArray vals = JsArray_new();
    double nums[] = { 42, 68, 55, 84, 71, 93 };
    for (int i = 0; i < 6; i++) JsArray_push(vals, js_number(nums[i]));

    Dictionary item_style = Dictionary_create();
    Dictionary_set_at(item_style, "color", js_string("#111111"));
    Dictionary s0 = Dictionary_create();
    Dictionary_set_at(s0, "type", js_string("bar"));
    Dictionary_set_at(s0, "name", js_string("Units"));
    Dictionary_set_at(s0, "barWidth", js_string("46%"));
    Dictionary_set_at(s0, "data", (JsValue){ vals.h });
    Dictionary_set_at(s0, "itemStyle", (JsValue){ item_style.h });
    JsArray series = JsArray_new();
    JsArray_push(series, (JsValue){ s0.h });

    EChartsCoreOption opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(opt, js_some_bool(false));
    ECUnitOption_set_at(opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(opt, "title", (JsValue){ title.h });
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
    append_cstr(meta, p, sizeof(meta), " \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111");
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);

    js_release(meta_el);
    js_release(opt);
    js_release(series);
    js_release(s0);
    js_release(item_style);
    js_release(vals);
    js_release(y_axis);
    js_release(y_lbl);
    js_release(y_split);
    js_release(x_axis);
    js_release(x_lbl);
    js_release(x_line);
    js_release(cats);
    js_release(grid);
    js_release(title);
    js_release(title_style);
    js_release(chart);
    js_release(box);
    js_release(doc);
}
