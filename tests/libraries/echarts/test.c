// ECharts from C, in a page: a bar chart on the page's element, options set
// and read back, pixel conversion, actions and events, resize and dispose.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps) { return (a > b ? a - b : b - a) < eps; }

// The nth value of a JavaScript array, as a handle.
static JsValue at(JsValue array, uint32_t i) { return JsArray_get((JsArray){ array.h }, i); }
static bool str_is(JsValue v, const char* expected) { return JsString_equals(js_to_string(v), expected); }

// A property of an object handle.
static JsValue RowGet(JsValue obj, const char* key) { return Dictionary_get_at((Dictionary){ obj.h }, key); }
static JsValue dict_value(Dictionary d) { return (JsValue){ d.h }; }

static int highlights, finished;
static bool highlight_event_ok;
static int32_t on_highlight(void* ctx, JsArray args) {
    highlights++;
    // The rest parameter arrives gathered: item 0 is the event.
    if (JsArray_length(args) >= 1) {
        JsValue ev = JsArray_get(args, 0);
        JsValue type = Dictionary_get_at((Dictionary){ ev.h }, "type");
        if (JsString_equals(js_to_string(type), "highlight")) highlight_event_ok = true;
    }
    return 0;
}
static int32_t on_finished(void* ctx, JsArray args) {
    finished++;
    return 0;
}

static ECharts the_chart;
static HTMLElement the_host;
__attribute__((export_name("finish"))) void finish(void) {
    // `finished` comes after the chart has drawn, a frame later.
    CHECK(finished >= 1);
    // Disposed, it is gone: the page's element has no chart.
    EChartsType_dispose(the_chart);
    CHECK(EChartsType_isDisposed(the_chart));
    CHECK(getInstanceByDom(the_host).h == 0);
}

__attribute__((export_name("run"))) void run(void) {
    HTMLElement host = element("chart");
    EChartsInitOpts init_opts = EChartsInitOpts_create();
    EChartsInitOpts_set_renderer(init_opts, RendererType_canvas);
    ECharts chart = init(host, (const char*)0, init_opts);
    CHECK(chart.h != 0);
    CHECK(!EChartsType_isDisposed(chart));
    CHECK(EChartsType_getWidth(chart) == 400 && EChartsType_getHeight(chart) == 300);
    CHECK(js_same(EChartsType_getDom(chart), host));
    CHECK(js_same(getInstanceByDom(host), chart));
    CHECK(JsString_len(EChartsType_getId(chart)) > 0);

    // The listener for `finished`, which comes after a render.
    Eventful_on(chart, "finished", any_any_array_to_void_or_boolean_fn_from(on_finished, 0), (JsValue){ 0 });
    Eventful_on(chart, "highlight", any_any_array_to_void_or_boolean_fn_from(on_highlight, 0), (JsValue){ 0 });

    // The option: a category axis, a value axis, one bar series.
    Dictionary x_axis = Dictionary_create();
    Dictionary_set_at(x_axis, "type", js_string("category"));
    JsArray categories = JsArray_new();
    JsArray_push(categories, js_string("Mon"));
    JsArray_push(categories, js_string("Tue"));
    JsArray_push(categories, js_string("Wed"));
    Dictionary_set_at(x_axis, "data", (JsValue){ categories.h });
    Dictionary y_axis = Dictionary_create();
    Dictionary_set_at(y_axis, "type", js_string("value"));
    Dictionary_set_at(y_axis, "min", js_number(0));
    Dictionary_set_at(y_axis, "max", js_number(40));
    Dictionary grid = Dictionary_create();
    Dictionary_set_at(grid, "left", js_number(50));
    Dictionary_set_at(grid, "right", js_number(50));
    Dictionary_set_at(grid, "top", js_number(50));
    Dictionary_set_at(grid, "bottom", js_number(50));
    BarSeriesOption series0 = BarSeriesOption_create();
    BarSeriesOption_set_name(series0, PositionSizeOption_of_string(JsString_from("sales")));
    JsArray values = JsArray_new();
    JsArray_push(values, js_number(5));
    JsArray_push(values, js_number(20));
    JsArray_push(values, js_number(36));
    BarSeriesOption_set_data(series0, values);
    JsArray series = JsArray_new();
    JsArray_push(series, (JsValue){ series0.h });

    EChartsCoreOption option = EChartsCoreOption_create();
    EChartsCoreOption_set_at(option, "xAxis", dict_value(x_axis));
    EChartsCoreOption_set_at(option, "yAxis", dict_value(y_axis));
    EChartsCoreOption_set_at(option, "grid", dict_value(grid));
    EChartsCoreOption_set_at(option, "series", (JsValue){ series.h });
    EChartsCoreOption_set_animation(option, js_some_bool(false));
    EChartsType_setOption(chart, option);

    // Read back what it settled on: the option, normalised into arrays.
    EChartsCoreOption got = EChartsType_getOption(chart);
    JsValue got_series = EChartsCoreOption_get_at(got, "series");
    CHECK(JsArray_length((JsArray){ got_series.h }) == 1);
    JsValue first = at(got_series, 0);
    CHECK(str_is(RowGet(first, "type"), "bar"));
    CHECK(str_is(RowGet(first, "name"), "sales"));
    JsValue data = RowGet(first, "data");
    CHECK(JsArray_length((JsArray){ data.h }) == 3);
    CHECK(js_to_number(at(data, 0)) == 5 && js_to_number(at(data, 1)) == 20 && js_to_number(at(data, 2)) == 36);
    JsValue got_x = EChartsCoreOption_get_at(got, "xAxis");
    CHECK(JsArray_length((JsArray){ got_x.h }) == 1);
    CHECK(str_is(RowGet(at(got_x, 0), "type"), "category"));
    JsValue got_x_data = RowGet(at(got_x, 0), "data");
    CHECK(JsArray_length((JsArray){ got_x_data.h }) == 3 && str_is(at(got_x_data, 1), "Tue"));
    CHECK(js_to_number(RowGet(at(EChartsCoreOption_get_at(got, "yAxis"), 0), "max")) == 40);

    // A later setOption merges: the series gets a new value and keeps its name.
    JsArray more = JsArray_new();
    JsArray_push(more, js_number(10));
    JsArray_push(more, js_number(20));
    JsArray_push(more, js_number(30));
    JsArray_push(more, js_number(40));
    Dictionary update = Dictionary_create();
    Dictionary_set_at(update, "data", (JsValue){ more.h });
    JsArray update_series = JsArray_new();
    JsArray_push(update_series, dict_value(update));
    EChartsCoreOption patch = EChartsCoreOption_create();
    EChartsCoreOption_set_at(patch, "series", (JsValue){ update_series.h });
    EChartsType_setOption(chart, patch);
    JsValue merged = at(EChartsCoreOption_get_at(EChartsType_getOption(chart), "series"), 0);
    CHECK(str_is(RowGet(merged, "name"), "sales") && str_is(RowGet(merged, "type"), "bar"));
    CHECK(JsArray_length((JsArray){ RowGet(merged, "data").h }) == 4);
    // With notMerge, it replaces.
    EChartsType_setOption(chart, option, js_some_bool(true), js_none_bool());
    CHECK(JsArray_length((JsArray){ RowGet(at(EChartsCoreOption_get_at(EChartsType_getOption(chart), "series"), 0), "data").h }) == 3);

    // Pixels: the grid is 50 in from each side of 400 x 300.
    ModelFinderObject finder_obj = ModelFinderObject_create();
    ModelFinderObject_set_gridIndex(finder_obj, js_number(0));
    ModelFinder finder = { .tag = ModelFinder_ModelFinderObject, .as.ModelFinderObject = finder_obj };
    JsArray_ScaleDataValue where = JsArray_ScaleDataValue_new();
    JsArray_ScaleDataValue_push(where, ScaleDataValue_of_string(JsString_from("Tue")));
    JsArray_ScaleDataValue_push(where, ScaleDataValue_of_number(20));
    JsArray_double px = EChartsType_convertToPixel_with_ScaleDataValue_array(chart, finder, where);
    CHECK(JsArray_double_length(px) == 2);
    double x = JsArray_double_get(px, 0), y = JsArray_double_get(px, 1);
    // Tue is the middle of three bands across 300 px: x = 200. 20 of 40 is half of 200 px: y = 150.
    CHECK(near(x, 200, 1.5));
    CHECK(near(y, 150, 1.5));
    JsArray_double back = EChartsType_convertFromPixel_with_number_array(chart, finder, px);
    CHECK(JsArray_double_length(back) == 2 && near(JsArray_double_get(back, 1), 20, 0.1));
    JsArray_double inside = JsArray_double_new();
    JsArray_double_push(inside, 200);
    JsArray_double_push(inside, 150);
    CHECK(EChartsType_containPixel(chart, finder, inside));
    JsArray_double outside = JsArray_double_new();
    JsArray_double_push(outside, 10);
    JsArray_double_push(outside, 10);
    CHECK(!EChartsType_containPixel(chart, finder, outside));

    // An action: highlight the second bar. Its event reaches the listener.
    Dictionary action = Dictionary_create();
    Dictionary_set_at(action, "type", js_string("highlight"));
    Dictionary_set_at(action, "seriesIndex", js_number(0));
    Dictionary_set_at(action, "dataIndex", js_number(1));
    EChartsType_dispatchAction(chart, (Payload){ action.h }, js_none_bool());
    CHECK(highlights >= 1);
    CHECK(highlight_event_ok);

    // The chart as an image.
    JsString url = EChartsType_getDataURL(chart, (EChartsType_getDataURL_opts){ 0 });
    char head[16];
    JsString_copy(url, head, sizeof head);
    CHECK(head[0] == 'd' && head[1] == 'a' && head[2] == 't' && head[3] == 'a' && head[4] == ':');
    CHECK(JsString_len(url) > 200);
    // The canvas it drew on is as large as the chart (times the pixel ratio).
    HTMLCanvasElement drawn = EChartsType_getRenderedCanvas(chart, (JsValue){ 0 });
    CHECK(drawn.h != 0);

    // Resize to a smaller size, then back to what the container says ("auto":
    // a size given sticks until it is given as auto).
    ResizeOpts small = ResizeOpts_create();
    ResizeOpts_set_width(small, PositionSizeOption_of_number(300));
    ResizeOpts_set_height(small, PositionSizeOption_of_number(200));
    EChartsType_resize(chart, small);
    CHECK(EChartsType_getWidth(chart) == 300 && EChartsType_getHeight(chart) == 200);
    ResizeOpts automatic = ResizeOpts_create();
    ResizeOpts_set_width(automatic, PositionSizeOption_of_string(JsString_from("auto")));
    ResizeOpts_set_height(automatic, PositionSizeOption_of_string(JsString_from("auto")));
    EChartsType_resize(chart, automatic);
    CHECK(EChartsType_getWidth(chart) == 400 && EChartsType_getHeight(chart) == 300);

    // Clearing empties the option; disposing ends the chart.
    EChartsType_clear(chart);
    JsValue left = EChartsCoreOption_get_at(EChartsType_getOption(chart), "series");
    CHECK(left.h == 0 || JsArray_length((JsArray){ left.h }) == 0);
    EChartsType_setOption(chart, option);
    CHECK(JsArray_length((JsArray){ EChartsCoreOption_get_at(EChartsType_getOption(chart), "series").h }) == 1);
    // (The dispose is left to `finish`: `finished` is still to come.)
    the_chart = chart;
    the_host = host;
}
