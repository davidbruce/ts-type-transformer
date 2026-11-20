#![no_main]

use konva::*;

fn add_stage_box(layer: &Layer, x: f64, title: &str, sub: &str, fill: &str, ink: &str) {
    let gc = GroupConfig::default().with_x(x).with_y(95.0);
    let g = Group::new(Some(&gc));

    let rc = RectConfig::default()
        .with_width(126.0)
        .with_height(68.0)
        .with_corner_radius(3.0)
        .with_fill(fill)
        .with_stroke("#111111")
        .with_stroke_width(2.0);

    let t1 = TextConfig::default()
        .with_x(12.0)
        .with_y(16.0)
        .with_text(title)
        .with_font_size(12.0)
        .with_font_style("bold")
        .with_font_family("monospace")
        .with_fill(ink);

    let t2 = TextConfig::default()
        .with_x(12.0)
        .with_y(38.0)
        .with_text(sub)
        .with_font_size(11.0)
        .with_font_family("monospace")
        .with_fill(ink);

    g.add([
        GroupOrShape::from(Rect::new(Some(&rc))),
        GroupOrShape::from(Text::new(Some(&t1))),
        GroupOrShape::from(Text::new(Some(&t2))),
    ]);
    layer.add([g]);
}

fn add_arrow(layer: &Layer, pts: [f64; 4]) {
    let ac = ArrowConfig::default()
        .with_points(JsArray::from(pts))
        .with_pointer_length(8.0)
        .with_pointer_width(8.0)
        .with_fill("#111111")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    layer.add([Arrow::new(Some(&ac))]);
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
    layer.add([Rect::new(Some(&bg))]);

    add_stage_box(&layer, 28.0, "01 PARSE", ".d.ts AST", "#ffffff", "#111111");
    add_stage_box(&layer, 194.0, "02 LOWER", "Typed IR", "#f7f7f7", "#111111");
    add_stage_box(&layer, 360.0, "03 EMIT", "6 Targets", "#efefef", "#111111");
    add_stage_box(&layer, 526.0, "04 RUN", "WASM / JS", "#111111", "#ffffff");

    add_arrow(&layer, [154.0, 129.0, 190.0, 129.0]);
    add_arrow(&layer, [320.0, 129.0, 356.0, 129.0]);
    add_arrow(&layer, [486.0, 129.0, 522.0, 129.0]);
    add_arrow(&layer, [257.0, 163.0, 257.0, 251.0]);
    add_arrow(&layer, [423.0, 163.0, 423.0, 251.0]);

    let fbc = LineConfig::default()
        .with_points(JsArray::from([589.0, 163.0, 589.0, 212.0, 91.0, 212.0, 91.0, 168.0]))
        .with_stroke("#737373")
        .with_stroke_width(1.5)
        .with_dash([6.0, 4.0]);
    let fb = Line::<LineConfig>::new(Some(fbc));
    layer.add([GroupOrShape::from(Shape::<LineConfig>::from(fb).upcast::<ShapeConfig>())]);

    let bar = RectConfig::default()
        .with_x(28.0)
        .with_y(255.0)
        .with_width(624.0)
        .with_height(58.0)
        .with_corner_radius(3.0)
        .with_fill("#f7f7f7")
        .with_stroke("#111111")
        .with_stroke_width(1.5);

    let btxt = TextConfig::default()
        .with_x(180.0)
        .with_y(278.0)
        .with_text("Shared DOM & Package Handle Runtime")
        .with_font_size(12.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    layer.add([
        GroupOrShape::from(Rect::new(Some(&bar))),
        GroupOrShape::from(Text::new(Some(&btxt))),
    ]);

    layer.draw();

    let rects = layer.find::<Node<NodeConfig>>(JsValue::from("Rect")).len();
    let arrows = layer.find::<Node<NodeConfig>>(JsValue::from("Arrow")).len();
    let json_len = layer.to_json().len();

    let msg = format!("Pipeline diagram \u{00B7} {rects} rects \u{00B7} {arrows} arrows \u{00B7} JSON {json_len} bytes");
    dom::document()
        .get_element_by_id("meta")
        .expect("missing #meta")
        .set_text_content(Some(&msg));
}
