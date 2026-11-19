#![no_main]

use dom::NodeMethods as _;
use pixi_js::*;

fn build_opts() -> ApplicationInitOptions {
    let opts = ApplicationInitOptions::create();
    opts.set_width(Some(680.0));
    opts.set_height(Some(400.0));
    opts.set_background_color(Some(&JsValue::from(16777215.0)));
    opts.set_preference(StringOrRendererPreferenceArray::String("webgl".into()));
    opts.set_preserve_drawing_buffer(Some(true));
    opts.set_antialias(Some(true));
    opts.set_auto_start(Some(false));
    opts
}

fn add<C: JsType, D: JsType>(parent: &Container<C>, child: D) {
    parent.add_child(JsArray::<D>::from(vec![child]));
}

fn stroke_g(g: &Graphics, width: f64, color_hex: u32) {
    let ss = StrokeStyle::create();
    ss.set_width(Some(width));
    ss.set_color(Some(&JsValue::from(color_hex as f64)));
    g.stroke(JsArray::from(vec![JsValue::from(&ss)]));
}

fn mk_rect(x: f64, y: f64, w: f64, h: f64) -> Rectangle {
    Rectangle::new(
        NumberOrString::Number(x),
        NumberOrString::Number(y),
        NumberOrString::Number(w),
        NumberOrString::Number(h),
    )
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

        let query = mk_rect(190.0, 105.0, 260.0, 170.0);
        let padded = query.js_clone().pad(Some(10.0), Some(10.0));
        let hull = query.js_clone();

        let candidates = [
            mk_rect(210.0, 125.0, 90.0, 60.0),
            mk_rect(330.0, 165.0, 95.0, 80.0),
            mk_rect(110.0, 80.0, 130.0, 85.0),
            mk_rect(390.0, 75.0, 130.0, 90.0),
            mk_rect(135.0, 225.0, 110.0, 90.0),
            mk_rect(40.0, 40.0, 100.0, 110.0),
            mk_rect(510.0, 48.0, 125.0, 115.0),
            mk_rect(505.0, 245.0, 130.0, 110.0),
        ];

        let mut contained = 0usize;
        let mut intersecting = 0usize;
        let mut disjoint = 0usize;

        let g = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
        for c in &candidates {
            hull.enlarge(c);
            g.rect(c.x(), c.y(), c.width(), c.height());
            if query.contains_rect(c) {
                contained += 1;
                g.fill(Some(&JsValue::from(0x111111 as f64)));
            } else if query.intersects(c, None) {
                intersecting += 1;
                g.fill(Some(&JsValue::from(0x737373 as f64)));
                stroke_g(&g, 2.0, 0x111111);
            } else {
                disjoint += 1;
                g.fill(Some(&JsValue::from(0xe5e5e5 as f64)));
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
            meta.set_text_content(Some(&text));
        }
    });
}
