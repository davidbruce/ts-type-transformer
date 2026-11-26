#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

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

    Pixi_Graphics grid = Pixi_Graphics_new();
    for (int x = 40; x <= 640; x += 40) {
        Pixi_Graphics_moveTo(grid, (double)x, 0.0);
        Pixi_Graphics_lineTo(grid, (double)x, 400.0);
    }
    for (int y = 40; y <= 360; y += 40) {
        Pixi_Graphics_moveTo(grid, 0.0, (double)y);
        Pixi_Graphics_lineTo(grid, 680.0, (double)y);
    }
    stroke_g(grid, 1.0, (double)0xe5e5e5);

    Pixi_Graphics rect_g = Pixi_Graphics_new();
    Pixi_Graphics_rect(rect_g, 40.0, 46.0, 140.0, 88.0);
    Pixi_Graphics_fill(rect_g, 0x111111);

    Pixi_Graphics round_g = Pixi_Graphics_new();
    Pixi_Graphics_roundRect(round_g, 216.0, 46.0, 156.0, 88.0, 16.0);
    Pixi_Graphics_fill(round_g, 0xf7f7f7);
    stroke_g(round_g, 2.0, (double)0x111111);

    Pixi_Graphics circle_g = Pixi_Graphics_new();
    Pixi_Graphics_circle(circle_g, 458.0, 90.0, 44.0);
    Pixi_Graphics_fill(circle_g, 0x404040);
    Pixi_Graphics_circle(circle_g, 458.0, 90.0, 22.0);
    stroke_g(circle_g, 2.0, (double)0xffffff);
    Pixi_Graphics_circle(circle_g, 458.0, 90.0, 52.0);
    stroke_g(circle_g, 1.5, (double)0x111111);

    Pixi_Graphics ellipse_g = Pixi_Graphics_new();
    Pixi_Graphics_ellipse(ellipse_g, 588.0, 90.0, 56.0, 36.0);
    Pixi_Graphics_fill(ellipse_g, 0xd4d4d4);
    stroke_g(ellipse_g, 1.5, (double)0x111111);

    Pixi_Graphics poly_g = Pixi_Graphics_new();
    Pixi_Graphics_regularPoly(poly_g, 130.0, 274.0, 58.0, 6.0, 0.25);
    Pixi_Graphics_fill(poly_g, 0x737373);
    stroke_g(poly_g, 2.0, (double)0x111111);

    Pixi_Graphics star_g = Pixi_Graphics_new();
    Pixi_Graphics_star(star_g, 340.0, 274.0, 5.0, 62.0, 26.0, -0.314);
    Pixi_Graphics_fill(star_g, 0x111111);

    Pixi_Graphics diamond_g = Pixi_Graphics_new();
    Pixi_Graphics_moveTo(diamond_g, 548.0, 212.0);
    Pixi_Graphics_lineTo(diamond_g, 618.0, 274.0);
    Pixi_Graphics_lineTo(diamond_g, 548.0, 336.0);
    Pixi_Graphics_lineTo(diamond_g, 478.0, 274.0);
    Pixi_Graphics_closePath(diamond_g);
    Pixi_Graphics_fill(diamond_g, 0xb5b5b5);
    stroke_g(diamond_g, 1.5, (double)0x111111);
    Pixi_Container_addChild(stage, grid, rect_g, round_g, circle_g, ellipse_g, poly_g, star_g, diamond_g);

    Pixi_Application_render(g_app);

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        HTMLElement_set_textContent(meta, "7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
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
