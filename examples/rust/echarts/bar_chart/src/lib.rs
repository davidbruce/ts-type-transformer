#![no_main]

use echarts::*;

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").expect("missing #chart");
    let chart = init(&box_el, None, None);

    let title = TitleComponentOption::default()
        .with_text("Monthly Output")
        .with_text_style(&TitleTextStyleOption::default().with_color("#111111").with_font_size(13.0));
    title.set_left(16.0);
    title.set_top(12.0);

    let grid = GridComponentOption::default();
    grid.set_left(48.0);
    grid.set_right(24.0);
    grid.set_top(48.0);
    grid.set_bottom(36.0);

    let x_axis = XAXisComponentOptionXAxis::default()
        .with_data(["Jan", "Feb", "Mar", "Apr", "May", "Jun"])
        .with_axis_line(&AxisLineOption::default().with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#111111"))))
        .with_axis_label(&AxisLabelOption::default().with_color("#555555"));

    let y_axis = YAXisComponentOptionYAxisValue::default()
        .with_min(0.0)
        .with_max(100.0)
        .with_split_line(&SplitLineOption::default().with_line_style(&LineStyleOption::<JsValue>::default().with_color(JsValue::from("#e5e5e5"))))
        .with_axis_label(&AxisLabelOption::default().with_color("#555555"));

    let bars = BarSeriesOption::default()
        .with_name("Units")
        .with_bar_width("46%")
        .with_data([42.0, 68.0, 55.0, 84.0, 71.0, 93.0])
        .with_item_style(&BarItemStyleOption::default().with_color("#111111"));

    let opt = EChartsOption::default().with_animation(false);
    opt.set_background_color("#ffffff");
    opt.set_title(&title);
    opt.set_grid(&grid);
    opt.set_x_axis(&x_axis);
    opt.set_y_axis(&y_axis);
    opt.set_series(&bars);
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(&format!(
        "Canvas: {}x{} \u{00B7} 6 bars \u{00B7} Monochrome #111111",
        chart.get_width() as i32,
        chart.get_height() as i32
    ));
}
