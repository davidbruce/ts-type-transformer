#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

using namespace echarts;

template <class Axis>
static Axis make_value_axis() {
    return Axis({
        .axisLabel = AxisLabelOption({.color = "#555555"}),
        .splitLine = SplitLineOption({.lineStyle = LineStyleOption({.color = "#e5e5e5"})}),
        .min = 0,
        .max = 100,
    });
}

static ts::Array<ts::Array<double>> make_pts(const double pts[][2], int n) {
    auto arr = ts::Array<ts::Array<double>>::create();
    for (int i = 0; i < n; i++) arr.push(ts::Array<double>::of(pts[i][0], pts[i][1]));
    return arr;
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    ECharts chart = echarts::init(*doc.getElementById("chart"));

    const double ca_pts[10][2] = { {14,22},{21,31},{28,27},{33,42},{40,48},{46,53},{54,61},{62,59},{71,74},{82,85} };
    const double cb_pts[10][2] = { {18,64},{25,72},{31,68},{39,79},{47,65},{55,81},{64,77},{72,88},{79,82},{88,92} };

    auto s0 = ScatterSeriesOption({
        .data = make_pts(ca_pts, 10),
        .symbol = "circle",
        .symbolSize = 10,
        .name = "Cluster A",
        .itemStyle = ItemStyleOption({.color = "#111111"}),
    });

    auto s1 = ScatterSeriesOption({
        .data = make_pts(cb_pts, 10),
        .symbol = "rect",
        .symbolSize = 10,
        .name = "Cluster B",
        .itemStyle = ItemStyleOption({.borderColor = "#111111", .borderWidth = 2, .color = "#ffffff"}),
    });

    EChartsOption opt({
        .backgroundColor = "#ffffff",
        .animation = false,
        .grid = GridComponentOption({.top = 48, .right = 24, .bottom = 36, .left = 48}),
        .xAxis = make_value_axis<XAXisComponentOption_xAxis_value>(),
        .yAxis = make_value_axis<YAXisComponentOption_yAxis_value>(),
        .legend = PlainLegendComponentOption({.textStyle = LabelOption({.color = "#111111"}), .top = 8}),
        .series = ts::Array<ScatterSeriesOption>::of(s0, s1),
    });
    chart.setOption(opt);

    doc.getElementById("meta")->textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 20 data points across 2 clusters"
    );
}
