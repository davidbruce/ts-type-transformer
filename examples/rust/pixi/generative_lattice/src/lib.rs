#![no_main]

use pixi_js::*;

fn build_opts() -> ApplicationInitOptions {
    ApplicationInitOptions::default()
        .with_width(680.0)
        .with_height(400.0)
        .with_background_color(16777215.0)
        .with_preference("webgl")
        .with_preserve_drawing_buffer(true)
        .with_antialias(true)
        .with_auto_start(false)
}

fn add<C: JsType, D: JsType>(parent: &Container<C>, child: D) {
    parent.add_child(JsArray::<D>::from([child]));
}

fn stroke_g(g: &Graphics, width: f64, color_hex: u32) {
    let ss = StrokeStyle::default()
        .with_width(width)
        .with_color(color_hex as f64);
    g.stroke(JsArray::from([JsValue::from(ss)]));
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    js::spawn(async {
        let app = Application::<JsValue>::new();
        let opts = build_opts();
        if app.init(Some(&opts)).await.is_err() {
            return;
        }

        let doc = dom::document();
        if let Some(stage_el) = doc.get_element_by_id("stage") {
            stage_el.append_child(app.canvas());
        }

        let stage = app.stage();

        let rings: [f64; 4] = [42.0, 82.0, 122.0, 162.0];
        let ring_colors: [u32; 4] = [0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5];

        let guides = Graphics::new(JsNone);
        for &r in &rings {
            guides.circle(340.0, 200.0, r);
            stroke_g(&guides, 1.0, 0xe5e5e5);
        }

        let unit_x = Point::new(Some(1.0), Some(0.0));
        for step in 0..12 {
            let theta = step as f64 * (2.0 * std::f64::consts::PI / 12.0);
            let spoke = Matrix::new(None, None, None, None, None, None);
            spoke.rotate(theta);
            let dir: Point = spoke.apply(&unit_x, None);
            guides
                .move_to(340.0, 200.0)
                .line_to(340.0 + dir.x() * 182.0, 200.0 + dir.y() * 182.0);
        }
        stroke_g(&guides, 1.0, 0xd4d4d4);
        add(&stage, guides);

        let tiles = Graphics::new(JsNone);
        for (r_idx, &radius) in rings.iter().enumerate() {
            let s = 8.0 + r_idx as f64 * 2.5;
            let corners = [(0.0, -s), (s * 0.72, 0.0), (0.0, s), (-s * 0.72, 0.0)];

            for step in 0..12 {
                let theta = step as f64 * (2.0 * std::f64::consts::PI / 12.0) + r_idx as f64 * 0.13;
                let dir_m = Matrix::new(None, None, None, None, None, None);
                dir_m.rotate(theta);
                let dir: Point = dir_m.apply(&unit_x, None);

                let tile_m = Matrix::new(None, None, None, None, None, None);
                tile_m
                    .rotate(theta + r_idx as f64 * 0.2)
                    .translate(340.0 + radius * dir.x(), 200.0 + radius * dir.y());

                let p0: Point = tile_m.apply(&Point::new(Some(corners[0].0), Some(corners[0].1)), None);
                tiles.move_to(p0.x(), p0.y());
                for &(cx, cy) in &corners[1..] {
                    let pk: Point = tile_m.apply(&Point::new(Some(cx), Some(cy)), None);
                    tiles.line_to(pk.x(), pk.y());
                }
                tiles
                    .close_path()
                    .fill(Some(&JsValue::from(ring_colors[r_idx] as f64)));
                stroke_g(&tiles, 1.2, 0x111111);
            }
        }
        add(&stage, tiles);

        let center_dot = Graphics::new(JsNone);
        center_dot
            .circle(340.0, 200.0, 8.0)
            .fill(Some(&JsValue::from(0x111111 as f64)));
        add(&stage, center_dot);

        app.render();

        if let Some(meta) = doc.get_element_by_id("meta") {
            meta.set_text_content(Some(
                "48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL",
            ));
        }
    });
}
