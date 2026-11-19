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

fn value_axis() -> JsValue {
    obj(&[
        ("type", JsValue::from("value")),
        ("min", JsValue::from(0.0)),
        ("max", JsValue::from(100.0)),
        ("splitLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#e5e5e5"))]))])),
        ("axisLabel", obj(&[("color", JsValue::from("#555555"))])),
    ])
}

fn pts(coords: &[(f64, f64)]) -> JsValue {
    arr(coords
        .iter()
        .map(|&(x, y)| arr(vec![JsValue::from(x), JsValue::from(y)]))
        .collect())
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
    opt.set_at(
        "grid",
        obj(&[
            ("left", JsValue::from(48.0)),
            ("right", JsValue::from(24.0)),
            ("top", JsValue::from(48.0)),
            ("bottom", JsValue::from(36.0)),
        ]),
    );
    opt.set_at("xAxis", value_axis());
    opt.set_at("yAxis", value_axis());
    opt.set_at(
        "series",
        arr(vec![
            obj(&[
                ("type", JsValue::from("scatter")),
                ("name", JsValue::from("Cluster A")),
                ("symbol", JsValue::from("circle")),
                ("symbolSize", JsValue::from(10.0)),
                ("itemStyle", obj(&[("color", JsValue::from("#111111"))])),
                ("data", pts(&[(14.0, 22.0), (21.0, 31.0), (28.0, 27.0), (33.0, 42.0), (40.0, 48.0), (46.0, 53.0), (54.0, 61.0), (62.0, 59.0), (71.0, 74.0), (82.0, 85.0)])),
            ]),
            obj(&[
                ("type", JsValue::from("scatter")),
                ("name", JsValue::from("Cluster B")),
                ("symbol", JsValue::from("rect")),
                ("symbolSize", JsValue::from(10.0)),
                ("itemStyle", obj(&[("color", JsValue::from("#ffffff")), ("borderColor", JsValue::from("#111111")), ("borderWidth", JsValue::from(2.0))])),
                ("data", pts(&[(18.0, 64.0), (25.0, 72.0), (31.0, 68.0), (39.0, 79.0), (47.0, 65.0), (55.0, 81.0), (64.0, 77.0), (72.0, 88.0), (79.0, 82.0), (88.0, 92.0)])),
            ]),
        ]),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").unwrap();
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 20 data points across 2 clusters",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
