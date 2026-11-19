#include "dom/dom.h"
#include "pixi_js/pixi_js.h"

#define TWO_PI (6.283185307179586)

static Application g_app;

static void stroke_g(Graphics g, double width, double color_hex) {
    StrokeStyle ss = StrokeStyle_create();
    StrokeAttributes_set_width(StrokeStyle_as_StrokeAttributes(ss), js_some_double(width));
    FillStyle_set_color(StrokeStyle_as_FillStyle(ss), js_number(color_hex));
    JsArray args = JsArray_new();
    JsArray_push(args, (JsValue){ ss.h });
    Graphics_stroke(g, args);
}

static void add_gfx(Container stage, Graphics g) {
    JsArray_Container kids = JsArray_Container_new();
    JsArray_Container_push(kids, Graphics_as_Container(g));
    Container_addChild(stage, kids);
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx;
    (void)v;
    if (!ok) return;

    Document doc = get_document();
    HTMLElement stage_el = Document_getElementById(doc, "stage");
    if (stage_el.h != 0) {
        Node_appendChild(stage_el, Application_get_canvas(g_app));
    }

    Container stage = Application_get_stage(g_app);

    static const double rings[4] = { 42.0, 82.0, 122.0, 162.0 };
    static const int ring_colors[4] = { 0x111111, 0x555555, 0xa3a3a3, 0xe5e5e5 };

    Graphics guides = Graphics_new__0();
    for (int r = 0; r < 4; r++) {
        Graphics_circle(guides, 340.0, 200.0, rings[r]);
        stroke_g(guides, 1.0, (double)0xe5e5e5);
    }

    Point unit_x = Point_new(js_some_double(1.0), js_some_double(0.0));
    for (int step = 0; step < 12; step++) {
        double theta = (double)step * (TWO_PI / 12.0);
        Matrix spoke = Matrix_rotate(Matrix_new__0(), theta);
        PointData dir = Matrix_apply(spoke, (PointData){ unit_x.h }, (PointData){ 0 });
        Graphics_moveTo(guides, 340.0, 200.0);
        Graphics_lineTo(guides, 340.0 + PointData_get_x(dir) * 182.0, 200.0 + PointData_get_y(dir) * 182.0);
    }
    stroke_g(guides, 1.0, (double)0xd4d4d4);
    add_gfx(stage, guides);

    Graphics tiles = Graphics_new__0();
    for (int r_idx = 0; r_idx < 4; r_idx++) {
        double radius = rings[r_idx];
        double s = 8.0 + (double)r_idx * 2.5;
        double cx[4] = { 0.0, s * 0.72, 0.0, -s * 0.72 };
        double cy[4] = { -s, 0.0, s, 0.0 };

        for (int step = 0; step < 12; step++) {
            double theta = (double)step * (TWO_PI / 12.0) + (double)r_idx * 0.13;
            Matrix dir_m = Matrix_rotate(Matrix_new__0(), theta);
            PointData dir = Matrix_apply(dir_m, (PointData){ unit_x.h }, (PointData){ 0 });

            Matrix tile_m = Matrix_new__0();
            Matrix_rotate(tile_m, theta + (double)r_idx * 0.2);
            Matrix_translate(tile_m, 340.0 + radius * PointData_get_x(dir), 200.0 + radius * PointData_get_y(dir));

            Point p0 = Point_new(js_some_double(cx[0]), js_some_double(cy[0]));
            PointData tp0 = Matrix_apply(tile_m, (PointData){ p0.h }, (PointData){ 0 });
            Graphics_moveTo(tiles, PointData_get_x(tp0), PointData_get_y(tp0));
            for (int k = 1; k < 4; k++) {
                Point pk = Point_new(js_some_double(cx[k]), js_some_double(cy[k]));
                PointData tpk = Matrix_apply(tile_m, (PointData){ pk.h }, (PointData){ 0 });
                Graphics_lineTo(tiles, PointData_get_x(tpk), PointData_get_y(tpk));
            }
            Graphics_closePath(tiles);
            Graphics_fill(tiles, js_number((double)ring_colors[r_idx]));
            stroke_g(tiles, 1.2, (double)0x111111);
        }
    }
    add_gfx(stage, tiles);

    Graphics center_dot = Graphics_new__0();
    Graphics_circle(center_dot, 340.0, 200.0, 8.0);
    Graphics_fill(center_dot, js_number((double)0x111111));
    add_gfx(stage, center_dot);

    Application_render(g_app);

    HTMLElement meta = Document_getElementById(doc, "meta");
    if (meta.h != 0) {
        Node_set_textContent(meta, "48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL");
    }
}

__attribute__((export_name("start"))) void start(void) {
    g_app = Application_new();
    Application_init_options opts = Application_init_options_create();
    Application_init_options_set_width(opts, js_some_double(680.0));
    Application_init_options_set_height(opts, js_some_double(400.0));
    Application_init_options_set_backgroundColor(opts, js_number((double)0xffffff));
    Application_init_options_set_preference(opts, string_or_RendererPreference_array_of_string(JsString_from("webgl")));
    Application_init_options_set_preserveDrawingBuffer(opts, js_some_bool(true));
    Application_init_options_set_antialias(opts, js_some_bool(true));
    Application_init_options_set_autoStart(opts, js_some_bool(false));
    Promise p = Application_init(g_app, opts);
    Promise_then(p, on_init, 0);
}
