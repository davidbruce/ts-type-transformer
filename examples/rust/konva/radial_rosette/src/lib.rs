#![no_main]

use core::f64::consts::PI;
use konva::*;

fn make_line(pts: [f64; 4], color: &str, width: f64) -> GroupOrShape {
    let lc = LineConfig::default()
        .with_points(JsArray::from(pts))
        .with_stroke(color)
        .with_stroke_width(width);
    let ln = Line::<LineConfig>::new(Some(lc));
    Shape::<LineConfig>::from(ln).upcast::<ShapeConfig>().into()
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let cx = 340.0;
    let cy = 200.0;

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

    for (r, col, fill, sw) in [
        (160.0, "#d4d4d4", None, 1.0),
        (125.0, "#a3a3a3", None, 1.0),
        (90.0, "#555555", None, 1.0),
        (52.0, "#111111", Some("#f7f7f7"), 1.5),
    ] {
        let mut cc = CircleConfig::default()
            .with_x(cx)
            .with_y(cy)
            .with_radius(r)
            .with_stroke(col)
            .with_stroke_width(sw);
        if let Some(f) = fill {
            cc = cc.with_fill(f);
        }
        layer.add([Circle::new(Some(&cc))]);
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
        let pc = RectConfig::default()
            .with_x(cx)
            .with_y(cy)
            .with_width(18.0)
            .with_height(52.0)
            .with_offset_x(9.0)
            .with_offset_y(108.0)
            .with_rotation((i as f64) * 45.0)
            .with_fill(if i % 2 == 0 { "#111111" } else { "#efefef" })
            .with_stroke("#111111")
            .with_stroke_width(1.5);
        layer.add([Rect::new(Some(&pc))]);
    }

    for i in 0..12 {
        let a = (i as f64) * PI / 6.0;
        let oc = CircleConfig::default()
            .with_x(cx + 125.0 * a.cos())
            .with_y(cy + 125.0 * a.sin())
            .with_radius(5.0)
            .with_fill(if i % 2 == 0 { "#111111" } else { "#ffffff" })
            .with_stroke("#111111")
            .with_stroke_width(1.5);
        layer.add([Circle::new(Some(&oc))]);
    }

    let cp = RegularPolygonConfig::default()
        .with_x(cx)
        .with_y(cy)
        .with_sides(8.0)
        .with_radius(26.0)
        .with_fill("#111111");
    let cd = CircleConfig::default()
        .with_x(cx)
        .with_y(cy)
        .with_radius(9.0)
        .with_fill("#ffffff");
    layer.add([
        GroupOrShape::from(RegularPolygon::new(Some(&cp))),
        GroupOrShape::from(Circle::new(Some(&cd))),
    ]);

    for (x, y, label) in [
        (516.0, 194.0, "0\u{00B0}"),
        (332.0, 376.0, "90\u{00B0}"),
        (140.0, 194.0, "180\u{00B0}"),
        (328.0, 14.0, "270\u{00B0}"),
    ] {
        let tc = TextConfig::default()
            .with_x(x)
            .with_y(y)
            .with_text(label)
            .with_font_size(11.0)
            .with_font_family("monospace")
            .with_fill("#555555");
        layer.add([Text::new(Some(&tc))]);
    }

    layer.draw();

    let count = layer.get_children(None).len();
    let msg = format!("Stage: 680x400 \u{00B7} Procedural rosette \u{00B7} {count} shapes on layer");
    dom::document()
        .get_element_by_id("meta")
        .expect("missing #meta")
        .set_text_content(Some(&msg));
}
