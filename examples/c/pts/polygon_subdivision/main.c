#include "pts/pts.h"
#include <math.h>

static Pts_Pt mk_pt(double x, double y) {
    Pts_Pt p = Pts_Pt_make(2.0);
    Pts_Pt_set_x(p, x);
    Pts_Pt_set_y(p, y);
    return p;
}

static Pts_Group mk_group2(Pts_Pt a, Pts_Pt b) {
    return Pts_Group_new(a, b);
}

static Pts_CanvasSpace g_space;
static Pts_CanvasForm g_form;

static const char* SHADES[9] = {
    "#f7f7f7", "#efefef", "#e5e5e5",
    "#d4d4d4", "#b5b5b5", "#999999",
    "#737373", "#404040", "#262626"
};

static void on_animate(void* ctx, double time_ms, double ftime_ms, Pts_Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope();

    Pts_Pt c = Pts_CanvasSpace_get_center(g_space);
    Pts_Pt size = Pts_CanvasSpace_get_size(g_space);
    Pts_Pt pointer = Pts_CanvasSpace_get_pointer(g_space);
    double sx = Pts_Pt_get_x(size), sy = Pts_Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    double radius = min_dim * 0.40;
    if (radius < 80.0) radius = 80.0;

    double px_norm = (Pts_Pt_get_x(pointer) / (sx > 1.0 ? sx : 1.0)) - 0.5;
    double t = Pts_Num_clamp(0.22 + 0.16 * sin(time_ms * 0.0012) + px_norm * 0.24, 0.10, 0.48);

    Pts_Group poly = Pts_Polygon_fromCenter(c, radius, 6.0);
    Pts_Group_rotate2D(poly, time_ms * 0.00035, c);
    Pts_Pt centroid = Pts_Polygon_centroid(poly);

    for (int depth = 0; depth < 9; depth++) {
        double sw = depth == 0 ? 1.6 : 1.0;
        Pts_CanvasForm f = Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, SHADES[depth]), "#111111", sw);
        Pts_CanvasForm_polygon(f, poly);
        if (depth == 0) {
            JsArray_Pts_Pt items = Pts_Group_as_array(poly);
            uint32_t n = JsArray_Pts_Pt_length(items);
            for (uint32_t i = 0; i < n; i++) {
                Pts_Pt pt = JsArray_Pts_Pt_get(items, i);
                Pts_Group ray = mk_group2(centroid, pt);
                Pts_CanvasForm_line(Pts_CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), ray);
                Pts_CanvasForm_point(Pts_CanvasForm_fillOnly(g_form, "#111111"), pt, 3.5, "circle");
            }
        }
        poly = Pts_Polygon_midpoints(poly, true, t);
    }

    double cx = Pts_Pt_get_x(c), cy = Pts_Pt_get_y(c);
    Pts_Group hbar = mk_group2(mk_pt(cx - 10.0, cy), mk_pt(cx + 10.0, cy));
    Pts_Group vbar = mk_group2(mk_pt(cx, cy - 10.0), mk_pt(cx, cy + 10.0));
    Pts_CanvasForm_line(Pts_CanvasForm_strokeOnly(g_form, "#ffffff", 1.5), hbar);
    Pts_CanvasForm_line(Pts_CanvasForm_strokeOnly(g_form, "#ffffff", 1.5), vbar);
    Pts_CanvasForm_point(
        Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#ffffff"), "#111111", 1.5),
        centroid, 5.0, "circle"
    );

    Pts_Group ring = Pts_Circle_fromCenter(pointer, 10.0);
    Pts_CanvasForm_circle(Pts_CanvasForm_strokeOnly(g_form, "#737373", 1.5), ring);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();

    g_space = Pts_CanvasSpace_setup(Pts_CanvasSpace_new("#pt"),
        Pts_CanvasSpaceOptions_create(.bgcolor = "#ffffff", .resize = true, .retina = true));
    g_form = Pts_CanvasSpace_getForm(g_space);

    Pts_IPlayer player = Pts_IPlayer_create();
    Pts_IPlayer_set_animate(player, on_animate);
    Pts_CanvasSpace_add(g_space, player);
    Pts_CanvasSpace_bindMouse(g_space);
    Pts_CanvasSpace_bindTouch(g_space);
    Pts_CanvasSpace_play(g_space);

    js_retain(g_space);
    js_retain(g_form);
}
