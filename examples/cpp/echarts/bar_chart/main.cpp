#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts chart = echarts::init(*doc.getElementById("chart"));

    auto title = ts::Value::object()
        .set("text", "Monthly Output")
        .set("left", 16)
        .set("top", 12)
        .set("textStyle", ts::Value::object().set("color", "#111111").set("fontSize", 13));

    auto grid = ts::Value::object().set("left", 48).set("right", 24).set("top", 48).set("bottom", 36);

    auto x_axis = ts::Value::object()
        .set("type", "category")
        .set("data", ts::Array<>::of("Jan", "Feb", "Mar", "Apr", "May", "Jun"))
        .set("axisLine", ts::Value::object().set("lineStyle", ts::Value::object().set("color", "#111111")))
        .set("axisLabel", ts::Value::object().set("color", "#555555"));

    auto y_axis = ts::Value::object()
        .set("type", "value")
        .set("min", 0)
        .set("max", 100)
        .set("splitLine", ts::Value::object().set("lineStyle", ts::Value::object().set("color", "#e5e5e5")))
        .set("axisLabel", ts::Value::object().set("color", "#555555"));

    auto series = ts::Array<>::of(
        ts::Value::object()
            .set("type", "bar")
            .set("name", "Units")
            .set("barWidth", "46%")
            .set("data", ts::Array<>::of(42, 68, 55, 84, 71, 93))
            .set("itemStyle", ts::Value::object().set("color", "#111111"))
    );

    auto opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("title", title)
        .set_at("grid", grid)
        .set_at("xAxis", x_axis)
        .set_at("yAxis", y_axis)
        .set_at("series", series);
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111"
    );
}
