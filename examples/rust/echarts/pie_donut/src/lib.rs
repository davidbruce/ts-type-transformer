#![no_main]

use echarts::*;

fn slice(name: &str, value: f64) -> PieDataItemOption {
    let item = PieDataItemOption::default();
    item.set_name(name);
    item.set_value(TArrayOrT::T(OptionDataValueNumeric::Number(value)));
    item
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let box_el = doc.get_element_by_id("chart").expect("missing #chart");
    let chart = init(&box_el, None, None);

    let legend = PlainLegendComponentOption::default()
        .with_bottom(8.0)
        .with_text_style(&LabelOption::<DetailOptionObject>::default().with_color("#111111"));
    let tooltip = TooltipComponentOption::default().with_trigger(TooltipComponentOptionTrigger::Item);

    let pie = PieSeriesOption::default()
        .with_name("Subsystems")
        .with_radius(JsArray::<PositionSizeOption>::from(["44%", "72%"]))
        .with_center(JsArray::<PositionSizeOption>::from(["50%", "46%"]))
        .with_item_style(&PieItemStyleOption::<PieCallbackDataParams>::default().with_border_color("#ffffff").with_border_width(2.0))
        .with_label(&PieLabelOption::default().with_color("#111111"))
        .with_data([slice("Runtime", 34.0), slice("Bindings", 26.0), slice("Glue", 18.0), slice("Parser", 14.0), slice("Tests", 8.0)]);

    let opt = EChartsOption::default().with_animation(false);
    opt.set_background_color("#ffffff");
    opt.set_color(JsArray::<JsValue>::from(["#111111", "#404040", "#737373", "#a3a3a3", "#d4d4d4"]));
    opt.set_legend(&legend);
    opt.set_tooltip(&tooltip);
    opt.set_series(&pie);
    chart.set_option(opt, None, None);

    let meta = doc.get_element_by_id("meta").expect("missing #meta");
    meta.set_text_content(&format!(
        "Canvas: {}x{} \u{00B7} 5 monochrome segments \u{00B7} Total 100",
        chart.get_width() as i32,
        chart.get_height() as i32
    ));
}
