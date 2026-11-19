#![no_main]

use konva::*;

fn make_line(pts: [f64; 4]) -> GroupOrShape {
    let arr: JsArray<f64> = pts.into_iter().collect();
    let lc = LineConfig::create();
    lc.set_points(LinePoints::Array(arr));
    lc.set_stroke(StringOrCanvasGradient::String("#e5e5e5".to_string()));
    lc.set_stroke_width(Some(1.0));
    let ln = Line::<LineConfig>::new(Some(lc));
    GroupOrShape::from(Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>())
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

    for x in (40..=640).step_by(40) {
        let xf = x as f64;
        layer.add([make_line([xf, 0.0, xf, 400.0])]);
    }
    for y in (40..=360).step_by(40) {
        let yf = y as f64;
        layer.add([make_line([0.0, yf, 680.0, yf])]);
    }

    // Group A.
    let gac = GroupConfig::create();
    gac.set_x(Some(40.0));
    gac.set_y(Some(55.0));
    let group_a = Group::new(Some(&gac));
    let gab = RectConfig::create();
    gab.set_width(Some(160.0));
    gab.set_height(Some(120.0));
    gab.set_fill(StringOrCanvasGradient::String("#f7f7f7".to_string()));
    gab.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    gab.set_stroke_width(Some(1.5));
    let gad = CircleConfig::create();
    gad.set_x(Some(80.0));
    gad.set_y(Some(60.0));
    gad.set_radius(Some(24.0));
    gad.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    let gal = TextConfig::create();
    gal.set_x(Some(12.0));
    gal.set_y(Some(10.0));
    gal.set_text(Some("Group A \u{00B7} 0\u{00B0}"));
    gal.set_font_size(Some(11.0));
    gal.set_font_family(Some("monospace"));
    gal.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    group_a.add([
        GroupOrShape::from(&Rect::new(Some(&gab))),
        GroupOrShape::from(&Circle::new(Some(&gad))),
        GroupOrShape::from(&Text::new(Some(&gal))),
    ]);
    layer.add([GroupOrShape::from(&group_a)]);

    // Group B.
    let gbc = GroupConfig::create();
    gbc.set_x(Some(340.0));
    gbc.set_y(Some(75.0));
    gbc.set_rotation(Some(24.0));
    let group_b = Group::new(Some(&gbc));
    let gbo = RectConfig::create();
    gbo.set_width(Some(150.0));
    gbo.set_height(Some(110.0));
    gbo.set_fill(StringOrCanvasGradient::String("#efefef".to_string()));
    gbo.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    gbo.set_stroke_width(Some(2.0));
    let dash: JsArray<f64> = [4.0, 3.0].into_iter().collect();
    let gbi = RectConfig::create();
    gbi.set_x(Some(22.0));
    gbi.set_y(Some(22.0));
    gbi.set_width(Some(106.0));
    gbi.set_height(Some(66.0));
    gbi.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    gbi.set_stroke(StringOrCanvasGradient::String("#555555".to_string()));
    gbi.set_stroke_width(Some(1.5));
    gbi.set_dash(Some(&dash));
    let gbd = CircleConfig::create();
    gbd.set_x(Some(75.0));
    gbd.set_y(Some(55.0));
    gbd.set_radius(Some(14.0));
    gbd.set_fill(StringOrCanvasGradient::String("#404040".to_string()));
    let gbl = TextConfig::create();
    gbl.set_x(Some(10.0));
    gbl.set_y(Some(8.0));
    gbl.set_text(Some("Group B \u{00B7} 24\u{00B0}"));
    gbl.set_font_size(Some(11.0));
    gbl.set_font_family(Some("monospace"));
    gbl.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    group_b.add([
        GroupOrShape::from(&Rect::new(Some(&gbo))),
        GroupOrShape::from(&Rect::new(Some(&gbi))),
        GroupOrShape::from(&Circle::new(Some(&gbd))),
        GroupOrShape::from(&Text::new(Some(&gbl))),
    ]);
    layer.add([GroupOrShape::from(&group_b)]);

    // Group C + nested inner group.
    let gcc = GroupConfig::create();
    gcc.set_x(Some(495.0));
    gcc.set_y(Some(55.0));
    gcc.set_scale_x(Some(1.15));
    gcc.set_scale_y(Some(1.15));
    let group_c = Group::new(Some(&gcc));
    let gco = RectConfig::create();
    gco.set_width(Some(130.0));
    gco.set_height(Some(110.0));
    gco.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    gco.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    gco.set_stroke_width(Some(2.0));
    let gcl = TextConfig::create();
    gcl.set_x(Some(8.0));
    gcl.set_y(Some(8.0));
    gcl.set_text(Some("Group C \u{00B7} 1.15x"));
    gcl.set_font_size(Some(10.0));
    gcl.set_font_family(Some("monospace"));
    gcl.set_fill(StringOrCanvasGradient::String("#111111".to_string()));

    let icc = GroupConfig::create();
    icc.set_x(Some(25.0));
    icc.set_y(Some(25.0));
    icc.set_rotation(Some(15.0));
    let inner_group = Group::new(Some(&icc));
    let mc = CircleConfig::create();
    mc.set_x(Some(40.0));
    mc.set_y(Some(30.0));
    mc.set_radius(Some(16.0));
    mc.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    let marker = Circle::new(Some(&mc));
    inner_group.add([GroupOrShape::from(&marker)]);
    group_c.add([
        GroupOrShape::from(&Rect::new(Some(&gco))),
        GroupOrShape::from(&Text::new(Some(&gcl))),
        GroupOrShape::from(&inner_group),
    ]);
    layer.add([GroupOrShape::from(&group_c)]);

    let abs = marker.get_absolute_position(None);
    let bbox = group_b.get_client_rect(None);
    let ax = abs.x().round() as i32;
    let ay = abs.y().round() as i32;
    let bw = bbox.width().round() as i32;
    let bh = bbox.height().round() as i32;
    let strip_txt = format!("Marker abs=({ax}, {ay}) \u{00B7} Group B bbox={bw}x{bh}");

    let strip = RectConfig::create();
    strip.set_x(Some(28.0));
    strip.set_y(Some(308.0));
    strip.set_width(Some(624.0));
    strip.set_height(Some(60.0));
    strip.set_fill(StringOrCanvasGradient::String("#f7f7f7".to_string()));
    strip.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
    strip.set_stroke_width(Some(1.5));
    let sum = TextConfig::create();
    sum.set_x(Some(44.0));
    sum.set_y(Some(331.0));
    sum.set_text(Some(&strip_txt));
    sum.set_font_size(Some(12.0));
    sum.set_font_family(Some("monospace"));
    sum.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    layer.add([
        GroupOrShape::from(&Rect::new(Some(&strip))),
        GroupOrShape::from(&Text::new(Some(&sum))),
    ]);

    layer.draw();

    let msg = format!(
        "Stage: 680x400 \u{00B7} Nested marker abs=({ax}, {ay}) \u{00B7} Rotated Group B rect={bw}x{bh}"
    );
    dom::document().get_element_by_id("meta").unwrap().set_text_content(Some(&msg));
}
