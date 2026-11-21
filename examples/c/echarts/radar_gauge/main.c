#include "dom/dom.h"
#include "echarts/echarts.h"
#include <stdio.h>

static Echarts_Dictionary color_obj(const char* hex) {
    Echarts_Dictionary d = Echarts_Dictionary_create();
    js_set_str(d, "color", hex);
    return d;
}

static Echarts_Dictionary line_wrap(const char* hex) {
    Echarts_Dictionary w = Echarts_Dictionary_create();
    js_set_val(w, "lineStyle", color_obj(hex));
    return w;
}

static void push_ind(JsArray arr, const char* name) {
    Echarts_Dictionary d = Echarts_Dictionary_create();
    js_set_str(d, "name", name);
    js_set_num(d, "max", 100);
    js_push_val(arr, d);
}

static JsArray num_arr(const double* vals, int n) {
    JsArray a = JsArray_new();
    for (int i = 0; i < n; i++) js_push_num(a, vals[i]);
    return a;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    ECharts radar = echarts_init(Document_getElementById(doc, "chart-radar"), NULL);
    ECharts gauge = echarts_init(Document_getElementById(doc, "chart-gauge"), NULL);

    /* ---- Radar ---- */
    Echarts_Dictionary r_leg = Echarts_Dictionary_create();
    js_set_num(r_leg, "bottom", 6);
    js_set_val(r_leg, "textStyle", color_obj("#111111"));

    JsArray r_sa_cols = JsArray_new();
    js_push_str(r_sa_cols, "#ffffff");
    js_push_str(r_sa_cols, "#f7f7f7");
    Echarts_Dictionary r_sa_as = Echarts_Dictionary_create();
    js_set_val(r_sa_as, "color", r_sa_cols);
    Echarts_Dictionary r_sa = Echarts_Dictionary_create();
    js_set_val(r_sa, "areaStyle", r_sa_as);

    JsArray inds = JsArray_new();
    push_ind(inds, "Throughput");
    push_ind(inds, "Memory");
    push_ind(inds, "Startup");
    push_ind(inds, "Type Safety");
    push_ind(inds, "Bundle");

    Echarts_Dictionary r_cfg = Echarts_Dictionary_create();
    js_set_str(r_cfg, "radius", "62%");
    js_set_val(r_cfg, "axisName", color_obj("#111111"));
    js_set_val(r_cfg, "splitLine", line_wrap("#d4d4d4"));
    js_set_val(r_cfg, "splitArea", r_sa);
    js_set_val(r_cfg, "axisLine", line_wrap("#b5b5b5"));
    js_set_val(r_cfg, "indicator", inds);

    const double v_wasm[5] = { 92, 86, 88, 96, 84 };
    const double v_base[5] = { 70, 68, 74, 72, 66 };

    Echarts_Dictionary ls0 = color_obj("#111111");
    js_set_num(ls0, "width", 2);
    Echarts_Dictionary item0 = Echarts_Dictionary_create();
    js_set_str(item0, "name", "WASM Target");
    js_set_val(item0, "value", num_arr(v_wasm, 5));
    js_set_val(item0, "lineStyle", ls0);
    js_set_val(item0, "itemStyle", color_obj("#111111"));
    js_set_val(item0, "areaStyle", color_obj("rgba(17,17,17,0.12)"));

    Echarts_Dictionary ls1 = color_obj("#737373");
    js_set_num(ls1, "width", 1.5);
    js_set_str(ls1, "type", "dashed");
    Echarts_Dictionary item1 = Echarts_Dictionary_create();
    js_set_str(item1, "name", "Baseline");
    js_set_val(item1, "value", num_arr(v_base, 5));
    js_set_val(item1, "lineStyle", ls1);
    js_set_val(item1, "itemStyle", color_obj("#737373"));

    JsArray r_data = JsArray_new();
    js_push_val(r_data, item0);
    js_push_val(r_data, item1);
    Echarts_Dictionary rs0 = Echarts_Dictionary_create();
    js_set_str(rs0, "type", "radar");
    js_set_val(rs0, "data", r_data);
    JsArray r_series = JsArray_new();
    js_push_val(r_series, rs0);

    EChartsCoreOption radar_opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(radar_opt, false);
    js_set_str(radar_opt, "backgroundColor", "#ffffff");
    js_set_val(radar_opt, "legend", r_leg);
    js_set_val(radar_opt, "radar", r_cfg);
    js_set_val(radar_opt, "series", r_series);
    ECharts_setOption(radar, radar_opt);

    /* ---- Gauge ---- */
    Echarts_Dictionary g_prog = Echarts_Dictionary_create();
    js_set_bool(g_prog, "show", true);
    js_set_num(g_prog, "width", 14);
    js_set_val(g_prog, "itemStyle", color_obj("#111111"));

    JsArray stop = JsArray_new();
    js_push_num(stop, 1);
    js_push_str(stop, "#e5e5e5");
    JsArray al_stops = JsArray_new();
    js_push_val(al_stops, stop);
    Echarts_Dictionary g_al_ls = Echarts_Dictionary_create();
    js_set_num(g_al_ls, "width", 14);
    js_set_val(g_al_ls, "color", al_stops);
    Echarts_Dictionary g_al = Echarts_Dictionary_create();
    js_set_val(g_al, "lineStyle", g_al_ls);

    Echarts_Dictionary g_ptr = Echarts_Dictionary_create();
    js_set_val(g_ptr, "itemStyle", color_obj("#111111"));

    Echarts_Dictionary g_det = color_obj("#111111");
    js_set_bool(g_det, "valueAnimation", false);
    js_set_str(g_det, "formatter", "{value}%");
    js_set_num(g_det, "fontSize", 22);

    Echarts_Dictionary g_title = color_obj("#555555");
    js_set_num(g_title, "fontSize", 12);

    Echarts_Dictionary g_datum = Echarts_Dictionary_create();
    js_set_num(g_datum, "value", 84);
    js_set_str(g_datum, "name", "Score");
    JsArray g_data = JsArray_new();
    js_push_val(g_data, g_datum);

    Echarts_Dictionary gs0 = Echarts_Dictionary_create();
    js_set_str(gs0, "type", "gauge");
    js_set_num(gs0, "min", 0);
    js_set_num(gs0, "max", 100);
    js_set_val(gs0, "progress", g_prog);
    js_set_val(gs0, "axisLine", g_al);
    js_set_val(gs0, "axisTick", line_wrap("#737373"));
    js_set_val(gs0, "splitLine", line_wrap("#111111"));
    js_set_val(gs0, "axisLabel", color_obj("#555555"));
    js_set_val(gs0, "pointer", g_ptr);
    js_set_val(gs0, "detail", g_det);
    js_set_val(gs0, "title", g_title);
    js_set_val(gs0, "data", g_data);
    JsArray g_series = JsArray_new();
    js_push_val(g_series, gs0);

    EChartsCoreOption gauge_opt = EChartsCoreOption_create();
    EChartsCoreOption_set_animation(gauge_opt, false);
    js_set_str(gauge_opt, "backgroundColor", "#ffffff");
    js_set_val(gauge_opt, "series", g_series);
    ECharts_setOption(gauge, gauge_opt);

    char meta[128];
    snprintf(meta, sizeof meta, "Radar: %dx%d \xC2\xB7 Gauge: %dx%d \xC2\xB7 Score 84%%",
        (int)ECharts_getWidth(radar), (int)ECharts_getHeight(radar),
        (int)ECharts_getWidth(gauge), (int)ECharts_getHeight(gauge));
    HTMLElement_set_textContent(Document_getElementById(doc, "meta"), meta);
    js_scope_close();
}
