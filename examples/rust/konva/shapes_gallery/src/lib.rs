#![no_main]

use konva::*;

fn add_caption(layer: &Layer, x: f64, y: f64, label: &str) {
    let tc = TextConfig::default()
        .with_x(x)
        .with_y(y)
        .with_text(label)
        .with_font_size(11.0)
        .with_font_family("monospace")
        .with_fill("#555555");
    layer.add([Text::new(&tc)]);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sc = StageConfig::default()
        .with_container("stage")
        .with_width(680.0)
        .with_height(400.0);
    let stage = Stage::new(&sc);

    let layer = Layer::new(None);
    stage.add(&layer, JsArray::<JsValue>::new());

    let bg = RectConfig::default()
        .with_x(0.0)
        .with_y(0.0)
        .with_width(680.0)
        .with_height(400.0)
        .with_fill("#ffffff");
    layer.add([Rect::new(&bg)]);

    // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
    let r1 = RectConfig::default()
        .with_x(40.0)
        .with_y(55.0)
        .with_width(110.0)
        .with_height(76.0)
        .with_corner_radius(8.0)
        .with_fill("#111111");
    layer.add([Rect::new(&r1)]);
    add_caption(&layer, 40.0, 142.0, "Rect (filled)");

    let r2 = RectConfig::default()
        .with_x(195.0)
        .with_y(55.0)
        .with_width(110.0)
        .with_height(76.0)
        .with_fill("#ffffff")
        .with_stroke("#111111")
        .with_stroke_width(2.0)
        .with_dash([6.0, 4.0]);
    layer.add([Rect::new(&r2)]);
    add_caption(&layer, 195.0, 142.0, "Rect (dashed)");

    let cc = CircleConfig::default()
        .with_x(415.0)
        .with_y(93.0)
        .with_radius(40.0)
        .with_fill("#404040")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    layer.add([Circle::new(&cc)]);
    add_caption(&layer, 395.0, 142.0, "Circle");

    let pc = RegularPolygonConfig::default()
        .with_x(575.0)
        .with_y(93.0)
        .with_sides(6.0)
        .with_radius(42.0)
        .with_fill("#efefef")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    layer.add([RegularPolygon::new(&pc)]);
    add_caption(&layer, 548.0, 142.0, "Hexagon");

    // Row 2: Star, Ring, Wedge, Framed Text.
    let st = StarConfig::default()
        .with_x(95.0)
        .with_y(268.0)
        .with_num_points(5.0)
        .with_inner_radius(19.0)
        .with_outer_radius(44.0)
        .with_fill("#737373")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    layer.add([Star::new(&st)]);
    add_caption(&layer, 62.0, 325.0, "Star (5pt)");

    let rc = RingConfig::default()
        .with_x(250.0)
        .with_y(268.0)
        .with_inner_radius(22.0)
        .with_outer_radius(42.0)
        .with_fill("#d4d4d4")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    layer.add([Ring::new(&rc)]);
    add_caption(&layer, 236.0, 325.0, "Ring");

    let wc = WedgeConfig::default()
        .with_x(415.0)
        .with_y(268.0)
        .with_radius(44.0)
        .with_angle(270.0)
        .with_rotation(-90.0)
        .with_fill("#111111")
        .with_stroke("#555555")
        .with_stroke_width(1.0);
    layer.add([Wedge::new(&wc)]);
    add_caption(&layer, 382.0, 325.0, "Wedge 270\u{00B0}");

    let tf = RectConfig::default()
        .with_x(500.0)
        .with_y(224.0)
        .with_width(150.0)
        .with_height(88.0)
        .with_fill("#f7f7f7")
        .with_stroke("#111111")
        .with_stroke_width(1.5);
    layer.add([Rect::new(&tf)]);

    let tc = TextConfig::default()
        .with_x(514.0)
        .with_y(240.0)
        .with_text("KONVA 2D\nMonochrome\nScene Graph")
        .with_font_size(13.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    layer.add([Text::new(&tc)]);
    add_caption(&layer, 558.0, 325.0, "Text");

    layer.draw();

    let count = layer.get_children(None).len();
    let msg = format!("Stage: 680x400 \u{00B7} 7 primitive types \u{00B7} {count} layer children");
    dom::document()
        .get_element_by_id("meta")
        .expect("missing #meta")
        .set_text_content(&msg);
}
