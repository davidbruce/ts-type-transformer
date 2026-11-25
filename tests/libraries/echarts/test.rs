// ECharts from Rust, in a page: a bar chart on the page's #chart element,
// its options set and read back, its size, a coordinate conversion, events
// and disposal. Every check! must hold.
#![no_main]
use bindings::echarts::*;
use bindings::*;
use std::cell::Cell;
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

// A plain JavaScript object of values, made with the one dictionary type the
// bindings have for it (`EChartsCoreOption` has an index signature).
fn obj(entries: &[(&str, JsValue)]) -> JsValue {
    let o = EChartsCoreOption::create();
    for (k, v) in entries {
        o.set_at(k, v.clone());
    }
    JsValue::from(&o)
}
fn list(items: Vec<JsValue>) -> JsValue {
    JsValue(JsArray::<JsValue>::from(items).0.clone())
}
fn text(s: &str) -> JsValue {
    JsValue::from(s)
}
fn num(x: f64) -> JsValue {
    JsValue::from(x)
}
fn as_obj(v: &JsValue) -> EChartsCoreOption {
    EChartsCoreOption(v.0.clone())
}
// A JsValue that is an array, as a JsArray: no conversion is bound.
fn as_array(v: &JsValue) -> JsArray<JsValue> {
    let h = v.0.clone();
    let id = h.0;
    std::mem::forget(h);
    JsArray::<JsValue>::from_handle(id)
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // A chart on the page's 400 x 300 element.
    let chart = init_with(Some(&test_page::element("chart")), None, None);
    check!(!chart.is_disposed());
    check!(chart.get_width() == 400.0 && chart.get_height() == 300.0);
    check!(chart.get_dom() == test_page::element("chart"));
    check!(get_instance_by_dom(&test_page::element("chart")).is_some_and(|c| c == chart.clone()));

    // A bar chart: three categories and their values.
    let option = EChartsOption::create();
    option.set_animation(Some(false));
    option.set_x_axis(&obj(&[
        ("type", text("category")),
        ("data", list(vec![text("mon"), text("tue"), text("wed")])),
    ]));
    option.set_y_axis(&obj(&[("type", text("value"))]));
    let bars = BarSeriesOption::create();
    bars.set_name(OptionName::String("sales".to_string()));
    bars.set_data(Some(&JsArray::<JsValue>::from(vec![
        num(5.0),
        num(20.0),
        num(36.0),
    ])));
    option.set_series(&JsValue(bars.0.clone()));
    chart.set_option(&option);

    // Read back what was set: the series, with its data.
    let read = chart.get_option();
    let series = as_array(&read.at("series"));
    check!(series.len() == 1);
    let first = as_obj(&series.get(0));
    check!(first.at("type").as_string() == "bar" && first.at("name").as_string() == "sales");
    let data = as_array(&first.at("data"));
    check!(data.len() == 3 && data.get(2).as_f64() == 36.0);
    check!(as_array(&as_obj(&as_array(&read.at("xAxis")).get(0)).at("data")).len() == 3);
    check!(chart.get_option().at("animation").as_bool() == false);

    // Setting again merges: a fourth value is added to the series.
    let more = EChartsOption::create();
    more.set_series(&list(vec![obj(&[(
        "data",
        list(vec![num(5.0), num(20.0), num(36.0), num(7.0)]),
    )])]));
    chart.set_option(&more);
    let merged = as_array(&as_obj(&as_array(&chart.get_option().at("series")).get(0)).at("data"));
    check!(merged.len() == 4);

    // Pixels: the categories lie left to right inside the chart.
    let finder = ModelFinderObject::create();
    finder.set_x_axis_index(0.0);
    let x_mon = chart.convert_to_pixel(
        ModelFinder::ModelFinderObject(finder.clone()),
        ScaleDataValue::String("mon".to_string()),
    );
    let x_wed = chart.convert_to_pixel(
        ModelFinder::ModelFinderObject(finder.clone()),
        ScaleDataValue::String("wed".to_string()),
    );
    check!(x_mon > 0.0 && x_mon < x_wed && x_wed < 400.0);
    let back = chart.convert_from_pixel(ModelFinder::ModelFinderObject(finder), x_wed);
    check!(back == 2.0);

    // Events: a handler on a custom event, triggered with an argument.
    let heard = Rc::new(Cell::new(0.0));
    let count = Rc::new(Cell::new(0));
    let handler = AnyAnyArrayToVoidOrBooleanFn::new({
        let heard = heard.clone();
        let count = count.clone();
        move |args: JsArray| {
            count.set(args.len());
            heard.set(args.get(0).as_f64());
            None
        }
    });
    chart.on("custom", &handler);
    chart.trigger("custom", vec![num(42.0), num(1.0)]);
    check!(heard.get() == 42.0 && count.get() == 2);
    chart.off_with("custom", None);
    chart.trigger("custom", vec![num(7.0)]);
    check!(heard.get() == 42.0);

    // Resized, cleared, then disposed.
    chart.resize();
    check!(chart.get_width() == 400.0);
    chart.clear();
    check!(
        chart.get_option().at("series").is_null()
            || as_array(&chart.get_option().at("series")).is_empty()
    );
    chart.dispose();
    check!(chart.is_disposed());
    check!(get_instance_by_dom(&test_page::element("chart")).is_none());
}
