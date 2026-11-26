#include "dom/dom.h"
#include "pixi_js/pixi_js.h"
#include <stdio.h>

static const double TWO_PI_3 = 2.0943951023931953;

static Pixi_Application g_app;

static void stroke_g(Pixi_Graphics g, double width, double color_hex) {
    Pixi_Graphics_stroke(g, Pixi_StrokeStyle_create(.width = width, .color = color_hex));
}

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

    Pixi_Graphics guide = Pixi_Graphics_new();
    Pixi_Graphics_moveTo(guide, 40.0, 200.0);
    Pixi_Graphics_lineTo(guide, 640.0, 200.0);
    Pixi_Graphics_moveTo(guide, 340.0, 24.0);
    Pixi_Graphics_lineTo(guide, 340.0, 376.0);
    stroke_g(guide, 1.0, (double)0xe5e5e5);
    Pixi_Graphics_circle(guide, 340.0, 200.0, 110.0);
    stroke_g(guide, 1.0, (double)0xd4d4d4);

    Pixi_Container root = Pixi_Container_new();
    Pixi_Container_set_x(root, 340.0);
    Pixi_Container_set_y(root, 200.0);

    for (int i = 0; i < 3; i++) {
        Pixi_Container arm = Pixi_Container_new();
        Pixi_Container_set_rotation(arm, (double)i * TWO_PI_3);

        Pixi_Graphics arm_bar = Pixi_Graphics_new();
        Pixi_Graphics_rect(arm_bar, 0.0, -4.0, 110.0, 8.0);
        Pixi_Graphics_fill(arm_bar, 0x737373);
        Pixi_Graphics_circle(arm_bar, 110.0, 0.0, 10.0);
        Pixi_Graphics_fill(arm_bar, 0x111111);

        Pixi_Container sub = Pixi_Container_new();
        Pixi_Container_set_x(sub, 110.0);
        Pixi_Container_set_rotation(sub, -0.45);

        Pixi_Graphics sub_bar = Pixi_Graphics_new();
        Pixi_Graphics_moveTo(sub_bar, -32.0, 0.0);
        Pixi_Graphics_lineTo(sub_bar, 48.0, 0.0);
        stroke_g(sub_bar, 2.0, (double)0x404040);

        Pixi_Graphics sat_box = Pixi_Graphics_new();
        Pixi_Graphics_rect(sat_box, -12.0, -12.0, 24.0, 24.0);
        Pixi_Graphics_fill(sat_box, 0x262626);
        Pixi_Graphics_set_x(sat_box, 48.0);

        Pixi_Graphics sat_orb = Pixi_Graphics_new();
        Pixi_Graphics_circle(sat_orb, 0.0, 0.0, 14.0);
        Pixi_Graphics_fill(sat_orb, 0xd4d4d4);
        stroke_g(sat_orb, 2.0, (double)0x111111);
        Pixi_Graphics_set_x(sat_orb, -32.0);
        Pixi_Container_addChild(sub, sub_bar, sat_box, sat_orb);

        Pixi_Container_addChild(arm, arm_bar, sub);
        Pixi_Container_addChild(root, arm);
    }

    Pixi_Graphics hub = Pixi_Graphics_new();
    Pixi_Graphics_circle(hub, 0.0, 0.0, 18.0);
    Pixi_Graphics_fill(hub, 0x111111);
    Pixi_Graphics_circle(hub, 0.0, 0.0, 7.0);
    Pixi_Graphics_fill(hub, 0xffffff);
    Pixi_Container_addChild(root, hub);

    Pixi_Container_addChild(stage, guide, root);
    Pixi_Application_render(g_app);

    Pixi_Bounds rb = Pixi_Container_getBounds(root);
    char buf[160];
    snprintf(buf, sizeof buf, "3 primary arms · 6 nested satellites · bounds %.1fx%.1f · 680x400 WebGL",
        Pixi_Bounds_get_width(rb), Pixi_Bounds_get_height(rb));

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
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
