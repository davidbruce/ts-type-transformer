#include "pts/pts.h"
#include <math.h>

static Pts_Pt mk_pt(double x, double y) {
    Pts_Pt p = Pts_Pt_make(2.0);
    Pts_Pt_set_x(p, x);
    Pts_Pt_set_y(p, y);
    return p;
}

static Pts_Group mk_group2(Pts_Pt a, Pts_Pt b) {
    return Pts_Group_new(JsArray_Pts_Pt_from((Pts_Pt[]){ a, b }, 2));
}

static Pts_CanvasSpace g_space;
static Pts_CanvasForm g_form;

static void on_animate(void* ctx, double time_ms, double ftime_ms, Pts_Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pts_Pt size = Pts_CanvasSpace_get_size(g_space);
    Pts_Pt c = Pts_CanvasSpace_get_center(g_space);
    Pts_Pt pointer = Pts_CanvasSpace_get_pointer(g_space);
    double w = Pts_Pt_get_x(size); if (w < 120.0) w = 120.0;
    double h = Pts_Pt_get_y(size); if (h < 120.0) h = 120.0;
    double cy = Pts_Pt_get_y(c);
    double px = Pts_Pt_get_x(pointer), py = Pts_Pt_get_y(pointer);
    double amp = h * 0.24;

    Pts_Group base_line = mk_group2(mk_pt(w * 0.06, cy), mk_pt(w * 0.94, cy));
    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#e5e5e5", 1.0), Pts_PtLikeIterable_of(base_line));

    JsArray_Pts_Pt arr = JsArray_Pts_Pt_new();
    int count = 11;
    for (int i = 0; i < count; i++) {
        double u = (double)i / (double)(count - 1);
        double x = w * (0.08 + 0.84 * u);
        double fi = (double)i;
        double y = cy
            + sin(time_ms * 0.0014 + fi * 0.72) * amp
            + cos(time_ms * 0.0009 - fi * 0.45) * (amp * 0.42);
        double dx = fabs(x - px);
        if (dx < w * 0.25 && (px > 1.0 || py > 1.0)) {
            double pull = 1.0 - dx / (w * 0.25);
            y += (py - y) * pull * 0.35;
        }
        JsArray_Pts_Pt_push(arr, mk_pt(x, y));
    }

    Pts_Group ctrls = Pts_Group_new(arr);
    Pts_Group bs = Pts_Curve_bspline(Pts_GroupLike_of(ctrls), 12.0);
    Pts_Group card = Pts_Curve_cardinal(Pts_PtLikeIterable_of(ctrls), 12.0, 0.35);
    Pts_Group cr = Pts_Curve_catmullRom(Pts_PtLikeIterable_of(ctrls), 12.0);

    JsArray_Pts_Pt cr_arr = Pts_Group_as_array(cr);
    uint32_t cr_len = JsArray_Pts_Pt_length(cr_arr);
    for (uint32_t i = 0; i < cr_len; i += 3) {
        Pts_Pt p = JsArray_Pts_Pt_get(cr_arr, i);
        Pts_Group stem = mk_group2(mk_pt(Pts_Pt_get_x(p), cy), p);
        Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), Pts_PtLikeIterable_of(stem));
    }

    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#d4d4d4", 1.0), Pts_PtLikeIterable_of(ctrls));
    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#b5b5b5", 2.0), Pts_PtLikeIterable_of(bs));
    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#555555", 1.8), Pts_PtLikeIterable_of(card));
    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#111111", 2.6), Pts_PtLikeIterable_of(cr));

    for (uint32_t i = 0; i < (uint32_t)count; i++) {
        Pts_Pt p = JsArray_Pts_Pt_get(arr, i);
        Pts_CanvasForm_point_2(
            Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#ffffff"), "#111111", 1.5),
            Pts_PtLike_of(p), 4.5, "circle"
        );
    }

    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();

    Pts_CanvasSpaceOptions opts = Pts_CanvasSpaceOptions_create();
    Pts_CanvasSpaceOptions_set_bgcolor(opts, "#ffffff");
    Pts_CanvasSpaceOptions_set_resize(opts, true);
    Pts_CanvasSpaceOptions_set_retina(opts, true);

    g_space = Pts_CanvasSpace_setup(Pts_CanvasSpace_new("#pt"), opts);
    g_form = Pts_CanvasSpace_getForm(g_space);

    Pts_IPlayer player = Pts_IPlayer_create();
    Pts_IPlayer_set_animate(player, Pts_AnimateCallbackFn_from(on_animate, 0));
    Pts_CanvasSpace_add(g_space, player);
    Pts_CanvasSpace_bindMouse(g_space);
    Pts_CanvasSpace_bindTouch(g_space);
    Pts_CanvasSpace_play(g_space);

    js_retain(g_space);
    js_retain(g_form);
    js_scope_close();
}
