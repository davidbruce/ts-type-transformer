#include "dom/dom.h"
#include "pixi_js/pixi_js.h"
#include <stdio.h>

static Pixi_Application g_app;

static Pixi_Rectangle mk_rect(double x, double y, double w, double h) {
    return Pixi_Rectangle_new_with_number_and_number_and_number_and_number(x, y, w, h);
}

static void stroke_g(Pixi_Graphics g, double width, double color_hex) {
    Pixi_StrokeStyle ss = Pixi_StrokeStyle_create();
    Pixi_StrokeStyle_set_width(ss, width);
    Pixi_StrokeStyle_set_color(ss, js_number(color_hex));
    JsArray args = JsArray_new();
    js_push_val(args, ss);
    Pixi_Graphics_stroke(g, args);
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    (void)v;
    if (!ok) return;
    js_scope_open();

    Document doc = get_document();
    HTMLElement stage_el = Document_getElementById(doc, "stage");
    if (stage_el.h != 0) {
        HTMLElement_appendChild(stage_el, Pixi_Application_get_canvas(g_app));
    }

    Pixi_Container stage = Pixi_Application_get_stage(g_app);
    Pixi_Rectangle query = mk_rect(190.0, 105.0, 260.0, 170.0);
    Pixi_Rectangle padded = Pixi_Rectangle_pad(Pixi_Rectangle_clone(query), 10.0, 10.0);
    Pixi_Rectangle hull = Pixi_Rectangle_clone(query);

    Pixi_Rectangle cands[8] = {
        mk_rect(210.0, 125.0, 90.0, 60.0),
        mk_rect(330.0, 165.0, 95.0, 80.0),
        mk_rect(110.0, 80.0, 130.0, 85.0),
        mk_rect(390.0, 75.0, 130.0, 90.0),
        mk_rect(135.0, 225.0, 110.0, 90.0),
        mk_rect(40.0, 40.0, 100.0, 110.0),
        mk_rect(510.0, 48.0, 125.0, 115.0),
        mk_rect(505.0, 245.0, 130.0, 110.0),
    };

    int contained = 0;
    int intersecting = 0;
    int disjoint = 0;

    Pixi_Graphics g = Pixi_Graphics_new();
    for (int i = 0; i < 8; i++) {
        Pixi_Rectangle c = cands[i];
        Pixi_Rectangle_enlarge(hull, c);
        Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(c), Pixi_Rectangle_get_y(c), Pixi_Rectangle_get_width(c), Pixi_Rectangle_get_height(c));
        if (Pixi_Rectangle_containsRect(query, c)) {
            contained++;
            Pixi_Graphics_fill(g, js_number((double)0x111111));
        } else if (Pixi_Rectangle_intersects(query, c)) {
            intersecting++;
            Pixi_Graphics_fill(g, js_number((double)0x737373));
            stroke_g(g, 2.0, (double)0x111111);
        } else {
            disjoint++;
            Pixi_Graphics_fill(g, js_number((double)0xe5e5e5));
            stroke_g(g, 1.0, (double)0xa3a3a3);
        }
    }

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(hull), Pixi_Rectangle_get_y(hull), Pixi_Rectangle_get_width(hull), Pixi_Rectangle_get_height(hull));
    stroke_g(g, 1.5, (double)0xa3a3a3);

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(padded), Pixi_Rectangle_get_y(padded), Pixi_Rectangle_get_width(padded), Pixi_Rectangle_get_height(padded));
    stroke_g(g, 1.0, (double)0x737373);

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(query), Pixi_Rectangle_get_y(query), Pixi_Rectangle_get_width(query), Pixi_Rectangle_get_height(query));
    stroke_g(g, 2.5, (double)0x111111);

    Pixi_Container_addChild(stage, JsArray_Pixi_Container_from((Pixi_Container[]){ AS_Pixi_Container(g) }, 1));
    Pixi_Application_render(g_app);

    char buf[160];
    snprintf(buf, sizeof buf,
        "query 260x170 · %d contained · %d intersecting · %d disjoint · enlarged hull %dx%d",
        contained, intersecting, disjoint,
        (int)(Pixi_Rectangle_get_width(hull) + 0.5), (int)(Pixi_Rectangle_get_height(hull) + 0.5));

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) HTMLElement_set_textContent(meta, buf);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_app = Pixi_Application_new();
    js_retain(g_app);
    Pixi_Application_init_options opts = Pixi_Application_init_options_create();
    Pixi_Application_init_options_set_width(opts, 680.0);
    Pixi_Application_init_options_set_height(opts, 400.0);
    Pixi_Application_init_options_set_backgroundColor(opts, js_number((double)0xffffff));
    Pixi_Application_init_options_set_preference(opts, Pixi_string_or_RendererPreference_array_of("webgl"));
    Pixi_Application_init_options_set_preserveDrawingBuffer(opts, true);
    Pixi_Application_init_options_set_antialias(opts, true);
    Pixi_Application_init_options_set_autoStart(opts, false);
    Promise_then(Pixi_Application_init(g_app, opts), on_init, 0);
    js_scope_close();
}
