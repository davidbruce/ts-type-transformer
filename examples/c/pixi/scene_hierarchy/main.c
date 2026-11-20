#include "dom/dom.h"
#include "pixi_js/pixi_js.h"
#include <stdio.h>

static const double TWO_PI_3 = 2.0943951023931953;

static Application g_app;

static void stroke_g(Graphics g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle_create();
    StrokeStyle_set_width(ss, width);
    StrokeStyle_set_color(ss, js_number(color_hex));
    JsArray args = JsArray_new();
    js_push_val(args, ss);
    Graphics_stroke(g, args);
}

static void add_child_c(Container parent, Container child) {
    Container_addChild(parent, JsArray_Container_from(&child, 1));
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    (void)v;
    if (!ok) return;
    js_scope_open();

    Document doc = get_document();
    HTMLElement stage_el = Document_getElementById(doc, "stage");
    if (stage_el.h != 0) {
        HTMLElement_appendChild(stage_el, Application_get_canvas(g_app));
    }

    Container stage = Application_get_stage(g_app);

    Graphics guide = Graphics_new();
    Graphics_moveTo(guide, 40.0, 200.0);
    Graphics_lineTo(guide, 640.0, 200.0);
    Graphics_moveTo(guide, 340.0, 24.0);
    Graphics_lineTo(guide, 340.0, 376.0);
    stroke_g(guide, 1.0, (double)0xe5e5e5);
    Graphics_circle(guide, 340.0, 200.0, 110.0);
    stroke_g(guide, 1.0, (double)0xd4d4d4);
    add_child_c(stage, AS_Container(guide));

    Container root = Container_new();
    Container_set_x(root, 340.0);
    Container_set_y(root, 200.0);

    for (int i = 0; i < 3; i++) {
        Container arm = Container_new();
        Container_set_rotation(arm, (double)i * TWO_PI_3);

        Graphics arm_bar = Graphics_new();
        Graphics_rect(arm_bar, 0.0, -4.0, 110.0, 8.0);
        Graphics_fill(arm_bar, js_number((double)0x737373));
        Graphics_circle(arm_bar, 110.0, 0.0, 10.0);
        Graphics_fill(arm_bar, js_number((double)0x111111));
        add_child_c(arm, AS_Container(arm_bar));

        Container sub = Container_new();
        Container_set_x(sub, 110.0);
        Container_set_rotation(sub, -0.45);

        Graphics sub_bar = Graphics_new();
        Graphics_moveTo(sub_bar, -32.0, 0.0);
        Graphics_lineTo(sub_bar, 48.0, 0.0);
        stroke_g(sub_bar, 2.0, (double)0x404040);
        add_child_c(sub, AS_Container(sub_bar));

        Graphics sat_box = Graphics_new();
        Graphics_rect(sat_box, -12.0, -12.0, 24.0, 24.0);
        Graphics_fill(sat_box, js_number((double)0x262626));
        Graphics_set_x(sat_box, 48.0);
        add_child_c(sub, AS_Container(sat_box));

        Graphics sat_orb = Graphics_new();
        Graphics_circle(sat_orb, 0.0, 0.0, 14.0);
        Graphics_fill(sat_orb, js_number((double)0xd4d4d4));
        stroke_g(sat_orb, 2.0, (double)0x111111);
        Graphics_set_x(sat_orb, -32.0);
        add_child_c(sub, AS_Container(sat_orb));

        add_child_c(arm, sub);
        add_child_c(root, arm);
    }

    Graphics hub = Graphics_new();
    Graphics_circle(hub, 0.0, 0.0, 18.0);
    Graphics_fill(hub, js_number((double)0x111111));
    Graphics_circle(hub, 0.0, 0.0, 7.0);
    Graphics_fill(hub, js_number((double)0xffffff));
    add_child_c(root, AS_Container(hub));

    add_child_c(stage, root);
    Application_render(g_app);

    Bounds rb = Container_getBounds(root);
    char buf[160];
    snprintf(buf, sizeof buf, "3 primary arms · 6 nested satellites · bounds %.1fx%.1f · 680x400 WebGL",
        Bounds_get_width(rb), Bounds_get_height(rb));

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_app = Application_new();
    js_retain(g_app);
    Application_init_options opts = Application_init_options_create();
    Application_init_options_set_width(opts, 680.0);
    Application_init_options_set_height(opts, 400.0);
    Application_init_options_set_backgroundColor(opts, js_number((double)0xffffff));
    Application_init_options_set_preference(opts, string_or_RendererPreference_array_of("webgl"));
    Application_init_options_set_preserveDrawingBuffer(opts, true);
    Application_init_options_set_antialias(opts, true);
    Application_init_options_set_autoStart(opts, false);
    Promise_then(Application_init(g_app, opts), on_init, 0);
    js_scope_close();
}
