#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value slice(const char* name, double val) {
    ts::Value d = ts::Value::object();
    d.set("name", std::string(name));
    d.set("value", val);
    return d;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement box = *doc.getElementById("chart");
    echarts::ECharts chart = echarts::init(box);

    ts::Array<> palette = ts::Array<>::create();
    for (const char* hex : { "#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4" }) {
        palette.push(ts::Value::string(hex));
    }

    ts::Value leg_ts = ts::Value::object();
    leg_ts.set("color", std::string("#111111"));
    ts::Value legend = ts::Value::object();
    legend.set("bottom", 8.0);
    legend.set("textStyle", leg_ts);

    ts::Value tooltip = ts::Value::object();
    tooltip.set("trigger", std::string("item"));

    ts::Array<> radius = ts::Array<>::create();
    radius.push(ts::Value::string("44%"));
    radius.push(ts::Value::string("72%"));

    ts::Array<> center = ts::Array<>::create();
    center.push(ts::Value::string("50%"));
    center.push(ts::Value::string("46%"));

    ts::Value item_style = ts::Value::object();
    item_style.set("borderColor", std::string("#ffffff"));
    item_style.set("borderWidth", 2.0);

    ts::Value lbl = ts::Value::object();
    lbl.set("color", std::string("#111111"));

    ts::Array<> data = ts::Array<>::create();
    data.push(slice("Runtime", 34));
    data.push(slice("Bindings", 26));
    data.push(slice("Glue", 18));
    data.push(slice("Parser", 14));
    data.push(slice("Tests", 8));

    ts::Value s0 = ts::Value::object();
    s0.set("type", std::string("pie"));
    s0.set("name", std::string("Subsystems"));
    s0.set("radius", radius);
    s0.set("center", center);
    s0.set("itemStyle", item_style);
    s0.set("label", lbl);
    s0.set("data", data);

    ts::Array<> series = ts::Array<>::create();
    series.push(s0);

    echarts::EChartsCoreOption opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    opt.set_at("color", ts::Value::of(palette));
    opt.set_at("legend", legend);
    opt.set_at("tooltip", tooltip);
    opt.set_at("series", ts::Value::of(series));
    chart.setOption(opt);

    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100"
    );
}
