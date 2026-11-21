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

    Pts_Pt c = Pts_CanvasSpace_get_center(g_space);
    Pts_Pt size = Pts_CanvasSpace_get_size(g_space);
    Pts_Pt pointer = Pts_CanvasSpace_get_pointer(g_space);
    double sx = Pts_Pt_get_x(size), sy = Pts_Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    if (min_dim < 120.0) min_dim = 120.0;
    double cx = Pts_Pt_get_x(c), cy = Pts_Pt_get_y(c);

    double ang = time_ms * 0.00035;
    double half_len = min_dim * 0.44;
    double ca = cos(ang), sa = sin(ang);
    Pts_Group chord = mk_group2(
        mk_pt(cx - ca * half_len, cy - sa * half_len),
        mk_pt(cx + ca * half_len, cy + sa * half_len)
    );
    Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#999999", 1.2), Pts_PtLikeIterable_of(chord));

    double px = Pts_Pt_get_x(pointer), py = Pts_Pt_get_y(pointer);
    Pts_Pt probe_center = (px > 1.0 || py > 1.0) ? pointer : c;
    Pts_Group probe = Pts_Circle_fromCenter(Pts_PtLike_of(probe_center), min_dim * 0.20);

    Pts_Group targets[5];
    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        double orbit_r = min_dim * (0.14 + 0.04 * fi);
        double theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
        Pts_Pt tc = mk_pt(cx + cos(theta) * orbit_r, cy + sin(theta) * orbit_r);
        double tr = min_dim * (0.11 + 0.025 * (double)(i % 3));
        targets[i] = Pts_Circle_fromCenter(Pts_PtLike_of(tc), tr);

        Pts_CanvasForm_circle_2(
            Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#f7f7f7"), "#737373", 1.4),
            Pts_PtLikeIterable_of(targets[i])
        );
        Pts_Pt foot = Pts_Line_perpendicularFromPt(Pts_PtIterable_of(chord), Pts_PtLike_of(tc), false);
        if (foot.h != 0) {
            Pts_Group stem = mk_group2(tc, foot);
            Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#b5b5b5", 1.0), Pts_PtLikeIterable_of(stem));
            Pts_CanvasForm_point_2(Pts_CanvasForm_fillOnly(g_form, "#555555"), Pts_PtLike_of(foot), 2.5, "circle");
        }
    }

    Pts_CanvasForm_circle_2(Pts_CanvasForm_strokeOnly(g_form, "#111111", 2.0), Pts_PtLikeIterable_of(probe));
    double pcx = Pts_Pt_get_x(probe_center), pcy = Pts_Pt_get_y(probe_center);
    Pts_CanvasForm_line_2(
        Pts_CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        Pts_PtLikeIterable_of(mk_group2(mk_pt(pcx - 6.0, pcy), mk_pt(pcx + 6.0, pcy)))
    );
    Pts_CanvasForm_line_2(
        Pts_CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        Pts_PtLikeIterable_of(mk_group2(mk_pt(pcx, pcy - 6.0), mk_pt(pcx, pcy + 6.0)))
    );

    for (int i = 0; i < 5; i++) {
        Pts_Group hits = Pts_Circle_intersectCircle2D(Pts_PtIterable_of(probe), Pts_PtIterable_of(targets[i]));
        JsArray_Pts_Pt arr = Pts_Group_as_array(hits);
        uint32_t n = JsArray_Pts_Pt_length(arr);
        if (n == 2) {
            Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, "#111111", 1.4), Pts_PtLikeIterable_of(hits));
        }
        for (uint32_t k = 0; k < n; k++) {
            Pts_Pt h = JsArray_Pts_Pt_get(arr, k);
            Pts_CanvasForm_point_2(
                Pts_CanvasForm_stroke(Pts_CanvasForm_fill(g_form, "#ffffff"), "#111111", 2.0),
                Pts_PtLike_of(h), 5.0, "circle"
            );
        }
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
