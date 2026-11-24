#![no_main]

use pixi_js::*;

fn init_app() -> Application<JsValue> {
    Application::<JsValue>::new()
}

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
    g.stroke_with(&ss);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    js::spawn(async {
        let app = init_app();
        let opts = build_opts();
        if app.init_with(&opts).await.is_err() {
            return;
        }

        let doc = dom::document();
        if let Some(stage_el) = doc.get_element_by_id("stage") {
            stage_el.append_child(app.canvas());
        }

        let stage = app.stage();

        let grid = Graphics::new_with(JsNone);
        for i in 1..=16 {
            let x = (i * 40) as f64;
            grid.move_to(x, 0.0).line_to(x, 400.0);
        }
        for j in 1..=9 {
            let y = (j * 40) as f64;
            grid.move_to(0.0, y).line_to(680.0, y);
        }
        stroke_g(&grid, 1.0, 0xe5e5e5);
        add(&stage, grid);

        let rect_g = Graphics::new_with(JsNone);
        rect_g
            .rect(40.0, 46.0, 140.0, 88.0)
            .fill_with(0x111111 as f64);
        add(&stage, rect_g);

        let round_g = Graphics::new_with(JsNone);
        round_g
            .round_rect_with(216.0, 46.0, 156.0, 88.0, 16.0)
            .fill_with(0xf7f7f7 as f64);
        stroke_g(&round_g, 2.0, 0x111111);
        add(&stage, round_g);

        let circle_g = Graphics::new_with(JsNone);
        circle_g
            .circle(458.0, 90.0, 44.0)
            .fill_with(0x404040 as f64);
        circle_g.circle(458.0, 90.0, 22.0);
        stroke_g(&circle_g, 2.0, 0xffffff);
        circle_g.circle(458.0, 90.0, 52.0);
        stroke_g(&circle_g, 1.5, 0x111111);
        add(&stage, circle_g);

        let ellipse_g = Graphics::new_with(JsNone);
        ellipse_g
            .ellipse(588.0, 90.0, 56.0, 36.0)
            .fill_with(0xd4d4d4 as f64);
        stroke_g(&ellipse_g, 1.5, 0x111111);
        add(&stage, ellipse_g);

        let poly_g = Graphics::new_with(JsNone);
        poly_g
            .regular_poly_with(130.0, 274.0, 58.0, 6.0, 0.25, None)
            .fill_with(0x737373 as f64);
        stroke_g(&poly_g, 2.0, 0x111111);
        add(&stage, poly_g);

        let star_g = Graphics::new_with(JsNone);
        star_g
            .star_with(340.0, 274.0, 5.0, 62.0, 26.0, -0.314)
            .fill_with(0x111111 as f64);
        add(&stage, star_g);

        let diamond_g = Graphics::new_with(JsNone);
        diamond_g
            .move_to(548.0, 212.0)
            .line_to(618.0, 274.0)
            .line_to(548.0, 336.0)
            .line_to(478.0, 274.0)
            .close_path()
            .fill_with(0xb5b5b5 as f64);
        stroke_g(&diamond_g, 1.5, 0x111111);
        add(&stage, diamond_g);

        app.render();

        if let Some(meta) = doc.get_element_by_id("meta") {
            meta.set_text_content(
                "7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL",
            );
        }
    });
}
