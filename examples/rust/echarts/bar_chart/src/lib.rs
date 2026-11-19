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

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").unwrap();
    let chart = echarts::init(Some(&box_el), None, None);

    let opt = EChartsCoreOption::create();
    opt.set_animation(Some(false));
    opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    opt.set_at(
        "title",
        obj(&[
            ("text", JsValue::from("Monthly Output")),
            ("left", JsValue::from(16.0)),
            ("top", JsValue::from(12.0)),
            ("textStyle", obj(&[("color", JsValue::from("#111111")), ("fontSize", JsValue::from(13.0))])),
        ]),
    );
    opt.set_at(
        "grid",
        obj(&[
            ("left", JsValue::from(48.0)),
            ("right", JsValue::from(24.0)),
            ("top", JsValue::from(48.0)),
            ("bottom", JsValue::from(36.0)),
        ]),
    );
    opt.set_at(
        "xAxis",
        obj(&[
            ("type", JsValue::from("category")),
            ("data", arr(vec![JsValue::from("Jan"), JsValue::from("Feb"), JsValue::from("Mar"), JsValue::from("Apr"), JsValue::from("May"), JsValue::from("Jun")])),
            ("axisLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#111111"))]))])),
            ("axisLabel", obj(&[("color", JsValue::from("#555555"))])),
        ]),
    );
    opt.set_at(
        "yAxis",
        obj(&[
            ("type", JsValue::from("value")),
            ("min", JsValue::from(0.0)),
            ("max", JsValue::from(100.0)),
            ("splitLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#e5e5e5"))]))])),
            ("axisLabel", obj(&[("color", JsValue::from("#555555"))])),
        ]),
    );
    opt.set_at(
        "series",
        arr(vec![obj(&[
            ("type", JsValue::from("bar")),
            ("name", JsValue::from("Units")),
            ("barWidth", JsValue::from("46%")),
            ("data", arr(vec![JsValue::from(42.0), JsValue::from(68.0), JsValue::from(55.0), JsValue::from(84.0), JsValue::from(71.0), JsValue::from(93.0)])),
            ("itemStyle", obj(&[("color", JsValue::from("#111111"))])),
        ])]),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").unwrap();
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 6 bars \u{00B7} Monochrome #111111",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
