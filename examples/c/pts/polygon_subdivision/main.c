#include "pts/pts.h"
#include <math.h>

static Pt mk_pt(double x, double y) {
    Pt p = Pt_make(2.0);
    Pt_set_x(p, x);
    Pt_set_y(p, y);
    return p;
}

static Group mk_group2(Pt a, Pt b) {
    return Group_new(JsArray_Pt_from((Pt[]){ a, b }, 2));
}

static CanvasSpace g_space;
static CanvasForm g_form;

static const char* SHADES[9] = {
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626"
};

static void on_animate(void* ctx, double time_ms, double ftime_ms, Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pt c = CanvasSpace_get_center(g_space);
    Pt size = CanvasSpace_get_size(g_space);
    Pt pointer = CanvasSpace_get_pointer(g_space);
    double sx = Pt_get_x(size), sy = Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    double radius = min_dim * 0.40;
    if (radius < 80.0) radius = 80.0;

    double px_norm = (Pt_get_x(pointer) / (sx > 1.0 ? sx : 1.0)) - 0.5;
    double t = Num_clamp(0.22 + 0.16 * sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

    Group poly = Polygon_fromCenter(PtLike_of(c), radius, 6.0);
    Group_rotate2D(poly, time_ms * 0.00035, PtLike_of(c));
    Pt centroid = Polygon_centroid(PtLikeIterable_of(poly));

    for (int depth = 0; depth < 9; depth++) {
        double sw = depth == 0 ? 1.6 : 1.0;
        CanvasForm f = CanvasForm_stroke(CanvasForm_fill(g_form, SHADES[depth]), "#111111", sw);
        CanvasForm_polygon_2(f, PtLikeIterable_of(poly));
        if (depth == 0) {
            JsArray_Pt items = Group_as_array(poly);
            uint32_t n = JsArray_Pt_length(items);
            for (uint32_t i = 0; i < n; i++) {
                Pt pt = JsArray_Pt_get(items, i);
                Group ray = mk_group2(centroid, pt);
                CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), PtLikeIterable_of(ray));
                CanvasForm_point_2(CanvasForm_fillOnly(g_form, "#111111"), PtLike_of(pt), 3.5, "circle");
            }
        }
        poly = Polygon_midpoints(PtIterable_of(poly), true, t);
    }

    double cx = Pt_get_x(c), cy = Pt_get_y(c);
    Group hbar = mk_group2(mk_pt(cx - 10.0, cy), mk_pt(cx + 10.0, cy));
    Group vbar = mk_group2(mk_pt(cx, cy - 10.0), mk_pt(cx, cy + 10.0));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#ffffff", 1.5), PtLikeIterable_of(hbar));
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#ffffff", 1.5), PtLikeIterable_of(vbar));
    CanvasForm_point_2(
        CanvasForm_stroke(CanvasForm_fill(g_form, "#ffffff"), "#111111", 1.5),
        PtLike_of(centroid), 5.0, "circle"
    );

    Group ring = Circle_fromCenter(PtLike_of(pointer), 10.0);
    CanvasForm_circle_2(CanvasForm_strokeOnly(g_form, "#737373", 1.5), PtLikeIterable_of(ring));

    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    CanvasSpaceOptions opts = CanvasSpaceOptions_create();
    CanvasSpaceOptions_set_bgcolor(opts, "#ffffff");
    CanvasSpaceOptions_set_resize(opts, true);
    CanvasSpaceOptions_set_retina(opts, true);

    g_space = CanvasSpace_setup(CanvasSpace_new("#pt"), opts);
    g_form = CanvasSpace_getForm(g_space);

    IPlayer player = IPlayer_create();
    IPlayer_set_animate(player, AnimateCallbackFn_from(on_animate, 0));
    CanvasSpace_add(g_space, player);
    CanvasSpace_bindMouse(g_space);
    CanvasSpace_bindTouch(g_space);
    CanvasSpace_play(g_space);

    js_retain(g_space);
    js_retain(g_form);
    js_scope_close();
}
