#![no_main]

use core::f64::consts::PI;
use konva::*;

fn make_line(pts: [f64; 4], color: &str, width: f64) -> GroupOrShape {
    let arr: JsArray<f64> = pts.into_iter().collect();
    let lc = LineConfig::create();
    lc.set_points(LinePoints::Array(arr));
    lc.set_stroke(StringOrCanvasGradient::String(color.to_string()));
    lc.set_stroke_width(Some(width));
    let ln = Line::<LineConfig>::new(Some(lc));
    GroupOrShape::from(Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>())
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let cx = 340.0;
    let cy = 200.0;

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

    for (r, col, fill, sw) in [
        (160.0, "#d4d4d4", None, 1.0),
        (125.0, "#a3a3a3", None, 1.0),
        (90.0, "#555555", None, 1.0),
        (52.0, "#111111", Some("#f7f7f7"), 1.5),
    ] {
        let cc = CircleConfig::create();
        cc.set_x(Some(cx));
        cc.set_y(Some(cy));
        cc.set_radius(Some(r));
        cc.set_stroke(StringOrCanvasGradient::String(col.to_string()));
        cc.set_stroke_width(Some(sw));
        if let Some(f) = fill {
            cc.set_fill(StringOrCanvasGradient::String(f.to_string()));
        }
        layer.add([GroupOrShape::from(&Circle::new(Some(&cc)))]);
    }

    for i in 0..12 {
        let a = (i as f64) * PI / 6.0;
        layer.add([make_line(
            [cx + 36.0 * a.cos(), cy + 36.0 * a.sin(), cx + 160.0 * a.cos(), cy + 160.0 * a.sin()],
            "#d4d4d4",
            1.0,
        )]);
    }

    for i in 0..24 {
        let a = (i as f64) * PI / 12.0;
        let r0 = if i % 2 == 0 { 148.0 } else { 154.0 };
        layer.add([make_line(
            [cx + r0 * a.cos(), cy + r0 * a.sin(), cx + 166.0 * a.cos(), cy + 166.0 * a.sin()],
            "#111111",
            if i % 2 == 0 { 2.0 } else { 1.0 },
        )]);
    }

    for i in 0..8 {
        let pc = RectConfig::create();
        pc.set_x(Some(cx));
        pc.set_y(Some(cy));
        pc.set_width(Some(18.0));
        pc.set_height(Some(52.0));
        pc.set_offset_x(Some(9.0));
        pc.set_offset_y(Some(108.0));
        pc.set_rotation(Some((i as f64) * 45.0));
        pc.set_fill(StringOrCanvasGradient::String(if i % 2 == 0 { "#111111" } else { "#efefef" }.to_string()));
        pc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
        pc.set_stroke_width(Some(1.5));
        layer.add([GroupOrShape::from(&Rect::new(Some(&pc)))]);
    }

    for i in 0..12 {
        let a = (i as f64) * PI / 6.0;
        let oc = CircleConfig::create();
        oc.set_x(Some(cx + 125.0 * a.cos()));
        oc.set_y(Some(cy + 125.0 * a.sin()));
        oc.set_radius(Some(5.0));
        oc.set_fill(StringOrCanvasGradient::String(if i % 2 == 0 { "#111111" } else { "#ffffff" }.to_string()));
        oc.set_stroke(StringOrCanvasGradient::String("#111111".to_string()));
        oc.set_stroke_width(Some(1.5));
        layer.add([GroupOrShape::from(&Circle::new(Some(&oc)))]);
    }

    let cp = RegularPolygonConfig::create();
    cp.set_x(Some(cx));
    cp.set_y(Some(cy));
    cp.set_sides(8.0);
    cp.set_radius(26.0);
    cp.set_fill(StringOrCanvasGradient::String("#111111".to_string()));
    let cd = CircleConfig::create();
    cd.set_x(Some(cx));
    cd.set_y(Some(cy));
    cd.set_radius(Some(9.0));
    cd.set_fill(StringOrCanvasGradient::String("#ffffff".to_string()));
    layer.add([
        GroupOrShape::from(&RegularPolygon::new(Some(&cp))),
        GroupOrShape::from(&Circle::new(Some(&cd))),
    ]);

    for (x, y, label) in [
        (516.0, 194.0, "0\u{00B0}"),
        (332.0, 376.0, "90\u{00B0}"),
        (140.0, 194.0, "180\u{00B0}"),
        (328.0, 14.0, "270\u{00B0}"),
    ] {
        let tc = TextConfig::create();
        tc.set_x(Some(x));
        tc.set_y(Some(y));
        tc.set_text(Some(label));
        tc.set_font_size(Some(11.0));
        tc.set_font_family(Some("monospace"));
        tc.set_fill(StringOrCanvasGradient::String("#555555".to_string()));
        layer.add([GroupOrShape::from(&Text::new(Some(&tc)))]);
    }

    layer.draw();

    let count = layer.get_children(None).len();
    let msg = format!("Stage: 680x400 \u{00B7} Procedural rosette \u{00B7} {count} shapes on layer");
    dom::document().get_element_by_id("meta").unwrap().set_text_content(Some(&msg));
}
