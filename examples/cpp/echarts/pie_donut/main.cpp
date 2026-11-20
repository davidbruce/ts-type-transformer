#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value slice(const char* name, double val) {
    return ts::Value::object().set("name", name).set("value", val);
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts chart = echarts::init(*doc.getElementById("chart"));

    auto palette = ts::Array<>::of("#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4");
    auto legend = ts::Value::object().set("bottom", 8).set("textStyle", ts::Value::object().set("color", "#111111"));
    auto tooltip = ts::Value::object().set("trigger", "item");

    auto data = ts::Array<>::of(
        slice("Runtime", 34),
        slice("Bindings", 26),
        slice("Glue", 18),
        slice("Parser", 14),
        slice("Tests", 8)
    );

    auto series = ts::Array<>::of(
        ts::Value::object()
            .set("type", "pie")
            .set("name", "Subsystems")
            .set("radius", ts::Array<>::of("44%", "72%"))
            .set("center", ts::Array<>::of("50%", "46%"))
            .set("itemStyle", ts::Value::object().set("borderColor", "#ffffff").set("borderWidth", 2))
            .set("label", ts::Value::object().set("color", "#111111"))
            .set("data", data)
    );

    auto opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("color", palette)
        .set_at("legend", legend)
        .set_at("tooltip", tooltip)
        .set_at("series", series);
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 5 monochrome segments \xC2\xB7 Total 100"
    );
}
