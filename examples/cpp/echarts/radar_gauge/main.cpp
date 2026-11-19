#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value color_obj(const char* hex) {
    ts::Value o = ts::Value::object();
    o.set("color", std::string(hex));
    return o;
}

static ts::Value line_wrap(const char* hex) {
    ts::Value o = ts::Value::object();
    o.set("lineStyle", color_obj(hex));
    return o;
}

static ts::Value indicator(const char* name) {
    ts::Value o = ts::Value::object();
    o.set("name", std::string(name));
    o.set("max", 100.0);
    return o;
}

static ts::Array<> nums(std::initializer_list<double> xs) {
    ts::Array<> a = ts::Array<>::create();
    for (double x : xs) a.push(ts::Value::number(x));
    return a;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts radar = echarts::init(*doc.getElementById("chart-radar"));
    echarts::ECharts gauge = echarts::init(*doc.getElementById("chart-gauge"));

    /* ---- Radar ---- */
    ts::Value r_leg = ts::Value::object();
    r_leg.set("bottom", 6.0);
    r_leg.set("textStyle", color_obj("#111111"));

    ts::Array<> r_sa_cols = ts::Array<>::create();
    r_sa_cols.push(ts::Value::string("#ffffff"));
    r_sa_cols.push(ts::Value::string("#f7f7f7"));
    ts::Value r_sa_as = ts::Value::object();
    r_sa_as.set("color", r_sa_cols);
    ts::Value r_sa = ts::Value::object();
    r_sa.set("areaStyle", r_sa_as);

    ts::Array<> inds = ts::Array<>::create();
    for (const char* name : { "Throughput", "Memory", "Startup", "Type Safety", "Bundle" }) {
        inds.push(indicator(name));
    }

    ts::Value r_cfg = ts::Value::object();
    r_cfg.set("radius", std::string("62%"));
    r_cfg.set("axisName", color_obj("#111111"));
    r_cfg.set("splitLine", line_wrap("#d4d4d4"));
    r_cfg.set("splitArea", r_sa);
    r_cfg.set("axisLine", line_wrap("#b5b5b5"));
    r_cfg.set("indicator", inds);

    ts::Value ls0 = color_obj("#111111");
    ls0.set("width", 2.0);
    ts::Value item0 = ts::Value::object();
    item0.set("name", std::string("WASM Target"));
    item0.set("value", nums({ 92, 86, 88, 96, 84 }));
    item0.set("lineStyle", ls0);
    item0.set("itemStyle", color_obj("#111111"));
    item0.set("areaStyle", color_obj("rgba(17,17,17,0.12)"));

    ts::Value ls1 = color_obj("#737373");
    ls1.set("width", 1.5);
    ls1.set("type", std::string("dashed"));
    ts::Value item1 = ts::Value::object();
    item1.set("name", std::string("Baseline"));
    item1.set("value", nums({ 70, 68, 74, 72, 66 }));
    item1.set("lineStyle", ls1);
    item1.set("itemStyle", color_obj("#737373"));

    ts::Array<> r_data = ts::Array<>::create();
    r_data.push(item0);
    r_data.push(item1);
    ts::Value rs0 = ts::Value::object();
    rs0.set("type", std::string("radar"));
    rs0.set("data", r_data);
    ts::Array<> r_series = ts::Array<>::create();
    r_series.push(rs0);

    echarts::EChartsCoreOption radar_opt = echarts::EChartsCoreOption::create();
    radar_opt.animation(false);
    radar_opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    radar_opt.set_at("legend", r_leg);
    radar_opt.set_at("radar", r_cfg);
    radar_opt.set_at("series", ts::Value::of(r_series));
    radar.setOption(radar_opt);

    /* ---- Gauge ---- */
    ts::Value g_prog = ts::Value::object();
    g_prog.set("show", true);
    g_prog.set("width", 14.0);
    g_prog.set("itemStyle", color_obj("#111111"));

    ts::Array<> stop = ts::Array<>::create();
    stop.push(ts::Value::number(1.0));
    stop.push(ts::Value::string("#e5e5e5"));
    ts::Array<> al_stops = ts::Array<>::create();
    al_stops.push(stop);
    ts::Value g_al_ls = ts::Value::object();
    g_al_ls.set("width", 14.0);
    g_al_ls.set("color", al_stops);
    ts::Value g_al = ts::Value::object();
    g_al.set("lineStyle", g_al_ls);

    ts::Value g_ptr = ts::Value::object();
    g_ptr.set("itemStyle", color_obj("#111111"));

    ts::Value g_det = color_obj("#111111");
    g_det.set("valueAnimation", false);
    g_det.set("formatter", std::string("{value}%"));
    g_det.set("fontSize", 22.0);

    ts::Value g_title = color_obj("#555555");
    g_title.set("fontSize", 12.0);

    ts::Value g_datum = ts::Value::object();
    g_datum.set("value", 84.0);
    g_datum.set("name", std::string("Score"));
    ts::Array<> g_data = ts::Array<>::create();
    g_data.push(g_datum);

    ts::Value gs0 = ts::Value::object();
    gs0.set("type", std::string("gauge"));
    gs0.set("min", 0.0);
    gs0.set("max", 100.0);
    gs0.set("progress", g_prog);
    gs0.set("axisLine", g_al);
    gs0.set("axisTick", line_wrap("#737373"));
    gs0.set("splitLine", line_wrap("#111111"));
    gs0.set("axisLabel", color_obj("#555555"));
    gs0.set("pointer", g_ptr);
    gs0.set("detail", g_det);
    gs0.set("title", g_title);
    gs0.set("data", g_data);
    ts::Array<> g_series = ts::Array<>::create();
    g_series.push(gs0);

    echarts::EChartsCoreOption gauge_opt = echarts::EChartsCoreOption::create();
    gauge_opt.animation(false);
    gauge_opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    gauge_opt.set_at("series", ts::Value::of(g_series));
    gauge.setOption(gauge_opt);

    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Radar: " + std::to_string(static_cast<int>(radar.getWidth())) + "x" +
        std::to_string(static_cast<int>(radar.getHeight())) + " \xC2\xB7 Gauge: " +
        std::to_string(static_cast<int>(gauge.getWidth())) + "x" +
        std::to_string(static_cast<int>(gauge.getHeight())) + " \xC2\xB7 Score 84%"
    );
}
