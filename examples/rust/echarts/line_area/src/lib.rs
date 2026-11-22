#![no_main]

use echarts::*;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").expect("missing #chart");
    let chart = init(&box_el, None, None);

    let legend = PlainLegendComponentOption::default()
        .with_top(8.0)
        .with_text_style(&LabelOption::<DetailOptionObject>::default().with_color("#111111"));
    let tooltip = TooltipComponentOption::default().with_trigger(TooltipComponentOptionTrigger::Axis);

    let grid = GridComponentOption::default();
    grid.set_left(48.0);
    grid.set_right(24.0);
    grid.set_top(48.0);
    grid.set_bottom(36.0);

    let x_axis = XAXisComponentOptionXAxis::default()
        .with_boundary_gap(false)
        .with_data(["W1", "W2", "W3", "W4", "W5", "W6", "W7", "W8"])
        .with_axis_line(&AxisLineOption::default().with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#111111"))))
        .with_axis_label(&AxisLabelOption::default().with_color("#555555"));

    let y_axis = YAXisComponentOptionYAxisValue::default()
        .with_min(0.0)
        .with_max(100.0)
        .with_split_line(&SplitLineOption::default().with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#e5e5e5"))))
        .with_axis_label(&AxisLabelOption::default().with_color("#555555"));

    let throughput = LineSeriesOption::default()
        .with_name("Throughput")
        .with_smooth(true)
        .with_data([28.0, 44.0, 39.0, 62.0, 58.0, 76.0, 81.0, 89.0])
        .with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#111111")).with_width(2.0))
        .with_item_style(&ItemStyleOption::<DefaultLabelFormatterCallbackParams>::default().with_color("#111111"))
        .with_area_style(&LineSeriesOptionAreaStyle::default().with_color("rgba(17,17,17,0.12)"));
    let baseline = LineSeriesOption::default()
        .with_name("Baseline")
        .with_smooth(false)
        .with_data([35.0, 38.0, 42.0, 46.0, 50.0, 54.0, 58.0, 62.0])
        .with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#737373")).with_width(1.5).with_type("dashed"))
        .with_item_style(&ItemStyleOption::<DefaultLabelFormatterCallbackParams>::default().with_color("#737373"));

    let opt = EChartsOption::default().with_animation(false);
    opt.set_background_color("#ffffff");
    opt.set_legend(&legend);
    opt.set_tooltip(&tooltip);
    opt.set_grid(&grid);
    opt.set_x_axis(&x_axis);
    opt.set_y_axis(&y_axis);
    opt.set_series(&JsArray::<JsValue>::from([JsValue::from(&throughput), JsValue::from(&baseline)]));
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(&format!(
        "Canvas: {}x{} \u{00B7} 2 series (Throughput + Baseline)",
        chart.get_width() as i32,
        chart.get_height() as i32
    ));
}
