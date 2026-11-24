#![no_main]

use echarts::{EChartsCoreOption, JsArray, JsValue};

fn ind(name: &str) -> JsValue {
    JsValue::object().with("name", name).with("max", 100.0)
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let radar = echarts::init_with(
        &doc.get_element_by_id("chart-radar")
            .expect("missing #chart-radar"),
        None,
        None,
    );
    let gauge = echarts::init_with(
        &doc.get_element_by_id("chart-gauge")
            .expect("missing #chart-gauge"),
        None,
        None,
    );

    let radar_opt = EChartsCoreOption::default().with_animation(false);
    radar_opt.set_at("backgroundColor", "#ffffff");
    radar_opt.set_at(
        "legend",
        JsValue::object()
            .with("bottom", 6.0)
            .with("textStyle", JsValue::object().with("color", "#111111")),
    );
    radar_opt.set_at(
        "radar",
        JsValue::object()
            .with("radius", "62%")
            .with("axisName", JsValue::object().with("color", "#111111"))
            .with(
                "splitLine",
                JsValue::object().with("lineStyle", JsValue::object().with("color", "#d4d4d4")),
            )
            .with(
                "splitArea",
                JsValue::object().with(
                    "areaStyle",
                    JsValue::object()
                        .with("color", JsArray::<JsValue>::from(["#ffffff", "#f7f7f7"])),
                ),
            )
            .with(
                "axisLine",
                JsValue::object().with("lineStyle", JsValue::object().with("color", "#b5b5b5")),
            )
            .with(
                "indicator",
                JsArray::<JsValue>::from([
                    ind("Throughput"),
                    ind("Memory"),
                    ind("Startup"),
                    ind("Type Safety"),
                    ind("Bundle"),
                ]),
            ),
    );
    radar_opt.set_at(
        "series",
        JsValue::from(JsArray::<JsValue>::from([JsValue::object()
            .with("type", "radar")
            .with(
                "data",
                JsArray::<JsValue>::from([
                    JsValue::object()
                        .with("name", "WASM Target")
                        .with(
                            "value",
                            JsArray::<JsValue>::from([92.0, 86.0, 88.0, 96.0, 84.0]),
                        )
                        .with(
                            "lineStyle",
                            JsValue::object()
                                .with("color", "#111111")
                                .with("width", 2.0),
                        )
                        .with("itemStyle", JsValue::object().with("color", "#111111"))
                        .with(
                            "areaStyle",
                            JsValue::object().with("color", "rgba(17,17,17,0.12)"),
                        ),
                    JsValue::object()
                        .with("name", "Baseline")
                        .with(
                            "value",
                            JsArray::<JsValue>::from([70.0, 68.0, 74.0, 72.0, 66.0]),
                        )
                        .with(
                            "lineStyle",
                            JsValue::object()
                                .with("color", "#737373")
                                .with("width", 1.5)
                                .with("type", "dashed"),
                        )
                        .with("itemStyle", JsValue::object().with("color", "#737373")),
                ]),
            )])),
    );
    radar.set_option(&radar_opt);

    let gauge_opt = EChartsCoreOption::default().with_animation(false);
    gauge_opt.set_at("backgroundColor", "#ffffff");
    gauge_opt.set_at(
        "series",
        JsValue::from(JsArray::<JsValue>::from([JsValue::object()
            .with("type", "gauge")
            .with("min", 0.0)
            .with("max", 100.0)
            .with(
                "progress",
                JsValue::object()
                    .with("show", true)
                    .with("width", 14.0)
                    .with("itemStyle", JsValue::object().with("color", "#111111")),
            )
            .with(
                "axisLine",
                JsValue::object().with(
                    "lineStyle",
                    JsValue::object().with("width", 14.0).with(
                        "color",
                        JsArray::<JsValue>::from([JsArray::<JsValue>::from([
                            JsValue::from(1.0),
                            JsValue::from("#e5e5e5"),
                        ])]),
                    ),
                ),
            )
            .with(
                "axisTick",
                JsValue::object().with("lineStyle", JsValue::object().with("color", "#737373")),
            )
            .with(
                "splitLine",
                JsValue::object().with("lineStyle", JsValue::object().with("color", "#111111")),
            )
            .with("axisLabel", JsValue::object().with("color", "#555555"))
            .with(
                "pointer",
                JsValue::object().with("itemStyle", JsValue::object().with("color", "#111111")),
            )
            .with(
                "detail",
                JsValue::object()
                    .with("valueAnimation", false)
                    .with("formatter", "{value}%")
                    .with("color", "#111111")
                    .with("fontSize", 22.0),
            )
            .with(
                "title",
                JsValue::object()
                    .with("color", "#555555")
                    .with("fontSize", 12.0),
            )
            .with(
                "data",
                JsArray::<JsValue>::from([JsValue::object()
                    .with("value", 84.0)
                    .with("name", "Score")]),
            )])),
    );
    gauge.set_option(&gauge_opt);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(&format!(
        "Radar: {}x{} \u{00B7} Gauge: {}x{} \u{00B7} Score 84%",
        radar.get_width() as i32,
        radar.get_height() as i32,
        gauge.get_width() as i32,
        gauge.get_height() as i32
    ));
}
