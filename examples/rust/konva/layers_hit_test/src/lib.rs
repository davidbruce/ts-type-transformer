#![no_main]

use core::cell::Cell;
use konva::*;
use std::rc::Rc;

fn make_line(pts: [f64; 4], col: &str, width: f64) -> GroupOrShape {
    let lc = LineConfig::default()
        .with_points(JsArray::from(pts))
        .with_stroke(col)
        .with_stroke_width(width);
    let ln = Line::<LineConfig>::new(Some(lc));
    Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>().into()
}

fn make_card(id: &str, x: f64, y: f64, fill: &str) -> Rect {
    let rc = RectConfig::default()
        .with_id(id)
        .with_name("card")
        .with_x(x)
        .with_y(y)
        .with_width(200.0)
        .with_height(130.0)
        .with_fill(fill)
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    Rect::new(Some(&rc))
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sc = StageConfig::default()
        .with_container("stage")
        .with_width(680.0)
        .with_height(400.0);
    let stage = Stage::new(&sc);

    let bg_layer = Layer::new(None);
    let fg_layer = Layer::new(None);
    stage.add(&bg_layer, JsArray::<JsValue>::new());
    stage.add(&fg_layer, JsArray::<JsValue>::new());

    // Layer 1: Blueprint Grid.
    let bgr = RectConfig::default()
        .with_x(0.0)
        .with_y(0.0)
        .with_width(680.0)
        .with_height(400.0)
        .with_fill("#ffffff");
    bg_layer.add([Rect::new(Some(&bgr))]);

    for x in (40..=640).step_by(40) {
        let xf = x as f64;
        bg_layer.add([make_line([xf, 20.0, xf, 380.0], "#e5e5e5", 1.0)]);
    }
    for y in (40..=360).step_by(40) {
        let yf = y as f64;
        bg_layer.add([make_line([20.0, yf, 660.0, yf], "#e5e5e5", 1.0)]);
    }

    let frame = RectConfig::default()
        .with_x(20.0)
        .with_y(20.0)
        .with_width(640.0)
        .with_height(360.0)
        .with_stroke("#b5b5b5")
        .with_stroke_width(1.5)
        .with_dash([6.0, 4.0]);
    let l1 = TextConfig::default()
        .with_x(32.0)
        .with_y(32.0)
        .with_text("Layer 1: Blueprint Grid (z=0)")
        .with_font_size(12.0)
        .with_font_family("monospace")
        .with_fill("#737373");
    bg_layer.add([
        GroupOrShape::from(Rect::new(Some(&frame))),
        GroupOrShape::from(Text::new(Some(&l1))),
    ]);

    // Layer 2: Stacked cards & badge.
    let card_a = make_card("card-a", 110.0, 80.0, "#efefef");
    let card_b = make_card("card-b", 210.0, 125.0, "#404040");
    let card_c = make_card("card-c", 310.0, 170.0, "#ffffff");
    let bc = CircleConfig::default()
        .with_id("badge")
        .with_x(555.0)
        .with_y(135.0)
        .with_radius(38.0)
        .with_fill("#111111");
    fg_layer.add([
        GroupOrShape::from(&card_a),
        GroupOrShape::from(card_b),
        GroupOrShape::from(card_c),
        GroupOrShape::from(Circle::new(Some(&bc))),
    ]);

    card_a.move_to_top();
    let card_a_z = card_a.get_z_index() as i32;

    let clicks = Rc::new(Cell::new(0));
    let on_click = KonvaEventListener::<Node<RectConfig>, dom::PointerEvent>::new({
        let clicks = clicks.clone();
        move |_| clicks.set(clicks.get() + 1)
    });
    card_a.on_click(&on_click);
    card_a.fire("click", None, None);
    card_a.fire("click", None, None);

    bg_layer.draw();
    fg_layer.draw();

    let pt = Vector2d::default().with_x(250.0).with_y(160.0);
    let hit_id = stage
        .get_intersection(&pt)
        .map_or_else(|| "none".to_string(), |s| s.id().invoke());

    let dc = CircleConfig::default()
        .with_x(250.0)
        .with_y(160.0)
        .with_radius(7.0)
        .with_fill("#ffffff")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    let l2 = TextConfig::default()
        .with_x(32.0)
        .with_y(354.0)
        .with_text("Layer 2: Elevated Stack (z=1) \u{00B7} Probe (250,160)")
        .with_font_size(12.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    fg_layer.add([
        GroupOrShape::from(Circle::new(Some(&dc))),
        make_line([238.0, 160.0, 262.0, 160.0], "#111111", 1.5),
        make_line([250.0, 148.0, 250.0, 172.0], "#111111", 1.5),
        GroupOrShape::from(Text::new(Some(&l2))),
    ]);
    fg_layer.draw();

    let msg = format!(
        "Layers: {} \u{00B7} card-a zIndex={card_a_z} \u{00B7} hit@(250,160)={hit_id} \u{00B7} clicks={}",
        stage.get_layers().len(),
        clicks.get()
    );
    dom::document()
        .get_element_by_id("meta")
        .expect("missing #meta")
        .set_text_content(Some(&msg));
}
