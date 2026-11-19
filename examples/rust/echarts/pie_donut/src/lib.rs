#![no_main]

use echarts::{ECUnitOptionMethods as _, EChartsCoreOption, EChartsTypeMethods as _, JsArray, JsValue};

fn obj(entries: &[(&str, JsValue)]) -> JsValue {
    let o = EChartsCoreOption::create();
    for (k, v) in entries {
        o.set_at(k, v.clone());
    }
    JsValue::from(&o)
}

fn arr(items: Vec<JsValue>) -> JsValue {
    JsArray::<JsValue>::from(items).into()
}

fn slice(name: &str, value: f64) -> JsValue {
    obj(&[("name", JsValue::from(name)), ("value", JsValue::from(value))])
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").unwrap();
    let chart = echarts::init(Some(&box_el), None, None);

    let opt = EChartsCoreOption::create();
    opt.set_animation(Some(false));
    opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    opt.set_at(
        "color",
        arr(["#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4"].into_iter().map(JsValue::from).collect()),
    );
    opt.set_at(
        "legend",
        obj(&[
            ("bottom", JsValue::from(8.0)),
            ("textStyle", obj(&[("color", JsValue::from("#111111"))])),
        ]),
    );
    opt.set_at("tooltip", obj(&[("trigger", JsValue::from("item"))]));
    opt.set_at(
        "series",
        arr(vec![obj(&[
            ("type", JsValue::from("pie")),
            ("name", JsValue::from("Subsystems")),
            ("radius", arr(vec![JsValue::from("44%"), JsValue::from("72%")])),
            ("center", arr(vec![JsValue::from("50%"), JsValue::from("46%")])),
            ("itemStyle", obj(&[("borderColor", JsValue::from("#ffffff")), ("borderWidth", JsValue::from(2.0))])),
            ("label", obj(&[("color", JsValue::from("#111111"))])),
            ("data", arr(vec![slice("Runtime", 34.0), slice("Bindings", 26.0), slice("Glue", 18.0), slice("Parser", 14.0), slice("Tests", 8.0)])),
        ])]),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").unwrap();
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 5 monochrome segments \u{00B7} Total 100",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
