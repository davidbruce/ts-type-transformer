#![no_main]

use echarts::{EChartsCoreOption, JsArray, JsValue};

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").expect("missing #chart");
    let chart = echarts::init(Some(&box_el), None, None);

    let opt = EChartsCoreOption::default().with_animation(false);
    opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    opt.set_at(
        "title",
        JsValue::object()
            .with("text", "Monthly Output")
            .with("left", 16.0)
            .with("top", 12.0)
            .with(
                "textStyle",
                JsValue::object().with("color", "#111111").with("fontSize", 13.0),
            ),
    );
    opt.set_at(
        "grid",
        JsValue::object()
            .with("left", 48.0)
            .with("right", 24.0)
            .with("top", 48.0)
            .with("bottom", 36.0),
    );
    opt.set_at(
        "xAxis",
        JsValue::object()
            .with("type", "category")
            .with("data", JsArray::<JsValue>::from(["Jan", "Feb", "Mar", "Apr", "May", "Jun"]))
            .with("axisLine", JsValue::object().with("lineStyle", JsValue::object().with("color", "#111111")))
            .with("axisLabel", JsValue::object().with("color", "#555555")),
    );
    opt.set_at(
        "yAxis",
        JsValue::object()
            .with("type", "value")
            .with("min", 0.0)
            .with("max", 100.0)
            .with("splitLine", JsValue::object().with("lineStyle", JsValue::object().with("color", "#e5e5e5")))
            .with("axisLabel", JsValue::object().with("color", "#555555")),
    );
    opt.set_at(
        "series",
        JsValue::from(JsArray::<JsValue>::from([JsValue::object()
            .with("type", "bar")
            .with("name", "Units")
            .with("barWidth", "46%")
            .with("data", JsArray::<JsValue>::from([42.0, 68.0, 55.0, 84.0, 71.0, 93.0]))
            .with("itemStyle", JsValue::object().with("color", "#111111"))])),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 6 bars \u{00B7} Monochrome #111111",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
