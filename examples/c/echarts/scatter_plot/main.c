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

static Dictionary make_value_axis(void) {
    Dictionary split_ls = Dictionary_create();
    Dictionary_set_at(split_ls, "color", js_string("#e5e5e5"));
    Dictionary split = Dictionary_create();
    Dictionary_set_at(split, "lineStyle", (JsValue){ split_ls.h });
    Dictionary lbl = Dictionary_create();
    Dictionary_set_at(lbl, "color", js_string("#555555"));
    Dictionary ax = Dictionary_create();
    Dictionary_set_at(ax, "type", js_string("value"));
    Dictionary_set_at(ax, "min", js_number(0));
    Dictionary_set_at(ax, "max", js_number(100));
    Dictionary_set_at(ax, "splitLine", (JsValue){ split.h });
    Dictionary_set_at(ax, "axisLabel", (JsValue){ lbl.h });
    js_release(lbl);
    js_release(split);
    js_release(split_ls);
    return ax;
}

static JsArray make_pts(const double pts[][2], int count) {
    JsArray arr = JsArray_new();
    for (int i = 0; i < count; i++) {
        JsArray pair = JsArray_new();
        JsArray_push(pair, js_number(pts[i][0]));
        JsArray_push(pair, js_number(pts[i][1]));
        JsArray_push(arr, (JsValue){ pair.h });
        js_release(pair);
    }
    return arr;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement box = Document_getElementById(doc, "chart");
    ECharts chart = init(box, (const char*)0, (EChartsInitOpts){ 0 });

    Dictionary leg_ts = Dictionary_create();
    Dictionary_set_at(leg_ts, "color", js_string("#111111"));
    Dictionary legend = Dictionary_create();
    Dictionary_set_at(legend, "top", js_number(8));
    Dictionary_set_at(legend, "textStyle", (JsValue){ leg_ts.h });

    Dictionary grid = Dictionary_create();
    Dictionary_set_at(grid, "left", js_number(48));
    Dictionary_set_at(grid, "right", js_number(24));
    Dictionary_set_at(grid, "top", js_number(48));
    Dictionary_set_at(grid, "bottom", js_number(36));

    Dictionary x_axis = make_value_axis();
    Dictionary y_axis = make_value_axis();

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };
    JsArray data_a = make_pts(ca_pts, 10);
    JsArray data_b = make_pts(cb_pts, 10);

    Dictionary is_a = Dictionary_create();
    Dictionary_set_at(is_a, "color", js_string("#111111"));
    Dictionary s0 = Dictionary_create();
    Dictionary_set_at(s0, "type", js_string("scatter"));
    Dictionary_set_at(s0, "name", js_string("Cluster A"));
    Dictionary_set_at(s0, "symbol", js_string("circle"));
    Dictionary_set_at(s0, "symbolSize", js_number(10));
    Dictionary_set_at(s0, "itemStyle", (JsValue){ is_a.h });
    Dictionary_set_at(s0, "data", (JsValue){ data_a.h });

    Dictionary is_b = Dictionary_create();
    Dictionary_set_at(is_b, "color", js_string("#ffffff"));
    Dictionary_set_at(is_b, "borderColor", js_string("#111111"));
    Dictionary_set_at(is_b, "borderWidth", js_number(2));
    Dictionary s1 = Dictionary_create();
    Dictionary_set_at(s1, "type", js_string("scatter"));
    Dictionary_set_at(s1, "name", js_string("Cluster B"));
    Dictionary_set_at(s1, "symbol", js_string("rect"));
    Dictionary_set_at(s1, "symbolSize", js_number(10));
    Dictionary_set_at(s1, "itemStyle", (JsValue){ is_b.h });
    Dictionary_set_at(s1, "data", (JsValue){ data_b.h });

    JsArray series = JsArray_new();
    JsArray_push(series, (JsValue){ s0.h });
    JsArray_push(series, (JsValue){ s1.h });

    EChartsCoreOption opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(opt, js_some_bool(false));
    ECUnitOption_set_at(opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(opt, "legend", (JsValue){ legend.h });
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
    append_cstr(meta, p, sizeof(meta), " \xC2\xB7 20 data points across 2 clusters");
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);

    js_release(meta_el);
    js_release(opt);
    js_release(series); js_release(s1); js_release(is_b); js_release(data_b);
    js_release(s0); js_release(is_a); js_release(data_a);
    js_release(y_axis); js_release(x_axis); js_release(grid); js_release(legend); js_release(leg_ts);
    js_release(chart); js_release(box); js_release(doc);
}
