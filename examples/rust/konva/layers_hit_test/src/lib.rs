#![no_main]

use core::cell::Cell;
use konva::*;
use std::rc::Rc;

fn make_line(pts: [f64; 4], col: &str, width: f64) -> GroupOrShape {
    let arr: JsArray<f64> = pts.into_iter().collect();
    let lc = LineConfig::create();
    lc.set_points(LinePoints::Array(arr));
    lc.set_stroke(StringOrCanvasGradient::String(col.to_string()));
    lc.set_stroke_width(Some(width));
    let ln = Line::<LineConfig>::new(Some(lc));
    GroupOrShape::from(Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>())
}

fn make_card(id: &str, x: f64, y: f64, fill: &str) -> Rect {
    let rc = RectConfig::create();
    rc.set_id(Some(id));
    rc.set_name(Some("card"));
    rc.set_x(Some(x));
    rc.set_y(Some(y));
    rc.set_width(Some(200.0));
    rc.set_height(Some(130.0));
    rc.set_fill(StringOrCanvasGradient::String(fill.to_string()));
    rc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    rc.set_stroke_width(Some(2.0));
    Rect::new(Some(&rc))
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let sc = StageConfig::create();
    sc.set_container(HTMLDivElementOrString::String("stage".to_string()));
    sc.set_width(Some(680.0));
    sc.set_height(Some(400.0));
    let stage = Stage::new(&sc);

    let bg_layer = Layer::new(None);
    let fg_layer = Layer::new(None);
    stage.add(&bg_layer, Vec::<JsValue>::new());
    stage.add(&fg_layer, Vec::<JsValue>::new());

    // Layer 1: Blueprint Grid.
    let bgr = RectConfig::create();
    bgr.set_x(Some(0.0));
    bgr.set_y(Some(0.0));
    bgr.set_width(Some(680.0));
    bgr.set_height(Some(400.0));
    bgr.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    bg_layer.add([GroupOrShape::from(&Rect::new(Some(&bgr)))]);

    for x in (40..=640).step_by(40) {
        let xf = x as f64;
        bg_layer.add([make_line([xf, 20.0, xf, 380.0], "#e5e5e5", 1.0)]);
    }
    for y in (40..=360).step_by(40) {
        let yf = y as f64;
        bg_layer.add([make_line([20.0, yf, 660.0, yf], "#e5e5e5", 1.0)]);
    }

    let dash: JsArray<f64> = [6.0, 4.0].into_iter().collect();
    let frame = RectConfig::create();
    frame.set_x(Some(20.0));
    frame.set_y(Some(20.0));
    frame.set_width(Some(640.0));
    frame.set_height(Some(360.0));
    frame.set_stroke(StringOrCanvasGradient::String("#b5b5b5".to_string()));
    frame.set_stroke_width(Some(1.5));
    frame.set_dash(Some(&dash));
    let l1 = TextConfig::create();
    l1.set_x(Some(32.0));
    l1.set_y(Some(32.0));
    l1.set_text(Some("Layer 1: Blueprint Grid (z=0)"));
    l1.set_font_size(Some(12.0));
    l1.set_font_family(Some("monospace"));
    l1.set_fill(StringOrCanvasGradient::String("#737373".to_string()));
    bg_layer.add([
        GroupOrShape::from(&Rect::new(Some(&frame))),
        GroupOrShape::from(&Text::new(Some(&l1))),
    ]);

    // Layer 2: Stacked cards & badge.
    let card_a = make_card("card-a", 110.0, 80.0, "#efefef");
    let card_b = make_card("card-b", 210.0, 125.0, "#404040");
    let card_c = make_card("card-c", 310.0, 170.0, "#ffffff");
    let bc = CircleConfig::create();
    bc.set_id(Some("badge"));
    bc.set_x(Some(555.0));
    bc.set_y(Some(135.0));
    bc.set_radius(Some(38.0));
    bc.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    fg_layer.add([
        GroupOrShape::from(&card_a),
        GroupOrShape::from(&card_b),
        GroupOrShape::from(&card_c),
        GroupOrShape::from(&Circle::new(Some(&bc))),
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

    let pt = Vector2d::create();
    pt.set_x(250.0);
    pt.set_y(160.0);
    let hit_id = stage
        .get_intersection(&pt)
        .map_or_else(|| "none".to_string(), |s| s.id().invoke());

    let dc = CircleConfig::create();
    dc.set_x(Some(250.0));
    dc.set_y(Some(160.0));
    dc.set_radius(Some(7.0));
    dc.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    dc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    dc.set_stroke_width(Some(2.0));
    let l2 = TextConfig::create();
    l2.set_x(Some(32.0));
    l2.set_y(Some(354.0));
    l2.set_text(Some("Layer 2: Elevated Stack (z=1) \u{00B7} Probe (250,160)"));
    l2.set_font_size(Some(12.0));
    l2.set_font_family(Some("monospace"));
    l2.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    fg_layer.add([
        GroupOrShape::from(&Circle::new(Some(&dc))),
        make_line([238.0, 160.0, 262.0, 160.0], "#111111", 1.5),
        make_line([250.0, 148.0, 250.0, 172.0], "#111111", 1.5),
        GroupOrShape::from(&Text::new(Some(&l2))),
    ]);
    fg_layer.draw();

    let msg = format!(
        "Layers: {} \u{00B7} card-a zIndex={card_a_z} \u{00B7} hit@(250,160)={hit_id} \u{00B7} clicks={}",
        stage.get_layers().len(),
        clicks.get()
    );
    dom::document().get_element_by_id("meta").unwrap().set_text_content(Some(&msg));
}
