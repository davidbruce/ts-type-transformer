#include "dom/dom.h"
#include "pixi_js/pixi_js.h"
#include <stdio.h>

static Pixi_Application g_app;

static Pixi_Rectangle mk_rect(double x, double y, double w, double h) {
    return Pixi_Rectangle_new(x, y, w, h);
}

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
    if (js_not_null(stage_el)) {
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
            Pixi_Graphics_fill(g, 0x111111);
        } else if (Pixi_Rectangle_intersects(query, c)) {
            intersecting++;
            Pixi_Graphics_fill(g, 0x737373);
            stroke_g(g, 2.0, (double)0x111111);
        } else {
            disjoint++;
            Pixi_Graphics_fill(g, 0xe5e5e5);
            stroke_g(g, 1.0, (double)0xa3a3a3);
        }
    }

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(hull), Pixi_Rectangle_get_y(hull), Pixi_Rectangle_get_width(hull), Pixi_Rectangle_get_height(hull));
    stroke_g(g, 1.5, (double)0xa3a3a3);

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(padded), Pixi_Rectangle_get_y(padded), Pixi_Rectangle_get_width(padded), Pixi_Rectangle_get_height(padded));
    stroke_g(g, 1.0, (double)0x737373);

    Pixi_Graphics_rect(g, Pixi_Rectangle_get_x(query), Pixi_Rectangle_get_y(query), Pixi_Rectangle_get_width(query), Pixi_Rectangle_get_height(query));
    stroke_g(g, 2.5, (double)0x111111);

    Pixi_Container_addChild(stage, g);
    Pixi_Application_render(g_app);

    char buf[160];
    snprintf(buf, sizeof buf,
        "query 260x170 · %d contained · %d intersecting · %d disjoint · enlarged hull %dx%d",
        contained, intersecting, disjoint,
        (int)(Pixi_Rectangle_get_width(hull) + 0.5), (int)(Pixi_Rectangle_get_height(hull) + 0.5));

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (js_not_null(meta)) HTMLElement_set_textContent(meta, buf);
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
