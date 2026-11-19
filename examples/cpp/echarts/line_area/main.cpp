#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value color_obj(const char* hex) {
    ts::Value o = ts::Value::object();
    o.set("color", std::string(hex));
    return o;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement box = *doc.getElementById("chart");
    echarts::ECharts chart = echarts::init(box);

    ts::Value legend = ts::Value::object();
    legend.set("top", 8.0);
    legend.set("textStyle", color_obj("#111111"));

    ts::Value tooltip = ts::Value::object();
    tooltip.set("trigger", std::string("axis"));

    ts::Value grid = ts::Value::object();
    grid.set("left", 48.0);
    grid.set("right", 24.0);
    grid.set("top", 48.0);
    grid.set("bottom", 36.0);

    ts::Array<> weeks = ts::Array<>::create();
    for (const char* w : { "W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8" }) weeks.push(ts::Value::string(w));
    ts::Value x_line = ts::Value::object();
    x_line.set("lineStyle", color_obj("#111111"));
    ts::Value x_axis = ts::Value::object();
    x_axis.set("type", std::string("category"));
    x_axis.set("boundaryGap", false);
    x_axis.set("data", weeks);
    x_axis.set("axisLine", x_line);
    x_axis.set("axisLabel", color_obj("#555555"));

    ts::Value y_split = ts::Value::object();
    y_split.set("lineStyle", color_obj("#e5e5e5"));
    ts::Value y_axis = ts::Value::object();
    y_axis.set("type", std::string("value"));
    y_axis.set("min", 0.0);
    y_axis.set("max", 100.0);
    y_axis.set("splitLine", y_split);
    y_axis.set("axisLabel", color_obj("#555555"));

    ts::Array<> d0 = ts::Array<>::create();
    for (double v : { 28.0, 44.0, 39.0, 62.0, 58.0, 76.0, 81.0, 89.0 }) d0.push(ts::Value::number(v));
    ts::Value ls0 = color_obj("#111111");
    ls0.set("width", 2.0);
    ts::Value s0 = ts::Value::object();
    s0.set("type", std::string("line"));
    s0.set("name", std::string("Throughput"));
    s0.set("smooth", true);
    s0.set("data", d0);
    s0.set("lineStyle", ls0);
    s0.set("itemStyle", color_obj("#111111"));
    s0.set("areaStyle", color_obj("rgba(17,17,17,0.12)"));

    ts::Array<> d1 = ts::Array<>::create();
    for (double v : { 35.0, 38.0, 42.0, 46.0, 50.0, 54.0, 58.0, 62.0 }) d1.push(ts::Value::number(v));
    ts::Value ls1 = color_obj("#737373");
    ls1.set("width", 1.5);
    ls1.set("type", std::string("dashed"));
    ts::Value s1 = ts::Value::object();
    s1.set("type", std::string("line"));
    s1.set("name", std::string("Baseline"));
    s1.set("smooth", false);
    s1.set("data", d1);
    s1.set("lineStyle", ls1);
    s1.set("itemStyle", color_obj("#737373"));

    ts::Array<> series = ts::Array<>::create();
    series.push(s0);
    series.push(s1);

    echarts::EChartsCoreOption opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    opt.set_at("legend", legend);
    opt.set_at("tooltip", tooltip);
    opt.set_at("grid", grid);
    opt.set_at("xAxis", x_axis);
    opt.set_at("yAxis", y_axis);
    opt.set_at("series", ts::Value::of(series));
    chart.setOption(opt);

    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 2 series (Throughput + Baseline)"
    );
}
