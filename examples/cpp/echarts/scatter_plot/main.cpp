#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value make_value_axis() {
    return ts::Value::object()
        .set("type", "value")
        .set("min", 0)
        .set("max", 100)
        .set("splitLine", ts::Value::object().set("lineStyle", ts::Value::object().set("color", "#e5e5e5")))
        .set("axisLabel", ts::Value::object().set("color", "#555555"));
}

static ts::Array<> make_pts(const double pts[][2], int n) {
    ts::Array<> arr = ts::Array<>::create();
    for (int i = 0; i < n; i++) arr.push(ts::Array<>::of(pts[i][0], pts[i][1]));
    return arr;
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    echarts::ECharts chart = echarts::init(*doc.getElementById("chart"));

    auto legend = ts::Value::object().set("top", 8).set("textStyle", ts::Value::object().set("color", "#111111"));
    auto grid = ts::Value::object().set("left", 48).set("right", 24).set("top", 48).set("bottom", 36);

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };

    auto s0 = ts::Value::object()
        .set("type", "scatter")
        .set("name", "Cluster A")
        .set("symbol", "circle")
        .set("symbolSize", 10)
        .set("itemStyle", ts::Value::object().set("color", "#111111"))
        .set("data", make_pts(ca_pts, 10));

    auto s1 = ts::Value::object()
        .set("type", "scatter")
        .set("name", "Cluster B")
        .set("symbol", "rect")
        .set("symbolSize", 10)
        .set("itemStyle", ts::Value::object().set("color", "#ffffff").set("borderColor", "#111111").set("borderWidth", 2))
        .set("data", make_pts(cb_pts, 10));

    auto opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"))
        .set_at("legend", legend)
        .set_at("grid", grid)
        .set_at("xAxis", make_value_axis())
        .set_at("yAxis", make_value_axis())
        .set_at("series", ts::Array<>::of(s0, s1));
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 20 data points across 2 clusters"
    );
}
