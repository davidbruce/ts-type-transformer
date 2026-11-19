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
static double abs_d(double v) { return v < 0.0 ? -v : v; }

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

    Pt size = Space_get_size(g_space);
    Pt c = Space_get_center(g_space);
    Pt pointer = MultiTouchSpace_get_pointer(g_space);
    double w = Pt_get_x(size); if (w < 120.0) w = 120.0;
    double h = Pt_get_y(size); if (h < 120.0) h = 120.0;
    double cy = Pt_get_y(c);
    double px = Pt_get_x(pointer), py = Pt_get_y(pointer);
    double amp = h * 0.24;

    Group base_line = mk_group2(mk_pt(w * 0.06, cy), mk_pt(w * 0.94, cy));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#e5e5e5", js_some_double(1.0)), PtLikeIterable_of(base_line));

    JsArray_Pt arr = JsArray_Pt_new();
    int count = 11;
    for (int i = 0; i < count; i++) {
        double u = (double)i / (double)(count - 1);
        double x = w * (0.08 + 0.84 * u);
        double fi = (double)i;
        double y = cy
            + approx_sin(time_ms * 0.0014 + fi * 0.72) * amp
            + approx_cos(time_ms * 0.0009 - fi * 0.45) * (amp * 0.42);
        double dx = abs_d(x - px);
        if (dx < w * 0.25 && (px > 1.0 || py > 1.0)) {
            double pull = 1.0 - dx / (w * 0.25);
            y += (py - y) * pull * 0.35;
        }
        JsArray_Pt_push(arr, mk_pt(x, y));
    }

    Group ctrls = Group_new(arr);
    Group bs = Curve_bspline(GroupLike_of(ctrls), js_some_double(12.0));
    Group card = Curve_cardinal(PtLikeIterable_of(ctrls), js_some_double(12.0), js_some_double(0.35));
    Group cr = Curve_catmullRom(PtLikeIterable_of(ctrls), js_some_double(12.0));

    JsArray_Pt cr_arr = Group_as_array(cr);
    uint32_t cr_len = JsArray_Pt_length(cr_arr);
    for (uint32_t i = 0; i < cr_len; i += 3) {
        Pt p = JsArray_Pt_get(cr_arr, i);
        Group stem = mk_group2(mk_pt(Pt_get_x(p), cy), p);
        CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#d4d4d4", js_some_double(1.0)), PtLikeIterable_of(stem));
    }

    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#d4d4d4", js_some_double(1.0)), PtLikeIterable_of(ctrls));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#b5b5b5", js_some_double(2.0)), PtLikeIterable_of(bs));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#555555", js_some_double(1.8)), PtLikeIterable_of(card));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#111111", js_some_double(2.6)), PtLikeIterable_of(cr));

    for (uint32_t i = 0; i < (uint32_t)count; i++) {
        Pt p = JsArray_Pt_get(arr, i);
        CanvasForm_point_2(
            CanvasForm_stroke(CanvasForm_fill(g_form, "#ffffff"), "#111111", js_some_double(1.5)),
            PtLike_of(p), js_some_double(4.5), "circle"
        );
    }

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
