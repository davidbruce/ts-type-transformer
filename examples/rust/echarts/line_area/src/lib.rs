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
        "legend",
        obj(&[
            ("top", JsValue::from(8.0)),
            ("textStyle", obj(&[("color", JsValue::from("#111111"))])),
        ]),
    );
    opt.set_at("tooltip", obj(&[("trigger", JsValue::from("axis"))]));
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
            ("boundaryGap", JsValue::from(false)),
            ("data", arr(["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"].into_iter().map(JsValue::from).collect())),
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
        arr(vec![
            obj(&[
                ("type", JsValue::from("line")),
                ("name", JsValue::from("Throughput")),
                ("smooth", JsValue::from(true)),
                ("data", arr([28.0, 44.0, 39.0, 62.0, 58.0, 76.0, 81.0, 89.0].into_iter().map(JsValue::from).collect())),
                ("lineStyle", obj(&[("color", JsValue::from("#111111")), ("width", JsValue::from(2.0))])),
                ("itemStyle", obj(&[("color", JsValue::from("#111111"))])),
                ("areaStyle", obj(&[("color", JsValue::from("rgba(17,17,17,0.12)"))])),
            ]),
            obj(&[
                ("type", JsValue::from("line")),
                ("name", JsValue::from("Baseline")),
                ("smooth", JsValue::from(false)),
                ("data", arr([35.0, 38.0, 42.0, 46.0, 50.0, 54.0, 58.0, 62.0].into_iter().map(JsValue::from).collect())),
                ("lineStyle", obj(&[("color", JsValue::from("#737373")), ("width", JsValue::from(1.5)), ("type", JsValue::from("dashed"))])),
                ("itemStyle", obj(&[("color", JsValue::from("#737373"))])),
            ]),
        ]),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").unwrap();
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 2 series (Throughput + Baseline)",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
