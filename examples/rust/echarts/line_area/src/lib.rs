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
        "legend",
        JsValue::object()
            .with("top", 8.0)
            .with("textStyle", JsValue::object().with("color", "#111111")),
    );
    opt.set_at("tooltip", JsValue::object().with("trigger", "axis"));
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
            .with("boundaryGap", false)
            .with("data", JsArray::<JsValue>::from(["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"]))
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
        JsValue::from(JsArray::<JsValue>::from([
            JsValue::object()
                .with("type", "line")
                .with("name", "Throughput")
                .with("smooth", true)
                .with("data", JsArray::<JsValue>::from([28.0, 44.0, 39.0, 62.0, 58.0, 76.0, 81.0, 89.0]))
                .with("lineStyle", JsValue::object().with("color", "#111111").with("width", 2.0))
                .with("itemStyle", JsValue::object().with("color", "#111111"))
                .with("areaStyle", JsValue::object().with("color", "rgba(17,17,17,0.12)")),
            JsValue::object()
                .with("type", "line")
                .with("name", "Baseline")
                .with("smooth", false)
                .with("data", JsArray::<JsValue>::from([35.0, 38.0, 42.0, 46.0, 50.0, 54.0, 58.0, 62.0]))
                .with(
                    "lineStyle",
                    JsValue::object()
                        .with("color", "#737373")
                        .with("width", 1.5)
                        .with("type", "dashed"),
                )
                .with("itemStyle", JsValue::object().with("color", "#737373")),
        ])),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 2 series (Throughput + Baseline)",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
