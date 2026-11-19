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

        let guide = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
        guide.move_to(40.0, 200.0).line_to(640.0, 200.0).move_to(340.0, 24.0).line_to(340.0, 376.0);
        stroke_g(&guide, 1.0, 0xe5e5e5);
        guide.circle(340.0, 200.0, 110.0);
        stroke_g(&guide, 1.0, 0xd4d4d4);
        add(&stage, guide);

        let root = Container::<JsValue>::new(None);
        root.position().set(Some(340.0), Some(200.0));

        for i in 0..3 {
            let arm = Container::<JsValue>::new(None);
            arm.set_rotation(i as f64 * 2.0 * std::f64::consts::PI / 3.0);

            let arm_bar = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
            arm_bar
                .rect(0.0, -4.0, 110.0, 8.0)
                .fill(Some(&JsValue::from(0x737373 as f64)));
            arm_bar
                .circle(110.0, 0.0, 10.0)
                .fill(Some(&JsValue::from(0x111111 as f64)));
            add(&arm, arm_bar);

            let sub = Container::<JsValue>::new(None);
            sub.set_x(110.0);
            sub.set_rotation(-0.45);

            let sub_bar = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
            sub_bar.move_to(-32.0, 0.0).line_to(48.0, 0.0);
            stroke_g(&sub_bar, 2.0, 0x404040);
            add(&sub, sub_bar);

            let sat_box = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
            sat_box
                .rect(-12.0, -12.0, 24.0, 24.0)
                .fill(Some(&JsValue::from(0x262626 as f64)));
            sat_box.set_x(48.0);
            add(&sub, sat_box);

            let sat_orb = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
            sat_orb
                .circle(0.0, 0.0, 14.0)
                .fill(Some(&JsValue::from(0xd4d4d4 as f64)));
            stroke_g(&sat_orb, 2.0, 0x111111);
            sat_orb.set_x(-32.0);
            add(&sub, sat_orb);

            add(&arm, sub);
            add(&root, arm);
        }

        let hub = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
        hub.circle(0.0, 0.0, 18.0)
            .fill(Some(&JsValue::from(0x111111 as f64)));
        hub.circle(0.0, 0.0, 7.0)
            .fill(Some(&JsValue::from(0xffffff as f64)));
        add(&root, hub);

        add(&stage, root.clone());
        app.render();

        let rb = root.get_bounds(None, None);
        if let Some(meta) = doc.get_element_by_id("meta") {
            let text = format!(
                "3 primary arms · 6 nested satellites · bounds {:.1}x{:.1} · 680x400 WebGL",
                rb.width(),
                rb.height()
            );
            meta.set_text_content(Some(&text));
        }
    });
}
