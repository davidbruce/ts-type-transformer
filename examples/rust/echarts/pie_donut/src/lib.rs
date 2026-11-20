#![no_main]

use echarts::{EChartsCoreOption, JsArray, JsValue};

fn slice(name: &str, value: f64) -> JsValue {
    JsValue::object().with("name", name).with("value", value)
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").expect("missing #chart");
    let chart = echarts::init(Some(&box_el), None, None);

    let opt = EChartsCoreOption::default().with_animation(false);
    opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    opt.set_at(
        "color",
        JsArray::<JsValue>::from(["#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4"]),
    );
    opt.set_at(
        "legend",
        JsValue::object()
            .with("bottom", 8.0)
            .with("textStyle", JsValue::object().with("color", "#111111")),
    );
    opt.set_at("tooltip", JsValue::object().with("trigger", "item"));
    opt.set_at(
        "series",
        JsValue::from(JsArray::<JsValue>::from([JsValue::object()
            .with("type", "pie")
            .with("name", "Subsystems")
            .with("radius", JsArray::<JsValue>::from(["44%", "72%"]))
            .with("center", JsArray::<JsValue>::from(["50%", "46%"]))
            .with(
                "itemStyle",
                JsValue::object()
                    .with("borderColor", "#ffffff")
                    .with("borderWidth", 2.0),
            )
            .with("label", JsValue::object().with("color", "#111111"))
            .with(
                "data",
                JsArray::<JsValue>::from([
                    slice("Runtime", 34.0),
                    slice("Bindings", 26.0),
                    slice("Glue", 18.0),
                    slice("Parser", 14.0),
                    slice("Tests", 8.0),
                ]),
            )])),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 5 monochrome segments \u{00B7} Total 100",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
