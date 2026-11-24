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

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    js::spawn(async {
        let app = Application::<JsValue>::new();
        let opts = build_opts();
        if app.init_with(&opts).await.is_err() {
            return;
        }

        let doc = dom::document();
        if let Some(stage_el) = doc.get_element_by_id("stage") {
            stage_el.append_child(app.canvas());
        }

        let stage = app.stage();

        let frame = Graphics::new_with(JsNone);
        frame.rect(32.0, 20.0, 616.0, 360.0);
        let ss = StrokeStyle::default()
            .with_width(1.5)
            .with_color(0xd4d4d4 as f64);
        frame.stroke_with(&ss);
        add(&stage, frame);

        let shades: [u32; 8] = [
            0x111111, 0x262626, 0x404040, 0x555555, 0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5,
        ];
        let white = Texture::<TextureSource<JsValue>>::white().upcast::<TextureSource<JsValue>>();

        for row in 0..6usize {
            for col in 0..10usize {
                let sprite = Sprite::new_with(white.clone());
                let idx = (row * 3 + col * 5) % shades.len();
                sprite.set_width(52.0);
                sprite.set_height(48.0);
                sprite.anchor().set_with(0.5, 0.5);
                sprite
                    .position()
                    .set_with(70.0 + col as f64 * 60.0, 56.0 + row as f64 * 56.0);
                sprite.set_rotation((col as f64 - 4.5) * 0.04 + (row as f64 - 2.5) * 0.03);
                sprite.set_alpha(0.65 + 0.35 * ((col + row) % 3) as f64 / 2.0);
                sprite.set_tint(shades[idx] as f64);
                add(&stage, sprite);
            }
        }

        app.render();

        if let Some(meta) = doc.get_element_by_id("meta") {
            meta.set_text_content(
                "60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL",
            );
        }
    });
}
