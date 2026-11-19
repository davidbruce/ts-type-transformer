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

        let frame = Graphics::new(None::<GraphicsOptionsOrGraphicsContext>);
        frame.rect(32.0, 20.0, 616.0, 360.0);
        let ss = StrokeStyle::create();
        ss.set_width(Some(1.5));
        ss.set_color(Some(&JsValue::from(0xd4d4d4 as f64)));
        frame.stroke(JsArray::from(vec![JsValue::from(&ss)]));
        add(&stage, frame);

        let shades: [u32; 8] = [
            0x111111, 0x262626, 0x404040, 0x555555,
            0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5,
        ];
        let white = Texture::<TextureSource<JsValue>>::white().upcast::<TextureSource<JsValue>>();

        for row in 0..6usize {
            for col in 0..10usize {
                let sprite = Sprite::new(SpriteOptionsOrTexture::from(white.clone()));
                let idx = (row * 3 + col * 5) % shades.len();
                sprite.set_width(52.0);
                sprite.set_height(48.0);
                sprite.anchor().set(Some(0.5), Some(0.5));
                sprite
                    .position()
                    .set(Some(70.0 + col as f64 * 60.0), Some(56.0 + row as f64 * 56.0));
                sprite.set_rotation((col as f64 - 4.5) * 0.04 + (row as f64 - 2.5) * 0.03);
                sprite.set_alpha(0.65 + 0.35 * ((col + row) % 3) as f64 / 2.0);
                sprite.set_tint(JsValue::from(shades[idx] as f64));
                add(&stage, sprite);
            }
        }

        app.render();

        if let Some(meta) = doc.get_element_by_id("meta") {
            meta.set_text_content(Some(
                "60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL",
            ));
        }
    });
}
