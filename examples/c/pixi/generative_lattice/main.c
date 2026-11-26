#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

static const double TWO_PI = 6.283185307179586;

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

    static const double rings[4] = { 42.0, 82.0, 122.0, 162.0 };
    static const int ring_colors[4] = { 0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5 };

    Pixi_Graphics guides = Pixi_Graphics_new();
    for (int r = 0; r < 4; r++) {
        Pixi_Graphics_circle(guides, 340.0, 200.0, rings[r]);
        stroke_g(guides, 1.0, (double)0xe5e5e5);
    }

    Pixi_Point unit_x = Pixi_Point_new(1.0, 0.0);
    for (int step = 0; step < 12; step++) {
        double theta = (double)step * (TWO_PI / 12.0);
        Pixi_Matrix spoke = Pixi_Matrix_rotate(Pixi_Matrix_new(), theta);
        Pixi_PointData dir = Pixi_Matrix_apply(spoke, (Pixi_PointData){ unit_x.h });
        Pixi_Graphics_moveTo(guides, 340.0, 200.0);
        Pixi_Graphics_lineTo(guides, 340.0 + Pixi_PointData_get_x(dir) * 182.0, 200.0 + Pixi_PointData_get_y(dir) * 182.0);
    }
    stroke_g(guides, 1.0, (double)0xd4d4d4);

    Pixi_Graphics tiles = Pixi_Graphics_new();
    for (int r_idx = 0; r_idx < 4; r_idx++) {
        double radius = rings[r_idx];
        double s = 8.0 + (double)r_idx * 2.5;
        double cx[4] = { 0.0, s * 0.72, 0.0, -s * 0.72 };
        double cy[4] = { -s, 0.0, s, 0.0 };

        for (int step = 0; step < 12; step++) {
            double theta = (double)step * (TWO_PI / 12.0) + (double)r_idx * 0.13;
            Pixi_Matrix dir_m = Pixi_Matrix_rotate(Pixi_Matrix_new(), theta);
            Pixi_PointData dir = Pixi_Matrix_apply(dir_m, (Pixi_PointData){ unit_x.h });

            Pixi_Matrix tile_m = Pixi_Matrix_new();
            Pixi_Matrix_rotate(tile_m, theta + (double)r_idx * 0.2);
            Pixi_Matrix_translate(tile_m, 340.0 + radius * Pixi_PointData_get_x(dir), 200.0 + radius * Pixi_PointData_get_y(dir));

            Pixi_Point p0 = Pixi_Point_new(cx[0], cy[0]);
            Pixi_PointData tp0 = Pixi_Matrix_apply(tile_m, (Pixi_PointData){ p0.h });
            Pixi_Graphics_moveTo(tiles, Pixi_PointData_get_x(tp0), Pixi_PointData_get_y(tp0));
            for (int k = 1; k < 4; k++) {
                Pixi_Point pk = Pixi_Point_new(cx[k], cy[k]);
                Pixi_PointData tpk = Pixi_Matrix_apply(tile_m, (Pixi_PointData){ pk.h });
                Pixi_Graphics_lineTo(tiles, Pixi_PointData_get_x(tpk), Pixi_PointData_get_y(tpk));
            }
            Pixi_Graphics_closePath(tiles);
            Pixi_Graphics_fill(tiles, ring_colors[r_idx]);
            stroke_g(tiles, 1.2, (double)0x111111);
        }
    }

    Pixi_Graphics center_dot = Pixi_Graphics_new();
    Pixi_Graphics_circle(center_dot, 340.0, 200.0, 8.0);
    Pixi_Graphics_fill(center_dot, 0x111111);
    Pixi_Container_addChild(stage, guides, tiles, center_dot);

    Pixi_Application_render(g_app);

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        HTMLElement_set_textContent(meta, "48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL");
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
