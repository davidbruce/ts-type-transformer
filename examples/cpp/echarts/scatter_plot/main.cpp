#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

static ts::Value make_value_axis() {
    ts::Value ls = ts::Value::object();
    ls.set("color", std::string("#e5e5e5"));
    ts::Value split = ts::Value::object();
    split.set("lineStyle", ls);
    ts::Value lbl = ts::Value::object();
    lbl.set("color", std::string("#555555"));
    ts::Value ax = ts::Value::object();
    ax.set("type", std::string("value"));
    ax.set("min", 0.0);
    ax.set("max", 100.0);
    ax.set("splitLine", split);
    ax.set("axisLabel", lbl);
    return ax;
}

static ts::Array<> make_pts(const double pts[][2], int n) {
    ts::Array<> arr = ts::Array<>::create();
    for (int i = 0; i < n; i++) {
        ts::Array<> pair = ts::Array<>::create();
        pair.push(ts::Value::number(pts[i][0]));
        pair.push(ts::Value::number(pts[i][1]));
        arr.push(pair);
    }
    return arr;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement box = *doc.getElementById("chart");
    echarts::ECharts chart = echarts::init(box);

    ts::Value leg_ts = ts::Value::object();
    leg_ts.set("color", std::string("#111111"));
    ts::Value legend = ts::Value::object();
    legend.set("top", 8.0);
    legend.set("textStyle", leg_ts);

    ts::Value grid = ts::Value::object();
    grid.set("left", 48.0);
    grid.set("right", 24.0);
    grid.set("top", 48.0);
    grid.set("bottom", 36.0);

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };

    ts::Value is_a = ts::Value::object();
    is_a.set("color", std::string("#111111"));
    ts::Value s0 = ts::Value::object();
    s0.set("type", std::string("scatter"));
    s0.set("name", std::string("Cluster A"));
    s0.set("symbol", std::string("circle"));
    s0.set("symbolSize", 10.0);
    s0.set("itemStyle", is_a);
    s0.set("data", make_pts(ca_pts, 10));

    ts::Value is_b = ts::Value::object();
    is_b.set("color", std::string("#ffffff"));
    is_b.set("borderColor", std::string("#111111"));
    is_b.set("borderWidth", 2.0);
    ts::Value s1 = ts::Value::object();
    s1.set("type", std::string("scatter"));
    s1.set("name", std::string("Cluster B"));
    s1.set("symbol", std::string("rect"));
    s1.set("symbolSize", 10.0);
    s1.set("itemStyle", is_b);
    s1.set("data", make_pts(cb_pts, 10));

    ts::Array<> series = ts::Array<>::create();
    series.push(s0);
    series.push(s1);

    echarts::EChartsCoreOption opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    opt.set_at("legend", legend);
    opt.set_at("grid", grid);
    opt.set_at("xAxis", make_value_axis());
    opt.set_at("yAxis", make_value_axis());
    opt.set_at("series", ts::Value::of(series));
    chart.setOption(opt);

    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 20 data points across 2 clusters"
    );
}
