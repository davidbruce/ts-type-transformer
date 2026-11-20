#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value color_obj(const char* hex) {
    return ts::Value::object().set("color", hex);
}

static ts::Value line_wrap(const char* hex) {
    return ts::Value::object().set("lineStyle", color_obj(hex));
}

static ts::Value indicator(const char* name) {
    return ts::Value::object().set("name", name).set("max", 100);
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts radar = echarts::init(*doc.getElementById("chart-radar"));
    echarts::ECharts gauge = echarts::init(*doc.getElementById("chart-gauge"));

    /* ---- Radar ---- */
    auto r_leg = ts::Value::object().set("bottom", 6).set("textStyle", color_obj("#111111"));
    auto r_sa = ts::Value::object().set(
        "areaStyle",
        ts::Value::object().set("color", ts::Array<>::of("#ffffff", "#f7f7f7"))
    );
    auto inds = ts::Array<>::of(
        indicator("Throughput"),
        indicator("Memory"),
        indicator("Startup"),
        indicator("Type Safety"),
        indicator("Bundle")
    );
    auto r_cfg = ts::Value::object()
        .set("radius", "62%")
        .set("axisName", color_obj("#111111"))
        .set("splitLine", line_wrap("#d4d4d4"))
        .set("splitArea", r_sa)
        .set("axisLine", line_wrap("#b5b5b5"))
        .set("indicator", inds);

    auto item0 = ts::Value::object()
        .set("name", "WASM Target")
        .set("value", ts::Array<>::of(92, 86, 88, 96, 84))
        .set("lineStyle", color_obj("#111111").set("width", 2))
        .set("itemStyle", color_obj("#111111"))
        .set("areaStyle", color_obj("rgba(17,17,17,0.12)"));

    auto item1 = ts::Value::object()
        .set("name", "Baseline")
        .set("value", ts::Array<>::of(70, 68, 74, 72, 66))
        .set("lineStyle", color_obj("#737373").set("width", 1.5).set("type", "dashed"))
        .set("itemStyle", color_obj("#737373"));

    auto radar_opt = echarts::EChartsCoreOption::create();
    radar_opt.animation(false);
    radar_opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("legend", r_leg)
        .set_at("radar", r_cfg)
        .set_at("series", ts::Array<>::of(ts::Value::object().set("type", "radar").set("data", ts::Array<>::of(item0, item1))));
    radar.setOption(radar_opt);

    /* ---- Gauge ---- */
    auto g_prog = ts::Value::object().set("show", true).set("width", 14).set("itemStyle", color_obj("#111111"));
    auto g_al = ts::Value::object().set(
        "lineStyle",
        ts::Value::object().set("width", 14).set("color", ts::Array<>::of(ts::Array<>::of(1.0, "#e5e5e5")))
    );
    auto g_det = color_obj("#111111").set("valueAnimation", false).set("formatter", "{value}%").set("fontSize", 22);

    auto gs0 = ts::Value::object()
        .set("type", "gauge")
        .set("min", 0)
        .set("max", 100)
        .set("progress", g_prog)
        .set("axisLine", g_al)
        .set("axisTick", line_wrap("#737373"))
        .set("splitLine", line_wrap("#111111"))
        .set("axisLabel", color_obj("#555555"))
        .set("pointer", ts::Value::object().set("itemStyle", color_obj("#111111")))
        .set("detail", g_det)
        .set("title", color_obj("#555555").set("fontSize", 12))
        .set("data", ts::Array<>::of(ts::Value::object().set("value", 84).set("name", "Score")));

    auto gauge_opt = echarts::EChartsCoreOption::create();
    gauge_opt.animation(false);
    gauge_opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("series", ts::Array<>::of(gs0));
    gauge.setOption(gauge_opt);

    doc.getElementById("meta")->textContent(
        "Radar: " + std::to_string(static_cast<int>(radar.getWidth())) + "x" +
        std::to_string(static_cast<int>(radar.getHeight())) + " \xC2\xB7 Gauge: " +
        std::to_string(static_cast<int>(gauge.getWidth())) + "x" +
        std::to_string(static_cast<int>(gauge.getHeight())) + " \xC2\xB7 Score 84%"
    );
}
