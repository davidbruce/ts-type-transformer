#include "pts/pts.h"
#include <math.h>
#include <stdio.h>

static CanvasSpace g_space;
static CanvasForm g_form;

static void on_animate(void* ctx, double time_ms, double ftime_ms, Space current_space) {
    (void)ctx; (void)ftime_ms; (void)current_space;
    js_scope_open();

    Pt pointer = CanvasSpace_get_pointer(g_space);
    Group grid = Create_gridPts(CanvasSpace_get_innerBound(g_space), 18.0, 11.0);
    Pt size = CanvasSpace_get_size(g_space);
    double sx = Pt_get_x(size);
    double sy = Pt_get_y(size);
    double max_dist = (sx > sy ? sx : sy) * 0.55;
    if (max_dist < 1.0) max_dist = 1.0;

    JsArray_Pt items = Group_as_array(grid);
    uint32_t count = JsArray_Pt_length(items);
    for (uint32_t i = 0; i < count; i++) {
        Pt pt = JsArray_Pt_get(items, i);
        JsArray args = JsArray_new();
        js_push_val(args, pointer);
        Pt delta = Pt__subtract(pt, args);
        double dist = Pt_magnitude(delta);
        double t = 1.0 - Num_clamp(dist / max_dist, 0.0, 1.0);
        double angle = Pt_angle(delta) + sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
        double len = 8.0 + 34.0 * t;
        Group seg = Line_fromAngle(PtLike_of(pt), angle, len);

        char color[40];
        int light = (int)(65.0 - 57.0 * t + 0.5);
        double alpha = 0.25 + 0.75 * t;
        snprintf(color, sizeof color, "hsla(0, 0%%, %d%%, %.2f)", light, alpha);

        CanvasForm_line_2(CanvasForm_strokeOnly(g_form, color, 1.5 + 2.5 * t), PtLikeIterable_of(seg));
        Group dot = Circle_fromCenter(PtLike_of(pt), 1.5 + 4.5 * t * t);
        CanvasForm_circle_2(CanvasForm_fillOnly(g_form, color), PtLikeIterable_of(dot));
    }

    double ring_radius = 18.0 + 6.0 * sin(time_ms * 0.004);
    Group ring = Circle_fromCenter(PtLike_of(pointer), ring_radius);
    CanvasForm_circle_2(CanvasForm_strokeOnly(g_form, "#11111199", 2.0), PtLikeIterable_of(ring));

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
