#include "dom/dom.hpp"
#include "echarts/echarts.hpp"

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement box = *doc.getElementById("chart");
    echarts::ECharts chart = echarts::init(box);

    ts::Value title_style = ts::Value::object();
    title_style.set("color", std::string("#111111"));
    title_style.set("fontSize", 13.0);
    ts::Value title = ts::Value::object();
    title.set("text", std::string("Monthly Output"));
    title.set("left", 16.0);
    title.set("top", 12.0);
    title.set("textStyle", title_style);

    ts::Value grid = ts::Value::object();
    grid.set("left", 48.0);
    grid.set("right", 24.0);
    grid.set("top", 48.0);
    grid.set("bottom", 36.0);

    ts::Array<> cats = ts::Array<>::create();
    for (const char* m : { "Jan", "Feb", "Mar", "Apr", "May", "Jun" }) cats.push(ts::Value::string(m));
    ts::Value x_ls = ts::Value::object();
    x_ls.set("color", std::string("#111111"));
    ts::Value x_line = ts::Value::object();
    x_line.set("lineStyle", x_ls);
    ts::Value x_lbl = ts::Value::object();
    x_lbl.set("color", std::string("#555555"));
    ts::Value x_axis = ts::Value::object();
    x_axis.set("type", std::string("category"));
    x_axis.set("data", cats);
    x_axis.set("axisLine", x_line);
    x_axis.set("axisLabel", x_lbl);

    ts::Value y_ls = ts::Value::object();
    y_ls.set("color", std::string("#e5e5e5"));
    ts::Value y_split = ts::Value::object();
    y_split.set("lineStyle", y_ls);
    ts::Value y_lbl = ts::Value::object();
    y_lbl.set("color", std::string("#555555"));
    ts::Value y_axis = ts::Value::object();
    y_axis.set("type", std::string("value"));
    y_axis.set("min", 0.0);
    y_axis.set("max", 100.0);
    y_axis.set("splitLine", y_split);
    y_axis.set("axisLabel", y_lbl);

    ts::Array<> vals = ts::Array<>::create();
    for (double v : { 42.0, 68.0, 55.0, 84.0, 71.0, 93.0 }) vals.push(ts::Value::number(v));
    ts::Value item_style = ts::Value::object();
    item_style.set("color", std::string("#111111"));
    ts::Value s0 = ts::Value::object();
    s0.set("type", std::string("bar"));
    s0.set("name", std::string("Units"));
    s0.set("barWidth", std::string("46%"));
    s0.set("data", vals);
    s0.set("itemStyle", item_style);
    ts::Array<> series = ts::Array<>::create();
    series.push(s0);

    echarts::EChartsCoreOption opt = echarts::EChartsCoreOption::create();
    opt.animation(false);
    opt.set_at("backgroundColor", ts::Value::string("#ffffff"));
    opt.set_at("title", title);
    opt.set_at("grid", grid);
    opt.set_at("xAxis", x_axis);
    opt.set_at("yAxis", y_axis);
    opt.set_at("series", ts::Value::of(series));
    chart.setOption(opt);

    dom::HTMLElement meta = *doc.getElementById("meta");
    meta.textContent(
        "Canvas: " + std::to_string(static_cast<int>(chart.getWidth())) + "x" +
        std::to_string(static_cast<int>(chart.getHeight())) + " \xC2\xB7 6 bars \xC2\xB7 Monochrome #111111"
    );
}
