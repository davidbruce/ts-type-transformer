#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

static Application g_app;

static void stroke_g(Graphics g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle_create();
    StrokeStyle_set_width(ss, width);
    StrokeStyle_set_color(ss, js_number(color_hex));
    JsArray args = JsArray_new();
    js_push_val(args, ss);
    Graphics_stroke(g, args);
}

static void add_gfx(Container stage, Graphics g) {
    Container_addChild(stage, JsArray_Container_from((Container[]){ AS_Container(g) }, 1));
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

    Graphics grid = Graphics_new();
    for (int x = 40; x <= 640; x += 40) {
        Graphics_moveTo(grid, (double)x, 0.0);
        Graphics_lineTo(grid, (double)x, 400.0);
    }
    for (int y = 40; y <= 360; y += 40) {
        Graphics_moveTo(grid, 0.0, (double)y);
        Graphics_lineTo(grid, 680.0, (double)y);
    }
    stroke_g(grid, 1.0, (double)0xe5e5e5);
    add_gfx(stage, grid);

    Graphics rect_g = Graphics_new();
    Graphics_rect(rect_g, 40.0, 46.0, 140.0, 88.0);
    Graphics_fill(rect_g, js_number((double)0x111111));
    add_gfx(stage, rect_g);

    Graphics round_g = Graphics_new();
    Graphics_roundRect(round_g, 216.0, 46.0, 156.0, 88.0, 16.0);
    Graphics_fill(round_g, js_number((double)0xf7f7f7));
    stroke_g(round_g, 2.0, (double)0x111111);
    add_gfx(stage, round_g);

    Graphics circle_g = Graphics_new();
    Graphics_circle(circle_g, 458.0, 90.0, 44.0);
    Graphics_fill(circle_g, js_number((double)0x404040));
    Graphics_circle(circle_g, 458.0, 90.0, 22.0);
    stroke_g(circle_g, 2.0, (double)0xffffff);
    Graphics_circle(circle_g, 458.0, 90.0, 52.0);
    stroke_g(circle_g, 1.5, (double)0x111111);
    add_gfx(stage, circle_g);

    Graphics ellipse_g = Graphics_new();
    Graphics_ellipse(ellipse_g, 588.0, 90.0, 56.0, 36.0);
    Graphics_fill(ellipse_g, js_number((double)0xd4d4d4));
    stroke_g(ellipse_g, 1.5, (double)0x111111);
    add_gfx(stage, ellipse_g);

    Graphics poly_g = Graphics_new();
    Graphics_regularPoly(poly_g, 130.0, 274.0, 58.0, 6.0, 0.25);
    Graphics_fill(poly_g, js_number((double)0x737373));
    stroke_g(poly_g, 2.0, (double)0x111111);
    add_gfx(stage, poly_g);

    Graphics star_g = Graphics_new();
    Graphics_star(star_g, 340.0, 274.0, 5.0, 62.0, 26.0, -0.314);
    Graphics_fill(star_g, js_number((double)0x111111));
    add_gfx(stage, star_g);

    Graphics diamond_g = Graphics_new();
    Graphics_moveTo(diamond_g, 548.0, 212.0);
    Graphics_lineTo(diamond_g, 618.0, 274.0);
    Graphics_lineTo(diamond_g, 548.0, 336.0);
    Graphics_lineTo(diamond_g, 478.0, 274.0);
    Graphics_closePath(diamond_g);
    Graphics_fill(diamond_g, js_number((double)0xb5b5b5));
    stroke_g(diamond_g, 1.5, (double)0x111111);
    add_gfx(stage, diamond_g);

    Application_render(g_app);

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        HTMLElement_set_textContent(meta, "7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL");
    }
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
