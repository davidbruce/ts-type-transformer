#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

static Pixi_Application g_app;

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    (void)v;
    if (!ok) return;
    js_scope();

    Document doc = get_document();
    HTMLElement stage_el = Document_getElementById(doc, "stage");
    if (stage_el.h != 0) {
        HTMLElement_appendChild(stage_el, Pixi_Application_get_canvas(g_app));
    }

    Pixi_Container stage = Pixi_Application_get_stage(g_app);

    Pixi_Graphics frame = Pixi_Graphics_new();
    Pixi_Graphics_rect(frame, 32.0, 20.0, 616.0, 360.0);
    Pixi_Graphics_stroke(frame, Pixi_StrokeStyle_create(.width = 1.5, .color = 0xd4d4d4));
    Pixi_Container_addChild(stage, frame);

    static const int shades[8] = {
        0x111111, 0x262626, 0x404040, 0x555555,
        0x737373, 0xa3a3a3, 0xd4d4d4, 0xe5e5e5
    };
    Pixi_Texture white = Pixi_Texture_get_WHITE();

    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 10; col++) {
            Pixi_Sprite s = Pixi_Sprite_new(white);
            int idx = (row * 3 + col * 5) % 8;
            Pixi_Sprite_set_width(s, 52.0);
            Pixi_Sprite_set_height(s, 48.0);
            Pixi_ObservablePoint_set(Pixi_Sprite_get_anchor(s), 0.5, 0.5);
            Pixi_Sprite_set_x(s, 70.0 + (double)col * 60.0);
            Pixi_Sprite_set_y(s, 56.0 + (double)row * 56.0);
            Pixi_Sprite_set_rotation(s, ((double)col - 4.5) * 0.04 + ((double)row - 2.5) * 0.03);
            Pixi_Sprite_set_alpha(s, 0.65 + 0.35 * (double)((col + row) % 3) / 2.0);
            Pixi_Sprite_set_tint(s, shades[idx]);
            Pixi_Container_addChild(stage, s);
        }
    }

    Pixi_Application_render(g_app);

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        HTMLElement_set_textContent(meta, "60 Texture.WHITE sprites · 10x6 mosaic · 8 monochrome tints · 680x400 WebGL");
    }
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_app = Pixi_Application_new();
    js_retain(g_app);
    Pixi_Application_init_options opts = Pixi_Application_init_options_create(
        .width = 680.0, .height = 400.0, .backgroundColor = 0xffffff,
        .preference = "webgl",
        .preserveDrawingBuffer = true, .antialias = true, .autoStart = false);
    Promise_then(Pixi_Application_init(g_app, opts), on_init, 0);
}
