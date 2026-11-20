#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value color_obj(const char* hex) {
    return ts::Value::object().set("color", hex);
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts chart = echarts::init(*doc.getElementById("chart"));

    auto legend = ts::Value::object().set("top", 8).set("textStyle", color_obj("#111111"));
    auto tooltip = ts::Value::object().set("trigger", "axis");
    auto grid = ts::Value::object().set("left", 48).set("right", 24).set("top", 48).set("bottom", 36);

    auto x_axis = ts::Value::object()
        .set("type", "category")
        .set("boundaryGap", false)
        .set("data", ts::Array<>::of("W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"))
        .set("axisLine", ts::Value::object().set("lineStyle", color_obj("#111111")))
        .set("axisLabel", color_obj("#555555"));

    auto y_axis = ts::Value::object()
        .set("type", "value")
        .set("min", 0)
        .set("max", 100)
        .set("splitLine", ts::Value::object().set("lineStyle", color_obj("#e5e5e5")))
        .set("axisLabel", color_obj("#555555"));

    auto s0 = ts::Value::object()
        .set("type", "line")
        .set("name", "Throughput")
        .set("smooth", true)
        .set("data", ts::Array<>::of(28, 44, 39, 62, 58, 76, 81, 89))
        .set("lineStyle", color_obj("#111111").set("width", 2))
        .set("itemStyle", color_obj("#111111"))
        .set("areaStyle", color_obj("rgba(17,17,17,0.12)"));

    auto s1 = ts::Value::object()
        .set("type", "line")
        .set("name", "Baseline")
        .set("smooth", false)
        .set("data", ts::Array<>::of(35, 38, 42, 46, 50, 54, 58, 62))
        .set("lineStyle", color_obj("#737373").set("width", 1.5).set("type", "dashed"))
        .set("itemStyle", color_obj("#737373"));

    auto opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("legend", legend)
        .set_at("tooltip", tooltip)
        .set_at("grid", grid)
        .set_at("xAxis", x_axis)
        .set_at("yAxis", y_axis)
        .set_at("series", ts::Array<>::of(s0, s1));
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 2 series (Throughput + Baseline)"
    );
}
