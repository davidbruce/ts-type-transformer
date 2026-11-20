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

static void on_animate(void* ctx, double time_ms, double ftime_ms, Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pt c = CanvasSpace_get_center(g_space);
    Pt size = CanvasSpace_get_size(g_space);
    Pt pointer = CanvasSpace_get_pointer(g_space);
    double sx = Pt_get_x(size), sy = Pt_get_y(size);
    double min_dim = sx < sy ? sx : sy;
    if (min_dim < 120.0) min_dim = 120.0;
    double cx = Pt_get_x(c), cy = Pt_get_y(c);

    double ang = time_ms * 0.00035;
    double half_len = min_dim * 0.44;
    double ca = cos(ang), sa = sin(ang);
    Group chord = mk_group2(
        mk_pt(cx - ca * half_len, cy - sa * half_len),
        mk_pt(cx + ca * half_len, cy + sa * half_len)
    );
    CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#999999", 1.2), PtLikeIterable_of(chord));

    double px = Pt_get_x(pointer), py = Pt_get_y(pointer);
    Pt probe_center = (px > 1.0 || py > 1.0) ? pointer : c;
    Group probe = Circle_fromCenter(PtLike_of(probe_center), min_dim * 0.20);

    Group targets[5];
    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        double orbit_r = min_dim * (0.14 + 0.04 * fi);
        double theta = time_ms * (0.00045 + 0.00012 * fi) + fi * 1.256;
        Pt tc = mk_pt(cx + cos(theta) * orbit_r, cy + sin(theta) * orbit_r);
        double tr = min_dim * (0.11 + 0.025 * (double)(i % 3));
        targets[i] = Circle_fromCenter(PtLike_of(tc), tr);

        CanvasForm_circle_2(
            CanvasForm_stroke(CanvasForm_fill(g_form, "#f7f7f7"), "#737373", 1.4),
            PtLikeIterable_of(targets[i])
        );
        Pt foot = Line_perpendicularFromPt(PtIterable_of(chord), PtLike_of(tc), false);
        if (foot.h != 0) {
            Group stem = mk_group2(tc, foot);
            CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#b5b5b5", 1.0), PtLikeIterable_of(stem));
            CanvasForm_point_2(CanvasForm_fillOnly(g_form, "#555555"), PtLike_of(foot), 2.5, "circle");
        }
    }

    CanvasForm_circle_2(CanvasForm_strokeOnly(g_form, "#111111", 2.0), PtLikeIterable_of(probe));
    double pcx = Pt_get_x(probe_center), pcy = Pt_get_y(probe_center);
    CanvasForm_line_2(
        CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        PtLikeIterable_of(mk_group2(mk_pt(pcx - 6.0, pcy), mk_pt(pcx + 6.0, pcy)))
    );
    CanvasForm_line_2(
        CanvasForm_strokeOnly(g_form, "#111111", 1.4),
        PtLikeIterable_of(mk_group2(mk_pt(pcx, pcy - 6.0), mk_pt(pcx, pcy + 6.0)))
    );

    for (int i = 0; i < 5; i++) {
        Group hits = Circle_intersectCircle2D(PtIterable_of(probe), PtIterable_of(targets[i]));
        JsArray_Pt arr = Group_as_array(hits);
        uint32_t n = JsArray_Pt_length(arr);
        if (n == 2) {
            CanvasForm_line_2(CanvasForm_strokeOnly(g_form, "#111111", 1.4), PtLikeIterable_of(hits));
        }
        for (uint32_t k = 0; k < n; k++) {
            Pt h = JsArray_Pt_get(arr, k);
            CanvasForm_point_2(
                CanvasForm_stroke(CanvasForm_fill(g_form, "#ffffff"), "#111111", 2.0),
                PtLike_of(h), 5.0, "circle"
            );
        }
    }

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
