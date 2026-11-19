#![no_main]

use konva::*;

fn add_caption(layer: &Layer, x: f64, y: f64, label: &str) {
    let tc = TextConfig::create();
    tc.set_x(Some(x));
    tc.set_y(Some(y));
    tc.set_text(Some(label));
    tc.set_font_size(Some(11.0));
    tc.set_font_family(Some("monospace"));
    tc.set_fill(StringOrCanvasGradient::String("#555555".to_string()));
    let t = Text::new(Some(&tc));
    layer.add([GroupOrShape::from(&t)]);
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

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    let r1 = RectConfig::create();
    r1.set_x(Some(40.0));
    r1.set_y(Some(55.0));
    r1.set_width(Some(110.0));
    r1.set_height(Some(76.0));
    r1.set_corner_radius(NumberOrNumberArray::Number(8.0));
    r1.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    layer.add([GroupOrShape::from(&Rect::new(Some(&r1)))]);
    add_caption(&layer, 40.0, 142.0, "Rect (filled)");

    let dash: JsArray<f64> = [6.0, 4.0].into_iter().collect();
    let r2 = RectConfig::create();
    r2.set_x(Some(195.0));
    r2.set_y(Some(55.0));
    r2.set_width(Some(110.0));
    r2.set_height(Some(76.0));
    r2.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    r2.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    r2.set_stroke_width(Some(2.0));
    r2.set_dash(Some(&dash));
    layer.add([GroupOrShape::from(&Rect::new(Some(&r2)))]);
    add_caption(&layer, 195.0, 142.0, "Rect (dashed)");

    let cc = CircleConfig::create();
    cc.set_x(Some(415.0));
    cc.set_y(Some(93.0));
    cc.set_radius(Some(40.0));
    cc.set_fill(StringOrCanvasGradient::String("#404040".to_string()));
    cc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    cc.set_stroke_width(Some(2.0));
    layer.add([GroupOrShape::from(&Circle::new(Some(&cc)))]);
    add_caption(&layer, 395.0, 142.0, "Circle");

    let pc = RegularPolygonConfig::create();
    pc.set_x(Some(575.0));
    pc.set_y(Some(93.0));
    pc.set_sides(6.0);
    pc.set_radius(42.0);
    pc.set_fill(StringOrCanvasGradient::String("#efefef".to_string()));
    pc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    pc.set_stroke_width(Some(2.0));
    layer.add([GroupOrShape::from(&RegularPolygon::new(Some(&pc)))]);
    add_caption(&layer, 548.0, 142.0, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    let st = StarConfig::create();
    st.set_x(Some(95.0));
    st.set_y(Some(268.0));
    st.set_num_points(5.0);
    st.set_inner_radius(19.0);
    st.set_outer_radius(44.0);
    st.set_fill(StringOrCanvasGradient::String("#737373".to_string()));
    st.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    st.set_stroke_width(Some(2.0));
    layer.add([GroupOrShape::from(&Star::new(Some(&st)))]);
    add_caption(&layer, 62.0, 325.0, "Star (5pt)");

    let rc = RingConfig::create();
    rc.set_x(Some(250.0));
    rc.set_y(Some(268.0));
    rc.set_inner_radius(22.0);
    rc.set_outer_radius(42.0);
    rc.set_fill(StringOrCanvasGradient::String("#d4d4d4".to_string()));
    rc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    rc.set_stroke_width(Some(2.0));
    layer.add([GroupOrShape::from(&Ring::new(Some(&rc)))]);
    add_caption(&layer, 236.0, 325.0, "Ring");

    let wc = WedgeConfig::create();
    wc.set_x(Some(415.0));
    wc.set_y(Some(268.0));
    wc.set_radius(44.0);
    wc.set_angle(270.0);
    wc.set_rotation(Some(-90.0));
    wc.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    wc.set_stroke(StringOrCanvasGradient::String("#555555".to_string()));
    wc.set_stroke_width(Some(1.0));
    layer.add([GroupOrShape::from(&Wedge::new(Some(&wc)))]);
    add_caption(&layer, 382.0, 325.0, "Wedge 270\u{00B0}");

    let tf = RectConfig::create();
    tf.set_x(Some(500.0));
    tf.set_y(Some(224.0));
    tf.set_width(Some(150.0));
    tf.set_height(Some(88.0));
    tf.set_fill(StringOrCanvasGradient::String("#f7f7f7".to_string()));
    tf.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    tf.set_stroke_width(Some(1.5));
    layer.add([GroupOrShape::from(&Rect::new(Some(&tf)))]);

    let tc = TextConfig::create();
    tc.set_x(Some(514.0));
    tc.set_y(Some(240.0));
    tc.set_text(Some("KONVA 2D\nMonochrome\nScene Graph"));
    tc.set_font_size(Some(13.0));
    tc.set_font_family(Some("monospace"));
    tc.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    layer.add([GroupOrShape::from(&Text::new(Some(&tc)))]);
    add_caption(&layer, 558.0, 325.0, "Text");

    layer.draw();

    let count = layer.get_children(None).len();
    let msg = format!("Stage: 680x400 \u{00B7} 7 primitive types \u{00B7} {count} layer children");
    dom::document().get_element_by_id("meta").unwrap().set_text_content(Some(&msg));
}
