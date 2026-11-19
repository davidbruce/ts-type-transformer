#include "pts/pts.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)

static double approx_sin(double x) {
    while (x > PI) x -= TWO_PI;
    while (x < -PI) x += TWO_PI;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0 * (1.0 - x2 / 72.0))));
}
static double approx_cos(double x) { return approx_sin(x + PI * 0.5); }

static Pt mk_pt(double x, double y) {
    Pt p = Pt_make(2.0);
    Pt_set_x(p, x);
    Pt_set_y(p, y);
    return p;
}

static Group mk_group2(Pt a, Pt b) {
    JsArray_Pt arr = JsArray_Pt_new();
    JsArray_Pt_push(arr, a);
    JsArray_Pt_push(arr, b);
    return Group_new(arr);
}

static CanvasSpace g_space;
static CanvasForm g_form;

static void on_animate(void* ctx, double time_ms, double ftime_ms, Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pt c = Space_get_center(g_space);
    Pt size = Space_get_size(g_space);
    Pt pointer = MultiTouchSpace_get_pointer(g_space);
    double sx = Pt_get_x(size), sy = Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    double base_r = min_dim * 0.34;
    if (base_r < 70.0) base_r = 70.0;
    double cx = Pt_get_x(c), cy = Pt_get_y(c);

    Group radial = Create_radialPts(PtLike_of(c), base_r, 20.0, js_some_double(time_ms * 0.00025));
    JsArray_Pt r_arr = Group_as_array(radial);
    uint32_t r_len = JsArray_Pt_length(r_arr);

    JsArray_Pt cloud_arr = JsArray_Pt_new();
    for (uint32_t i = 0; i < r_len; i++) {
        Pt p = JsArray_Pt_get(r_arr, i);
        double fi = (double)i;
        double scale = 0.48 + 0.52 * (0.5 + 0.5 * approx_sin(time_ms * 0.0011 + fi * 0.9) * approx_cos(time_ms * 0.0007 - fi * 0.5));
        double nx = cx + (Pt_get_x(p) - cx) * scale;
        double ny = cy + (Pt_get_y(p) - cy) * scale;
        JsArray_Pt_push(cloud_arr, mk_pt(nx, ny));
    }
    double px = Pt_get_x(pointer), py = Pt_get_y(pointer);
    if (px > 1.0 || py > 1.0) {
        JsArray_Pt_push(cloud_arr, mk_pt(px, py));
    }

    Group cloud = Group_new(cloud_arr);
    Group hull = Polygon_convexHull(PtLikeIterable_of(cloud), js_some_bool(false));
    Group bbox = Geom_boundingBox(PtIterable_of(hull));
    Pt cent = Polygon_centroid(PtLikeIterable_of(hull));

    CanvasForm_rect_2(CanvasForm_strokeOnly(g_form, "#b5b5b5", js_some_double(1.0)), PtLikeIterable_of(bbox));
    CanvasForm_polygon_2(
        CanvasForm_stroke(CanvasForm_fill(g_form, "#f7f7f7"), "#111111", js_some_double(2.0)),
        PtLikeIterable_of(hull)
    );

    JsArray_Group net0 = Polygon_network(PtIterable_of(hull), js_some_double(0.0));
    uint32_t net0_len = JsArray_Group_length(net0);
    for (uint32_t i = 0; i < net0_len; i++) {
        CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#d4d4d4", js_some_double(1.0)), PtLikeIterable_of(JsArray_Group_get(net0, i)));
    }
    JsArray_Pt hull_arr = Group_as_array(hull);
    uint32_t hull_len = JsArray_Pt_length(hull_arr);
    if (hull_len > 3) {
        JsArray_Group net_mid = Polygon_network(PtIterable_of(hull), js_some_double((double)(hull_len / 2)));
        uint32_t nm_len = JsArray_Group_length(net_mid);
        for (uint32_t i = 0; i < nm_len; i++) {
            CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#d4d4d4", js_some_double(1.0)), PtLikeIterable_of(JsArray_Group_get(net_mid, i)));
        }
    }

    uint32_t cl_len = JsArray_Pt_length(cloud_arr);
    for (uint32_t i = 0; i < cl_len; i++) {
        CanvasForm_point_2(CanvasForm_fillOnly(g_form, "#737373"), PtLike_of(JsArray_Pt_get(cloud_arr, i)), js_some_double(3.0), "circle");
    }
    for (uint32_t i = 0; i < hull_len; i++) {
        CanvasForm_point_2(
            CanvasForm_stroke(CanvasForm_fill(g_form, "#111111"), "#ffffff", js_some_double(1.5)),
            PtLike_of(JsArray_Pt_get(hull_arr, i)), js_some_double(5.0), "circle"
        );
    }

    double cent_x = Pt_get_x(cent), cent_y = Pt_get_y(cent);
    CanvasForm_line_2(
        CanvasForm_strokeOnly(g_form, "#111111", js_some_double(1.4)),
        PtLikeIterable_of(mk_group2(mk_pt(cent_x - 10.0, cent_y), mk_pt(cent_x + 10.0, cent_y)))
    );
    CanvasForm_line_2(
        CanvasForm_strokeOnly(g_form, "#111111", js_some_double(1.4)),
        PtLikeIterable_of(mk_group2(mk_pt(cent_x, cent_y - 10.0), mk_pt(cent_x, cent_y + 10.0)))
    );
    CanvasForm_point_2(
        CanvasForm_stroke(CanvasForm_fill(g_form, "#ffffff"), "#111111", js_some_double(1.6)),
        PtLike_of(cent), js_some_double(5.5), "circle"
    );

    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    CanvasSpaceOptions opts = CanvasSpaceOptions_create();
    CanvasSpaceOptions_set_bgcolor(opts, "#ffffff");
    CanvasSpaceOptions_set_resize(opts, js_some_bool(true));
    CanvasSpaceOptions_set_retina(opts, js_some_bool(true));

    CanvasSpace raw_space = CanvasSpace_new("#pt");
    g_space = CanvasSpace_setup(raw_space, opts);
    g_form = CanvasSpace_getForm(g_space);

    IPlayer player = IPlayer_create();
    AnimateCallbackFn anim = AnimateCallbackFn_from(on_animate, 0);
    IPlayer_set_animate(player, anim);
    Space_add(g_space, player);
    MultiTouchSpace_bindMouse(g_space);
    MultiTouchSpace_bindTouch(g_space);
    Space_play(g_space);

    js_release(anim);
    js_release(player);
    js_release(raw_space);
    js_release(opts);
}
