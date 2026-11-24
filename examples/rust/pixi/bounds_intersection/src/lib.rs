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
    g.stroke(&ss);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    js::spawn(async {
        let app = Application::<JsValue>::new();
        let opts = build_opts();
        if app.init(&opts).await.is_err() {
            return;
        }

        let doc = dom::document();
        if let Some(stage_el) = doc.get_element_by_id("stage") {
            stage_el.append_child(app.canvas());
        }

        let stage = app.stage();

        let query = Rectangle::new(190.0, 105.0, 260.0, 170.0);
        let padded = query.js_clone().pad(10.0, 10.0);
        let hull = query.js_clone();

        let candidates = [
            Rectangle::new(210.0, 125.0, 90.0, 60.0),
            Rectangle::new(330.0, 165.0, 95.0, 80.0),
            Rectangle::new(110.0, 80.0, 130.0, 85.0),
            Rectangle::new(390.0, 75.0, 130.0, 90.0),
            Rectangle::new(135.0, 225.0, 110.0, 90.0),
            Rectangle::new(40.0, 40.0, 100.0, 110.0),
            Rectangle::new(510.0, 48.0, 125.0, 115.0),
            Rectangle::new(505.0, 245.0, 130.0, 110.0),
        ];

        let mut contained = 0usize;
        let mut intersecting = 0usize;
        let mut disjoint = 0usize;

        let g = Graphics::new(JsNone);
        for c in &candidates {
            hull.enlarge(c);
            g.rect(c.x(), c.y(), c.width(), c.height());
            if query.contains_rect(c) {
                contained += 1;
                g.fill(0x111111 as f64);
            } else if query.intersects(c, None) {
                intersecting += 1;
                g.fill(0x737373 as f64);
                stroke_g(&g, 2.0, 0x111111);
            } else {
                disjoint += 1;
                g.fill(0xe5e5e5 as f64);
                stroke_g(&g, 1.0, 0xa3a3a3);
            }
        }

        g.rect(hull.x(), hull.y(), hull.width(), hull.height());
        stroke_g(&g, 1.5, 0xa3a3a3);
        g.rect(padded.x(), padded.y(), padded.width(), padded.height());
        stroke_g(&g, 1.0, 0x737373);
        g.rect(query.x(), query.y(), query.width(), query.height());
        stroke_g(&g, 2.5, 0x111111);

        add(&stage, g);
        app.render();

        if let Some(meta) = doc.get_element_by_id("meta") {
            let text = format!(
                "query 260x170 · {} contained · {} intersecting · {} disjoint · enlarged hull {:.0}x{:.0}",
                contained,
                intersecting,
                disjoint,
                hull.width(),
                hull.height()
            );
            meta.set_text_content(&text);
        }
    });
}
