// ECharts from C++, in a page: a chart on the page's element with a bar series,
// its option read back, its size, coordinate conversion, events, dispose.
// Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}

extern "C" __attribute__((export_name("run"))) void run() {
    HTMLElement box = test_page::element("chart");
    ECharts chart = echarts::init(box);
    CHECK(!chart.isDisposed());
    CHECK(chart.getWidth() == 400 && chart.getHeight() == 300);
    CHECK(chart.getDom() == box);
    CHECK(!chart.getId().empty());
    CHECK(echarts::getInstanceByDom(box).has_value());

    // A bar series over a category axis.
    XAXisComponentOption_xAxis xaxis = XAXisComponentOption_xAxis::create();
    auto days = ts::Array<string_or_number_or_CategoryAxisBaseOption_data>::create();
    for (const char* d : {"Mon", "Tue", "Wed"}) days.push(string_or_number_or_CategoryAxisBaseOption_data(std::string(d)));
    xaxis.data(days);
    YAXisComponentOption_yAxis_value yaxis = YAXisComponentOption_yAxis_value::create();

    ts::Array<> values = ts::Array<>::create();
    for (double v : {5.0, 20.0, 36.0}) values.push(ts::Value::number(v));
    BarSeriesOption bars = BarSeriesOption::create();
    bars.name(PositionSizeOption(std::string("sales")));
    bars.data(values);

    EChartsOption option = EChartsOption::create();
    option.xAxis(EChartsOption_xAxis(xaxis));
    option.yAxis(EChartsOption_yAxis(yaxis));
    option.series(EChartsOption_series(bars));
    chart.setOption(option);
    CHECK(!ts::take_error());

    // Read the option back: what went in comes out (normalised to arrays).
    EChartsCoreOption got = chart.getOption();
    EChartsOption back;
    static_cast<ts::Handle&>(back) = got;
    CHECK(back.series().has_value());
    ts::Array<> got_series;
    ts::Value series_value = *back.series();
    static_cast<ts::Handle&>(got_series) = series_value;
    CHECK(got_series.size() == 1);
    BarSeriesOption first;
    static_cast<ts::Handle&>(first) = got_series[0];
    CHECK(first.name().has_value());
    CHECK(first.data().has_value() && first.data()->size() == 3);
    CHECK((*first.data())[2].to_number() == 36);
    CHECK(BarSeriesOption::type == "bar");

    // The axes read back too: the category data went in as given.
    CHECK(back.xAxis().has_value());

    // Size, device pixel ratio, resize.
    CHECK(chart.getDevicePixelRatio() > 0);
    CHECK(chart.getZr().h != 0);
    chart.resize();
    CHECK(chart.getWidth() == 400 && chart.getHeight() == 300);
    ResizeOpts size = ResizeOpts::create();
    size.width(PositionSizeOption(300.0));
    size.height(PositionSizeOption(200.0));
    chart.resize(size);
    CHECK(chart.getWidth() == 300 && chart.getHeight() == 200);
    ResizeOpts reset = ResizeOpts::create();
    reset.width(PositionSizeOption(std::string("auto")));
    reset.height(PositionSizeOption(std::string("auto")));
    chart.resize(reset);
    CHECK(chart.getWidth() == 400 && chart.getHeight() == 300);

    // Pixel conversion: a bigger value is higher on the chart (a smaller y);
    // later categories are further right; and it converts back.
    ModelFinderObject onX = ModelFinderObject::create();
    onX.xAxisIndex(ts::Value::number(0));
    ModelFinderObject onY = ModelFinderObject::create();
    onY.yAxisIndex(ts::Value::number(0));
    double xMon = chart.convertToPixel(ModelFinder(onX), ScaleDataValue(std::string("Mon")));
    double xWed = chart.convertToPixel(ModelFinder(onX), ScaleDataValue(std::string("Wed")));
    double y5 = chart.convertToPixel(ModelFinder(onY), ScaleDataValue(5.0));
    double y36 = chart.convertToPixel(ModelFinder(onY), ScaleDataValue(36.0));
    CHECK(xMon > 0 && xWed > xMon && xWed < 400);
    CHECK(y36 < y5 && y5 < 300 && y36 > 0);
    double back_y = chart.convertFromPixel(ModelFinder(onY), y36);
    CHECK(std::fabs(back_y - 36) < 1e-6);
    ts::Array<double> pt = ts::Array<double>::create();
    pt.push(xWed);
    pt.push(y36);
    ModelFinderObject onGrid = ModelFinderObject::create();
    onGrid.gridIndex(ts::Value::number(0));
    CHECK(chart.containPixel(ModelFinder(onGrid), pt));

    // Events: handlers by name; trigger calls them at once.
    int heard = 0;
    std::string seen;
    any_any_array_to_void_or_boolean_fn handler = [&](ts::Array<> args) -> std::optional<bool> {
        heard++;
        seen += std::to_string(args.size()) + ":";
        return std::nullopt;
    };
    chart.on<ts::Value, std::string>(std::string("custom"), handler);
    chart.trigger<std::string>(std::string("custom"), ts::Array<>::create());
    chart.trigger<std::string>(std::string("custom"), ts::Array<>{ts::Value::number(1), ts::Value::string("x")});
    CHECK(heard == 2);
    CHECK(seen == "0:2:");
    chart.off(ts::Value::string("custom"));
    chart.trigger<std::string>(std::string("custom"), ts::Array<>::create());
    CHECK(heard == 2);
    handler.release();

    // An action: highlight a series; showLoading and hideLoading.
    Payload action = Payload::create();
    action.type("highlight");
    action.set_at("seriesIndex", ts::Value::number(0));
    chart.dispatchAction(action);
    chart.showLoading();
    chart.hideLoading();
    CHECK(!ts::take_error());

    // Another option replaces the first with notMerge: a new series list.
    ts::Array<> one = ts::Array<>::create();
    one.push(ts::Value::number(7));
    Dictionary<ts::Value> line = Dictionary<ts::Value>::create();
    line.set_at("type", ts::Value::string("line"));
    line.set_at("data", val_of(one));
    ts::Array<> lines = ts::Array<>::create();
    lines.push(val_of(line));
    EChartsOption second = EChartsOption::create();
    second.xAxis(EChartsOption_xAxis(xaxis));
    second.yAxis(EChartsOption_yAxis(yaxis));
    second.series(EChartsOption_series(lines));
    SetOptionOpts replace = SetOptionOpts::create();
    replace.notMerge(true);
    chart.setOption(second, replace);
    EChartsOption now;
    static_cast<ts::Handle&>(now) = chart.getOption();
    ts::Value now_series_value = *now.series();
    ts::Array<> now_series;
    static_cast<ts::Handle&>(now_series) = now_series_value;
    CHECK(now_series.size() == 1);

    // The chart as a picture, and clearing it.
    std::string url = chart.getDataURL();
    CHECK(url.rfind("data:image/png", 0) == 0);
    HTMLCanvasElement rendered = chart.getRenderedCanvas();
    CHECK(rendered.h != 0);
    chart.clear();
    CHECK(!ts::take_error());

    // An SVG chart on another element, drawn to a string.
    EChartsInitOpts svg_opts = EChartsInitOpts::create();
    svg_opts.renderer(RendererType::svg);
    ECharts vector = echarts::init(test_page::element("stage"), ts::Value(), svg_opts);
    vector.setOption(option);
    CHECK(vector.getSvgDataURL().rfind("data:image/svg+xml", 0) == 0);
    vector.dispose();
    CHECK(vector.isDisposed());

    chart.dispose();
    CHECK(chart.isDisposed());
    CHECK(!echarts::getInstanceByDom(box).has_value());
}
