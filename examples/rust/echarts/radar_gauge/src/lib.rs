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

fn ind(name: &str) -> JsValue {
    obj(&[("name", JsValue::from(name)), ("max", JsValue::from(100.0))])
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let radar = echarts::init(Some(&doc.get_element_by_id("chart-radar").unwrap()), None, None);
    let gauge = echarts::init(Some(&doc.get_element_by_id("chart-gauge").unwrap()), None, None);

    let radar_opt = EChartsCoreOption::create();
    radar_opt.set_animation(Some(false));
    radar_opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    radar_opt.set_at(
        "legend",
        obj(&[
            ("bottom", JsValue::from(6.0)),
            ("textStyle", obj(&[("color", JsValue::from("#111111"))])),
        ]),
    );
    radar_opt.set_at(
        "radar",
        obj(&[
            ("radius", JsValue::from("62%")),
            ("axisName", obj(&[("color", JsValue::from("#111111"))])),
            ("splitLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#d4d4d4"))]))])),
            ("splitArea", obj(&[("areaStyle", obj(&[("color", arr(vec![JsValue::from("#ffffff"), JsValue::from("#f7f7f7")]))]))])),
            ("axisLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#b5b5b5"))]))])),
            ("indicator", arr(vec![ind("Throughput"), ind("Memory"), ind("Startup"), ind("Type Safety"), ind("Bundle")])),
        ]),
    );
    radar_opt.set_at(
        "series",
        arr(vec![obj(&[
            ("type", JsValue::from("radar")),
            (
                "data",
                arr(vec![
                    obj(&[
                        ("name", JsValue::from("WASM Target")),
                        ("value", arr([92.0, 86.0, 88.0, 96.0, 84.0].into_iter().map(JsValue::from).collect())),
                        ("lineStyle", obj(&[("color", JsValue::from("#111111")), ("width", JsValue::from(2.0))])),
                        ("itemStyle", obj(&[("color", JsValue::from("#111111"))])),
                        ("areaStyle", obj(&[("color", JsValue::from("rgba(17,17,17,0.12)"))])),
                    ]),
                    obj(&[
                        ("name", JsValue::from("Baseline")),
                        ("value", arr([70.0, 68.0, 74.0, 72.0, 66.0].into_iter().map(JsValue::from).collect())),
                        ("lineStyle", obj(&[("color", JsValue::from("#737373")), ("width", JsValue::from(1.5)), ("type", JsValue::from("dashed"))])),
                        ("itemStyle", obj(&[("color", JsValue::from("#737373"))])),
                    ]),
                ]),
            ),
        ])]),
    );
    radar.set_option(radar_opt, None, None);

    let gauge_opt = EChartsCoreOption::create();
    gauge_opt.set_animation(Some(false));
    gauge_opt.set_at("backgroundColor", JsValue::from("#ffffff"));
    gauge_opt.set_at(
        "series",
        arr(vec![obj(&[
            ("type", JsValue::from("gauge")),
            ("min", JsValue::from(0.0)),
            ("max", JsValue::from(100.0)),
            ("progress", obj(&[("show", JsValue::from(true)), ("width", JsValue::from(14.0)), ("itemStyle", obj(&[("color", JsValue::from("#111111"))]))])),
            ("axisLine", obj(&[("lineStyle", obj(&[("width", JsValue::from(14.0)), ("color", arr(vec![arr(vec![JsValue::from(1.0), JsValue::from("#e5e5e5")])]))]))])),
            ("axisTick", obj(&[("lineStyle", obj(&[("color", JsValue::from("#737373"))]))])),
            ("splitLine", obj(&[("lineStyle", obj(&[("color", JsValue::from("#111111"))]))])),
            ("axisLabel", obj(&[("color", JsValue::from("#555555"))])),
            ("pointer", obj(&[("itemStyle", obj(&[("color", JsValue::from("#111111"))]))])),
            ("detail", obj(&[("valueAnimation", JsValue::from(false)), ("formatter", JsValue::from("{value}%")), ("color", JsValue::from("#111111")), ("fontSize", JsValue::from(22.0))])),
            ("title", obj(&[("color", JsValue::from("#555555")), ("fontSize", JsValue::from(12.0))])),
            ("data", arr(vec![obj(&[("value", JsValue::from(84.0)), ("name", JsValue::from("Score"))])])),
        ])]),
    );
    gauge.set_option(gauge_opt, None, None);

    let meta = doc.get_element_by_id("meta").unwrap();
    meta.set_text_content(Some(&format!(
        "Radar: {}x{} \u{00B7} Gauge: {}x{} \u{00B7} Score 84%",
        radar.get_width() as i32,
        radar.get_height() as i32,
        gauge.get_width() as i32,
        gauge.get_height() as i32
    )));
}
