#![no_main]

use echarts::{EChartsCoreOption, JsArray, JsValue};

fn value_axis() -> JsValue {
    JsValue::object()
        .with("type", "value")
        .with("min", 0.0)
        .with("max", 100.0)
        .with("splitLine", JsValue::object().with("lineStyle", JsValue::object().with("color", "#e5e5e5")))
        .with("axisLabel", JsValue::object().with("color", "#555555"))
}

fn pts(coords: &[(f64, f64)]) -> JsArray<JsValue> {
    coords
        .iter()
        .map(|&(x, y)| JsValue::from(JsArray::<JsValue>::from([x, y])))
        .collect()
}

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
    opt.set_at(
        "grid",
        JsValue::object()
            .with("left", 48.0)
            .with("right", 24.0)
            .with("top", 48.0)
            .with("bottom", 36.0),
    );
    opt.set_at("xAxis", value_axis());
    opt.set_at("yAxis", value_axis());
    opt.set_at(
        "series",
        JsValue::from(JsArray::<JsValue>::from([
            JsValue::object()
                .with("type", "scatter")
                .with("name", "Cluster A")
                .with("symbol", "circle")
                .with("symbolSize", 10.0)
                .with("itemStyle", JsValue::object().with("color", "#111111"))
                .with(
                    "data",
                    pts(&[
                        (14.0, 22.0), (21.0, 31.0), (28.0, 27.0), (33.0, 42.0), (40.0, 48.0),
                        (46.0, 53.0), (54.0, 61.0), (62.0, 59.0), (71.0, 74.0), (82.0, 85.0),
                    ]),
                ),
            JsValue::object()
                .with("type", "scatter")
                .with("name", "Cluster B")
                .with("symbol", "rect")
                .with("symbolSize", 10.0)
                .with(
                    "itemStyle",
                    JsValue::object()
                        .with("color", "#ffffff")
                        .with("borderColor", "#111111")
                        .with("borderWidth", 2.0),
                )
                .with(
                    "data",
                    pts(&[
                        (18.0, 64.0), (25.0, 72.0), (31.0, 68.0), (39.0, 79.0), (47.0, 65.0),
                        (55.0, 81.0), (64.0, 77.0), (72.0, 88.0), (79.0, 82.0), (88.0, 92.0),
                    ]),
                ),
        ])),
    );
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(Some(&format!(
        "Canvas: {}x{} \u{00B7} 20 data points across 2 clusters",
        chart.get_width() as i32,
        chart.get_height() as i32
    )));
}
