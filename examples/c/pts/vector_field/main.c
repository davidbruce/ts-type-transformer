#include "pts/pts.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)

static CanvasSpace g_space;
static CanvasForm g_form;

static double approx_sin(double x) {
    while (x > PI) x -= TWO_PI;
    while (x < -PI) x += TWO_PI;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0)));
}

static void format_gray_hsla(char* out, int light, int alpha_pct) {
    char* p = out;
    const char* prefix = "hsla(0, 0%, ";
    while (*prefix) *p++ = *prefix++;
    if (light >= 100) *p++ = (char)('0' + (light / 100) % 10);
    if (light >= 10) *p++ = (char)('0' + (light / 10) % 10);
    *p++ = (char)('0' + light % 10);
    const char* mid = "%, ";
    while (*mid) *p++ = *mid++;
    if (alpha_pct >= 100) {
        *p++ = '1';
        *p++ = '.';
        *p++ = '0';
        *p++ = '0';
    } else {
        *p++ = '0';
        *p++ = '.';
        *p++ = (char)('0' + (alpha_pct / 10) % 10);
        *p++ = (char)('0' + alpha_pct % 10);
    }
    *p++ = ')';
    *p = '\0';
}

static void on_animate(void* ctx, double time_ms, double ftime_ms, Space current_space) {
    (void)ctx;
    (void)ftime_ms;
    (void)current_space;
    js_scope_open();

    Pt pointer = MultiTouchSpace_get_pointer(g_space);
    Group grid = Create_gridPts(Space_get_innerBound(g_space), 18.0, 11.0);
    Pt size = Space_get_size(g_space);
    double sx = Pt_get_x(size);
    double sy = Pt_get_y(size);
    double max_dist = (sx > sy ? sx : sy) * 0.55;
    if (max_dist < 1.0) max_dist = 1.0;

    JsArray_Pt items = Group_as_array(grid);
    uint32_t count = JsArray_Pt_length(items);
    for (uint32_t i = 0; i < count; i++) {
        Pt pt = JsArray_Pt_get(items, i);
        JsArray args = JsArray_new();
        JsArray_push(args, js_value(pointer));
        Pt delta = Pt__subtract(pt, args);
        double dist = Pt_magnitude(delta);
        double t = 1.0 - Num_clamp(dist / max_dist, 0.0, 1.0);
        double angle = Pt_angle(delta) + approx_sin(time_ms * 0.0015 + dist * 0.01) * 0.35;
        double len = 8.0 + 34.0 * t;
        Group seg = Line_fromAngle(PtLike_of(pt), angle, len);

        char color[40];
        int light = (int)(65.0 - 57.0 * t + 0.5);
        int alpha_pct = (int)((0.25 + 0.75 * t) * 100.0 + 0.5);
        format_gray_hsla(color, light, alpha_pct);

        CanvasForm_line_2(CanvasForm_strokeOnly(g_form, color, js_some_double(1.5 + 2.5 * t)), PtLikeIterable_of(seg));
        Group dot = Circle_fromCenter(PtLike_of(pt), 1.5 + 4.5 * t * t);
        CanvasForm_circle_2(CanvasForm_fillOnly(g_form, color), PtLikeIterable_of(dot));
    }

    double ring_radius = 18.0 + 6.0 * approx_sin(time_ms * 0.004);
    Group ring = Circle_fromCenter(PtLike_of(pointer), ring_radius);
    CanvasForm_circle_2(CanvasForm_strokeOnly(g_form, "#11111199", js_some_double(2.0)), PtLikeIterable_of(ring));

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
