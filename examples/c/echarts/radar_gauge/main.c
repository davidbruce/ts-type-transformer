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

static Dictionary color_obj(const char* hex) {
    Dictionary d = Dictionary_create();
    Dictionary_set_at(d, "color", js_string(hex));
    return d;
}

static Dictionary line_wrap(const char* hex) {
    Dictionary ls = color_obj(hex);
    Dictionary w = Dictionary_create();
    Dictionary_set_at(w, "lineStyle", (JsValue){ ls.h });
    js_release(ls);
    return w;
}

static void push_ind(JsArray arr, const char* name) {
    Dictionary d = Dictionary_create();
    Dictionary_set_at(d, "name", js_string(name));
    Dictionary_set_at(d, "max", js_number(100));
    JsArray_push(arr, (JsValue){ d.h });
    js_release(d);
}

static JsArray num_arr(const double* vals, int n) {
    JsArray a = JsArray_new();
    for (int i = 0; i < n; i++) JsArray_push(a, js_number(vals[i]));
    return a;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement radar_box = Document_getElementById(doc, "chart-radar");
    HTMLElement gauge_box = Document_getElementById(doc, "chart-gauge");
    ECharts radar = init(radar_box, (const char*)0, (EChartsInitOpts){ 0 });
    ECharts gauge = init(gauge_box, (const char*)0, (EChartsInitOpts){ 0 });

    /* ---- Radar ---- */
    Dictionary r_leg_ts = color_obj("#111111");
    Dictionary r_leg = Dictionary_create();
    Dictionary_set_at(r_leg, "bottom", js_number(6));
    Dictionary_set_at(r_leg, "textStyle", (JsValue){ r_leg_ts.h });

    Dictionary r_an = color_obj("#111111");
    Dictionary r_sl = line_wrap("#d4d4d4");
    JsArray r_sa_cols = JsArray_new();
    JsArray_push(r_sa_cols, js_string("#ffffff"));
    JsArray_push(r_sa_cols, js_string("#f7f7f7"));
    Dictionary r_sa_as = Dictionary_create();
    Dictionary_set_at(r_sa_as, "color", (JsValue){ r_sa_cols.h });
    Dictionary r_sa = Dictionary_create();
    Dictionary_set_at(r_sa, "areaStyle", (JsValue){ r_sa_as.h });
    Dictionary r_al = line_wrap("#b5b5b5");

    JsArray inds = JsArray_new();
    push_ind(inds, "Throughput");
    push_ind(inds, "Memory");
    push_ind(inds, "Startup");
    push_ind(inds, "Type Safety");
    push_ind(inds, "Bundle");

    Dictionary r_cfg = Dictionary_create();
    Dictionary_set_at(r_cfg, "radius", js_string("62%"));
    Dictionary_set_at(r_cfg, "axisName", (JsValue){ r_an.h });
    Dictionary_set_at(r_cfg, "splitLine", (JsValue){ r_sl.h });
    Dictionary_set_at(r_cfg, "splitArea", (JsValue){ r_sa.h });
    Dictionary_set_at(r_cfg, "axisLine", (JsValue){ r_al.h });
    Dictionary_set_at(r_cfg, "indicator", (JsValue){ inds.h });

    const double v_wasm[5] = { 92, 86, 88, 96, 84 };
    const double v_base[5] = { 70, 68, 74, 72, 66 };
    JsArray va = num_arr(v_wasm, 5);
    JsArray vb = num_arr(v_base, 5);

    Dictionary ls0 = color_obj("#111111");
    Dictionary_set_at(ls0, "width", js_number(2));
    Dictionary is0 = color_obj("#111111");
    Dictionary as0 = color_obj("rgba(17,17,17,0.12)");
    Dictionary item0 = Dictionary_create();
    Dictionary_set_at(item0, "name", js_string("WASM Target"));
    Dictionary_set_at(item0, "value", (JsValue){ va.h });
    Dictionary_set_at(item0, "lineStyle", (JsValue){ ls0.h });
    Dictionary_set_at(item0, "itemStyle", (JsValue){ is0.h });
    Dictionary_set_at(item0, "areaStyle", (JsValue){ as0.h });

    Dictionary ls1 = color_obj("#737373");
    Dictionary_set_at(ls1, "width", js_number(1.5));
    Dictionary_set_at(ls1, "type", js_string("dashed"));
    Dictionary is1 = color_obj("#737373");
    Dictionary item1 = Dictionary_create();
    Dictionary_set_at(item1, "name", js_string("Baseline"));
    Dictionary_set_at(item1, "value", (JsValue){ vb.h });
    Dictionary_set_at(item1, "lineStyle", (JsValue){ ls1.h });
    Dictionary_set_at(item1, "itemStyle", (JsValue){ is1.h });

    JsArray r_data = JsArray_new();
    JsArray_push(r_data, (JsValue){ item0.h });
    JsArray_push(r_data, (JsValue){ item1.h });
    Dictionary rs0 = Dictionary_create();
    Dictionary_set_at(rs0, "type", js_string("radar"));
    Dictionary_set_at(rs0, "data", (JsValue){ r_data.h });
    JsArray r_series = JsArray_new();
    JsArray_push(r_series, (JsValue){ rs0.h });

    EChartsCoreOption radar_opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(radar_opt, js_some_bool(false));
    ECUnitOption_set_at(radar_opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(radar_opt, "legend", (JsValue){ r_leg.h });
    ECUnitOption_set_at(radar_opt, "radar", (JsValue){ r_cfg.h });
    ECUnitOption_set_at(radar_opt, "series", (JsValue){ r_series.h });
    EChartsType_setOption(radar, radar_opt);

    /* ---- Gauge ---- */
    Dictionary g_prog_is = color_obj("#111111");
    Dictionary g_prog = Dictionary_create();
    Dictionary_set_at(g_prog, "show", js_bool(true));
    Dictionary_set_at(g_prog, "width", js_number(14));
    Dictionary_set_at(g_prog, "itemStyle", (JsValue){ g_prog_is.h });

    JsArray stop = JsArray_new();
    JsArray_push(stop, js_number(1));
    JsArray_push(stop, js_string("#e5e5e5"));
    JsArray al_stops = JsArray_new();
    JsArray_push(al_stops, (JsValue){ stop.h });
    Dictionary g_al_ls = Dictionary_create();
    Dictionary_set_at(g_al_ls, "width", js_number(14));
    Dictionary_set_at(g_al_ls, "color", (JsValue){ al_stops.h });
    Dictionary g_al = Dictionary_create();
    Dictionary_set_at(g_al, "lineStyle", (JsValue){ g_al_ls.h });

    Dictionary g_tick = line_wrap("#737373");
    Dictionary g_split = line_wrap("#111111");
    Dictionary g_lbl = color_obj("#555555");
    Dictionary g_ptr_is = color_obj("#111111");
    Dictionary g_ptr = Dictionary_create();
    Dictionary_set_at(g_ptr, "itemStyle", (JsValue){ g_ptr_is.h });

    Dictionary g_det = color_obj("#111111");
    Dictionary_set_at(g_det, "valueAnimation", js_bool(false));
    Dictionary_set_at(g_det, "formatter", js_string("{value}%"));
    Dictionary_set_at(g_det, "fontSize", js_number(22));

    Dictionary g_title = color_obj("#555555");
    Dictionary_set_at(g_title, "fontSize", js_number(12));

    Dictionary g_datum = Dictionary_create();
    Dictionary_set_at(g_datum, "value", js_number(84));
    Dictionary_set_at(g_datum, "name", js_string("Score"));
    JsArray g_data = JsArray_new();
    JsArray_push(g_data, (JsValue){ g_datum.h });

    Dictionary gs0 = Dictionary_create();
    Dictionary_set_at(gs0, "type", js_string("gauge"));
    Dictionary_set_at(gs0, "min", js_number(0));
    Dictionary_set_at(gs0, "max", js_number(100));
    Dictionary_set_at(gs0, "progress", (JsValue){ g_prog.h });
    Dictionary_set_at(gs0, "axisLine", (JsValue){ g_al.h });
    Dictionary_set_at(gs0, "axisTick", (JsValue){ g_tick.h });
    Dictionary_set_at(gs0, "splitLine", (JsValue){ g_split.h });
    Dictionary_set_at(gs0, "axisLabel", (JsValue){ g_lbl.h });
    Dictionary_set_at(gs0, "pointer", (JsValue){ g_ptr.h });
    Dictionary_set_at(gs0, "detail", (JsValue){ g_det.h });
    Dictionary_set_at(gs0, "title", (JsValue){ g_title.h });
    Dictionary_set_at(gs0, "data", (JsValue){ g_data.h });
    JsArray g_series = JsArray_new();
    JsArray_push(g_series, (JsValue){ gs0.h });

    EChartsCoreOption gauge_opt = EChartsCoreOption_create();
    ECUnitOption_set_animation(gauge_opt, js_some_bool(false));
    ECUnitOption_set_at(gauge_opt, "backgroundColor", js_string("#ffffff"));
    ECUnitOption_set_at(gauge_opt, "series", (JsValue){ g_series.h });
    EChartsType_setOption(gauge, gauge_opt);

    char meta[128];
    uint32_t p = append_cstr(meta, 0, sizeof(meta), "Radar: ");
    p = append_i32(meta, p, sizeof(meta), (int32_t)EChartsType_getWidth(radar));
    p = append_cstr(meta, p, sizeof(meta), "x");
    p = append_i32(meta, p, sizeof(meta), (int32_t)EChartsType_getHeight(radar));
    p = append_cstr(meta, p, sizeof(meta), " \xC2\xB7 Gauge: ");
    p = append_i32(meta, p, sizeof(meta), (int32_t)EChartsType_getWidth(gauge));
    p = append_cstr(meta, p, sizeof(meta), "x");
    p = append_i32(meta, p, sizeof(meta), (int32_t)EChartsType_getHeight(gauge));
    append_cstr(meta, p, sizeof(meta), " \xC2\xB7 Score 84%");
    HTMLElement meta_el = Document_getElementById(doc, "meta");
    Node_set_textContent(meta_el, meta);

    js_release(meta_el);
    js_release(gauge_opt); js_release(g_series); js_release(gs0); js_release(g_data); js_release(g_datum);
    js_release(g_title); js_release(g_det); js_release(g_ptr); js_release(g_ptr_is);
    js_release(g_lbl); js_release(g_split); js_release(g_tick);
    js_release(g_al); js_release(g_al_ls); js_release(al_stops); js_release(stop);
    js_release(g_prog); js_release(g_prog_is);

    js_release(radar_opt); js_release(r_series); js_release(rs0); js_release(r_data);
    js_release(item1); js_release(is1); js_release(ls1); js_release(vb);
    js_release(item0); js_release(as0); js_release(is0); js_release(ls0); js_release(va);
    js_release(r_cfg); js_release(inds); js_release(r_al); js_release(r_sa); js_release(r_sa_as); js_release(r_sa_cols);
    js_release(r_sl); js_release(r_an); js_release(r_leg); js_release(r_leg_ts);
    js_release(gauge); js_release(radar); js_release(gauge_box); js_release(radar_box); js_release(doc);
}
