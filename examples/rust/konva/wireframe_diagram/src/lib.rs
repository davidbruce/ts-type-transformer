#![no_main]

use konva::*;

fn add_stage_box(layer: &Layer, x: f64, title: &str, sub: &str, fill: &str, ink: &str) {
    let gc = GroupConfig::create();
    gc.set_x(Some(x));
    gc.set_y(Some(95.0));
    let g = Group::new(Some(&gc));

    let rc = RectConfig::create();
    rc.set_width(Some(126.0));
    rc.set_height(Some(68.0));
    rc.set_corner_radius(NumberOrNumberArray::Number(3.0));
    rc.set_fill(StringOrCanvasGradient::String(fill.to_string()));
    rc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    rc.set_stroke_width(Some(2.0));

    let t1 = TextConfig::create();
    t1.set_x(Some(12.0));
    t1.set_y(Some(16.0));
    t1.set_text(Some(title));
    t1.set_font_size(Some(12.0));
    t1.set_font_style(Some("bold"));
    t1.set_font_family(Some("monospace"));
    t1.set_fill(StringOrCanvasGradient::String(ink.to_string()));

    let t2 = TextConfig::create();
    t2.set_x(Some(12.0));
    t2.set_y(Some(38.0));
    t2.set_text(Some(sub));
    t2.set_font_size(Some(11.0));
    t2.set_font_family(Some("monospace"));
    t2.set_fill(StringOrCanvasGradient::String(ink.to_string()));

    g.add([
        GroupOrShape::from(&Rect::new(Some(&rc))),
        GroupOrShape::from(&Text::new(Some(&t1))),
        GroupOrShape::from(&Text::new(Some(&t2))),
    ]);
    layer.add([GroupOrShape::from(&g)]);
}

fn add_arrow(layer: &Layer, pts: [f64; 4]) {
    let arr: JsArray<f64> = pts.into_iter().collect();
    let ac = ArrowConfig::create();
    ac.set_points(LinePoints::Array(arr));
    ac.set_pointer_length(Some(8.0));
    ac.set_pointer_width(Some(8.0));
    ac.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    ac.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    ac.set_stroke_width(Some(2.0));
    layer.add([GroupOrShape::from(&Arrow::new(Some(&ac)))]);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sc = StageConfig::create();
    sc.set_container(HTMLDivElementOrString::String("stage".to_string()));
    sc.set_width(Some(680.0));
    sc.set_height(Some(400.0));
    let stage = Stage::new(&sc);

    let layer = Layer::new(None);
    stage.add(&layer, Vec::<JsValue>::new());

    let bg = RectConfig::create();
    bg.set_x(Some(0.0));
    bg.set_y(Some(0.0));
    bg.set_width(Some(680.0));
    bg.set_height(Some(400.0));
    bg.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    layer.add([GroupOrShape::from(&Rect::new(Some(&bg)))]);

    add_stage_box(&layer, 28.0, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(&layer, 194.0, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(&layer, 360.0, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(&layer, 526.0, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(&layer, [154.0, 129.0, 190.0, 129.0]);
    add_arrow(&layer, [320.0, 129.0, 356.0, 129.0]);
    add_arrow(&layer, [486.0, 129.0, 522.0, 129.0]);
    add_arrow(&layer, [257.0, 163.0, 257.0, 251.0]);
    add_arrow(&layer, [423.0, 163.0, 423.0, 251.0]);

    let fb_pts: JsArray<f64> = [589.0, 163.0, 589.0, 212.0, 91.0, 212.0, 91.0, 168.0].into_iter().collect();
    let dash: JsArray<f64> = [6.0, 4.0].into_iter().collect();
    let fbc = LineConfig::create();
    fbc.set_points(LinePoints::Array(fb_pts));
    fbc.set_stroke(StringOrCanvasGradient::String("#737373".to_string()));
    fbc.set_stroke_width(Some(1.5));
    fbc.set_dash(Some(&dash));
    let fb = Line::<LineConfig>::new(Some(fbc));
    layer.add([GroupOrShape::from(Shape::<LineConfig>::from(fb).upcast::<ShapeConfig>())]);

    let bar = RectConfig::create();
    bar.set_x(Some(28.0));
    bar.set_y(Some(255.0));
    bar.set_width(Some(624.0));
    bar.set_height(Some(58.0));
    bar.set_corner_radius(NumberOrNumberArray::Number(3.0));
    bar.set_fill(StringOrCanvasGradient::String("#f7f7f7".to_string()));
    bar.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    bar.set_stroke_width(Some(1.5));

    let btxt = TextConfig::create();
    btxt.set_x(Some(180.0));
    btxt.set_y(Some(278.0));
    btxt.set_text(Some("Shared DOM & Package Handle Runtime"));
    btxt.set_font_size(Some(12.0));
    btxt.set_font_family(Some("monospace"));
    btxt.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    layer.add([
        GroupOrShape::from(&Rect::new(Some(&bar))),
        GroupOrShape::from(&Text::new(Some(&btxt))),
    ]);

    layer.draw();

    let rects = layer.find::<Node<NodeConfig>>(JsValue::from("Rect")).len();
    let arrows = layer.find::<Node<NodeConfig>>(JsValue::from("Arrow")).len();
    let json_len = layer.to_json().len();

    let msg = format!("Pipeline diagram \u{00B7} {rects} rects \u{00B7} {arrows} arrows \u{00B7} JSON {json_len} bytes");
    dom::document().get_element_by_id("meta").unwrap().set_text_content(Some(&msg));
}
