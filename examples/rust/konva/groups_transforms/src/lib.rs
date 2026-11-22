#![no_main]

use konva::*;

fn make_line(pts: [f64; 4]) -> GroupOrShape {
    let lc = LineConfig::default()
        .with_points(JsArray::from(pts))
        .with_stroke("#e5e5e5")
        .with_stroke_width(1.0);
    let ln = Line::<LineConfig>::new(Some(lc));
    Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>().into()
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

    for x in (40..=640).step_by(40) {
        let xf = x as f64;
        layer.add([make_line([xf, 0.0, xf, 400.0])]);
    }
    for y in (40..=360).step_by(40) {
        let yf = y as f64;
        layer.add([make_line([0.0, yf, 680.0, yf])]);
    }

    // Group A.
    let gac = GroupConfig::default().with_x(40.0).with_y(55.0);
    let group_a = Group::new(&gac);
    let gab = RectConfig::default()
        .with_width(160.0)
        .with_height(120.0)
        .with_fill("#f7f7f7")
        .with_stroke("#111111")
        .with_stroke_width(1.5);
    let gad = CircleConfig::default()
        .with_x(80.0)
        .with_y(60.0)
        .with_radius(24.0)
        .with_fill("#111111");
    let gal = TextConfig::default()
        .with_x(12.0)
        .with_y(10.0)
        .with_text("Group A \u{00B7} 0\u{00B0}")
        .with_font_size(11.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    group_a.add([
        GroupOrShape::from(Rect::new(&gab)),
        GroupOrShape::from(Circle::new(&gad)),
        GroupOrShape::from(Text::new(&gal)),
    ]);
    layer.add([group_a]);

    // Group B.
    let gbc = GroupConfig::default()
        .with_x(340.0)
        .with_y(75.0)
        .with_rotation(24.0);
    let group_b = Group::new(&gbc);
    let gbo = RectConfig::default()
        .with_width(150.0)
        .with_height(110.0)
        .with_fill("#efefef")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    let gbi = RectConfig::default()
        .with_x(22.0)
        .with_y(22.0)
        .with_width(106.0)
        .with_height(66.0)
        .with_fill("#ffffff")
        .with_stroke("#555555")
        .with_stroke_width(1.5)
        .with_dash([4.0, 3.0]);
    let gbd = CircleConfig::default()
        .with_x(75.0)
        .with_y(55.0)
        .with_radius(14.0)
        .with_fill("#404040");
    let gbl = TextConfig::default()
        .with_x(10.0)
        .with_y(8.0)
        .with_text("Group B \u{00B7} 24\u{00B0}")
        .with_font_size(11.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    group_b.add([
        GroupOrShape::from(Rect::new(&gbo)),
        GroupOrShape::from(Rect::new(&gbi)),
        GroupOrShape::from(Circle::new(&gbd)),
        GroupOrShape::from(Text::new(&gbl)),
    ]);
    layer.add([GroupOrShape::from(&group_b)]);

    // Group C + nested inner group.
    let gcc = GroupConfig::default()
        .with_x(495.0)
        .with_y(55.0)
        .with_scale_x(1.15)
        .with_scale_y(1.15);
    let group_c = Group::new(&gcc);
    let gco = RectConfig::default()
        .with_width(130.0)
        .with_height(110.0)
        .with_fill("#ffffff")
        .with_stroke("#111111")
        .with_stroke_width(2.0);
    let gcl = TextConfig::default()
        .with_x(8.0)
        .with_y(8.0)
        .with_text("Group C \u{00B7} 1.15x")
        .with_font_size(10.0)
        .with_font_family("monospace")
        .with_fill("#111111");

    let icc = GroupConfig::default()
        .with_x(25.0)
        .with_y(25.0)
        .with_rotation(15.0);
    let inner_group = Group::new(&icc);
    let mc = CircleConfig::default()
        .with_x(40.0)
        .with_y(30.0)
        .with_radius(16.0)
        .with_fill("#111111");
    let marker = Circle::new(&mc);
    inner_group.add([GroupOrShape::from(&marker)]);
    group_c.add([
        GroupOrShape::from(Rect::new(&gco)),
        GroupOrShape::from(Text::new(&gcl)),
        GroupOrShape::from(inner_group),
    ]);
    layer.add([group_c]);

    let abs = marker.get_absolute_position(None);
    let bbox = group_b.get_client_rect(None);
    let ax = abs.x().round() as i32;
    let ay = abs.y().round() as i32;
    let bw = bbox.width().round() as i32;
    let bh = bbox.height().round() as i32;
    let strip_txt = format!("Marker abs=({ax}, {ay}) \u{00B7} Group B bbox={bw}x{bh}");

    let strip = RectConfig::default()
        .with_x(28.0)
        .with_y(308.0)
        .with_width(624.0)
        .with_height(60.0)
        .with_fill("#f7f7f7")
        .with_stroke("#111111")
        .with_stroke_width(1.5);
    let sum = TextConfig::default()
        .with_x(44.0)
        .with_y(331.0)
        .with_text(strip_txt.as_str())
        .with_font_size(12.0)
        .with_font_family("monospace")
        .with_fill("#111111");
    layer.add([
        GroupOrShape::from(Rect::new(&strip)),
        GroupOrShape::from(Text::new(&sum)),
    ]);

    layer.draw();

    let msg = format!(
        "Stage: 680x400 \u{00B7} Nested marker abs=({ax}, {ay}) \u{00B7} Rotated Group B rect={bw}x{bh}"
    );
    dom::document()
        .get_element_by_id("meta")
        .expect("missing #meta")
        .set_text_content(&msg);
}
