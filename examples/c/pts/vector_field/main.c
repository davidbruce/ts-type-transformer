#include "pts/pts.h"
#include <math.h>
#include <stdio.h>

static Pts_CanvasSpace g_space;
static Pts_CanvasForm g_form;

static void on_animate(void* ctx, double time_ms, double ftime_ms, Pts_Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pts_Pt pointer = Pts_CanvasSpace_get_pointer(g_space);
    Pts_Group grid = Pts_Create_gridPts(Pts_CanvasSpace_get_innerBound(g_space), 18.0, 11.0);
    Pts_Pt size = Pts_CanvasSpace_get_size(g_space);
    double sx = Pts_Pt_get_x(size);
    double sy = Pts_Pt_get_y(size);
    double max_dist = (sx > sy ? sx : sy) * 0.55;
    if (max_dist < 1.0) max_dist = 1.0;

    JsArray_Pts_Pt items = Pts_Group_as_array(grid);
    uint32_t count = JsArray_Pts_Pt_length(items);
    for (uint32_t i = 0; i < count; i++) {
        Pts_Pt pt = JsArray_Pts_Pt_get(items, i);
        JsArray args = JsArray_new();
        js_push_val(args, pointer);
        Pts_Pt delta = Pts_Pt__subtract(pt, args);
        double dist = Pts_Pt_magnitude(delta);
        double t = 1.0 - Pts_Num_clamp(dist / max_dist, 0.0, 1.0);
        double angle = Pts_Pt_angle(delta) + sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
        double len = 8.0 + 34.0 * t;
        Pts_Group seg = Pts_Line_fromAngle(Pts_PtLike_of(pt), angle, len);

        char color[40];
        int light = (int)(65.0 - 57.0 * t + 0.5);
        double alpha = 0.25 + 0.75 * t;
        snprintf(color, sizeof color, "hsla(0, 0%%, %d%%, %.2f)", light, alpha);

        Pts_CanvasForm_line_2(Pts_CanvasForm_strokeOnly(g_form, color, 1.5 + 2.5 * t), Pts_PtLikeIterable_of(seg));
        Pts_Group dot = Pts_Circle_fromCenter(Pts_PtLike_of(pt), 1.5 + 4.5 * t * t);
        Pts_CanvasForm_circle_2(Pts_CanvasForm_fillOnly(g_form, color), Pts_PtLikeIterable_of(dot));
    }

    double ring_radius = 18.0 + 6.0 * sin(time_ms * 0.004);
    Pts_Group ring = Pts_Circle_fromCenter(Pts_PtLike_of(pointer), ring_radius);
    Pts_CanvasForm_circle_2(Pts_CanvasForm_strokeOnly(g_form, "#11111199", 2.0), Pts_PtLikeIterable_of(ring));

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
