#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

#define TWO_PI 6.28318530717958647692
#define TRAIL_LEN 20

static const double RESTITUTIONS[5] = { 0.00, 0.35, 0.60, 0.80, 0.95 };
static const double X_COORDS[5] = { -4.2, -2.1, 0.0, 2.1, 4.2 };
static const char* BALL_HEX[5] = { "#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626" };
static const char* REST_LABELS[5] = { "e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95" };

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

typedef struct { char buf[256]; int pos; } StrBuf;
static void sb_ch(StrBuf* s, char c) { if (s->pos < 254) s->buf[s->pos++] = c; }
static void sb_str(StrBuf* s, const char* p) { while (*p) sb_ch(s, *p++); }
static void sb_int(StrBuf* s, int v) {
    if (v < 0) { sb_ch(s, '-'); v = -v; }
    char tmp[16]; int n = 0;
    do { tmp[n++] = (char)('0' + (v % 10)); v /= 10; } while (v > 0);
    while (n--) sb_ch(s, tmp[n]);
}
static void sb_f1(StrBuf* s, double v) {
    if (v < 0.0) { sb_ch(s, '-'); v = -v; }
    int t = (int)(v * 10.0 + 0.5);
    sb_int(s, t / 10);
    sb_ch(s, '.');
    sb_ch(s, (char)('0' + (t % 10)));
}
static void sb_f2(StrBuf* s, double v) {
    if (v < 0.0) { sb_ch(s, '-'); v = -v; }
    int t = (int)(v * 100.0 + 0.5);
    sb_int(s, t / 100);
    sb_ch(s, '.');
    sb_ch(s, (char)('0' + ((t / 10) % 10)));
    sb_ch(s, (char)('0' + (t % 10)));
}

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_grav_btn;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;

static string_or_CanvasGradient_or_CanvasPattern g_c_white;
static string_or_CanvasGradient_or_CanvasPattern g_c_grid;
static string_or_CanvasGradient_or_CanvasPattern g_c_f7;
static string_or_CanvasGradient_or_CanvasPattern g_c_d4;
static string_or_CanvasGradient_or_CanvasPattern g_c_b5;
static string_or_CanvasGradient_or_CanvasPattern g_c_73;
static string_or_CanvasGradient_or_CanvasPattern g_c_11;
static string_or_CanvasGradient_or_CanvasPattern g_c_balls[5];

static World g_world = { 0 };
static RigidBody g_balls[5];
static double g_trail_x[5][TRAIL_LEN];
static double g_trail_y[5][TRAIL_LEN];
static int g_trail_count = 0;
static int g_step_count = 0;
static bool g_earth = true;
static bool g_paused = false;

static string_or_CanvasGradient_or_CanvasPattern mk_color(const char* hex) {
    return string_or_CanvasGradient_or_CanvasPattern_of_string(JsString_from(hex));
}

static void draw_dashed(double x1, double y1, double x2, double y2, int segs) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < segs; i += 2) {
        double t0 = (double)i / (double)segs;
        double t1 = (double)(i + 1) / (double)segs;
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, x1 + dx * t0, y1 + dy * t0);
        CanvasPath_lineTo(g_ctx, x1 + dx * t1, y1 + dy * t1);
        CanvasDrawPath_stroke(g_ctx);
    }
}

static void build_scene(void) {
    if (g_world.h != 0) {
        World_free(g_world);
        js_release(g_world);
    }
    Vector3 gv = Vector3_new(0.0, g_earth ? -9.81 : -1.62, 0.0);
    g_world = World_new(gv);
    js_release(gv);

    RigidBody ground = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, -0.4, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(7.0, 0.4, 2.0), ground);
    RigidBody lw = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), -6.8, 3.5, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(0.25, 4.0, 2.0), lw);
    RigidBody rw = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 6.8, 3.5, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(0.25, 4.0, 2.0), rw);

    for (int i = 0; i < 5; i++) {
        g_balls[i] = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), X_COORDS[i], 5.6, 0.0));
        ColliderDesc cd = ColliderDesc_setDensity(ColliderDesc_setFriction(ColliderDesc_setRestitution(ColliderDesc_ball(0.42), RESTITUTIONS[i]), 0.2), 1.2);
        World_createCollider(g_world, cd, g_balls[i]);
    }
    g_step_count = 0;
    g_trail_count = 0;
}

static void step_sim(void) {
    World_step(g_world);
    g_step_count++;
    int idx = (g_step_count - 1) % TRAIL_LEN;
    for (int i = 0; i < 5; i++) {
        Vector p = RigidBody_translation(g_balls[i]);
        g_trail_x[i][idx] = Vector_get_x(p);
        g_trail_y[i][idx] = Vector_get_y(p);
        js_release(p);
    }
    if (g_trail_count < TRAIL_LEN) g_trail_count++;
}

static void render(void) {
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_white);
    CanvasRect_fillRect(g_ctx, 0.0, 0.0, 680.0, 400.0);

    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_grid);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.0);
    for (int gx = -6; gx <= 6; gx++) {
        double x = to_cx((double)gx);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, x, 0.0);
        CanvasPath_lineTo(g_ctx, x, 350.0);
        CanvasDrawPath_stroke(g_ctx);
    }
    for (int gy = 1; gy <= 7; gy++) {
        double y = to_cy((double)gy);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, 0.0, y);
        CanvasPath_lineTo(g_ctx, 680.0, y);
        CanvasDrawPath_stroke(g_ctx);
    }

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_f7);
    CanvasRect_fillRect(g_ctx, 0.0, 350.0, 680.0, 50.0);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_d4);
    for (double hx = -20.0; hx < 700.0; hx += 18.0) {
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, hx, 350.0);
        CanvasPath_lineTo(g_ctx, hx - 24.0, 400.0);
        CanvasDrawPath_stroke(g_ctx);
    }
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_moveTo(g_ctx, 0.0, 350.0);
    CanvasPath_lineTo(g_ctx, 680.0, 350.0);
    CanvasDrawPath_stroke(g_ctx);

    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_d4);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.0);
    draw_dashed(to_cx(-5.0), to_cy(5.6), to_cx(5.0), to_cy(5.6), 40);
    for (int i = 0; i < 5; i++) {
        double cx = to_cx(X_COORDS[i]);
        draw_dashed(cx, to_cy(5.6), cx, 350.0, 20);
    }

    double energy = 0.0, max_y = 0.0;
    CanvasTextDrawingStyles_set_font(g_ctx, "11px ui-monospace, monospace");
    for (int i = 0; i < 5; i++) {
        CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_b5);
        for (int k = 0; k < g_trail_count; k++) {
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_arc(g_ctx, to_cx(g_trail_x[i][k]), to_cy(g_trail_y[i][k]), 1.6, 0.0, TWO_PI);
            CanvasDrawPath_fill(g_ctx);
        }

        Vector p = RigidBody_translation(g_balls[i]);
        Vector v = RigidBody_linvel(g_balls[i]);
        double px = Vector_get_x(p), py = Vector_get_y(p);
        double vx = Vector_get_x(v), vy = Vector_get_y(v);
        js_release(p);
        js_release(v);

        energy += 0.5 * (vx * vx + vy * vy);
        if (py > max_y) max_y = py;

        double cx = to_cx(px), cy = to_cy(py), rpx = 0.42 * 46.0;
        CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_balls[i]);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, cx, cy, rpx, 0.0, TWO_PI);
        CanvasDrawPath_fill(g_ctx);

        CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
        CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.6);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, cx, cy, rpx, 0.0, TWO_PI);
        CanvasDrawPath_stroke(g_ctx);

        CanvasFillStrokeStyles_set_fillStyle(g_ctx, i >= 3 ? g_c_d4 : g_c_73);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, cx - 5.0, cy - 5.0, 2.8, 0.0, TWO_PI);
        CanvasDrawPath_fill(g_ctx);

        CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_11);
        CanvasText_fillText(g_ctx, REST_LABELS[i], cx - 22.0, 368.0);
    }

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "5 spheres \xC2\xB7 g=");
    sb_str(&sb, g_earth ? "Earth (-9.81)" : "Moon (-1.62)");
    sb_str(&sb, " m/s\xC2\xB2 \xC2\xB7 max_y=");
    sb_f2(&sb, max_y);
    sb_str(&sb, " m \xC2\xB7 ke=");
    sb_f1(&sb, energy);
    sb_str(&sb, " \xC2\xB7 step=");
    sb_int(&sb, g_step_count);
    sb_str(&sb, g_paused ? " (paused)" : " (running)");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(g_meta, sb.buf);
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    if (!g_paused) {
        step_sim();
        render();
    }
    requestAnimationFrame(g_raf_cb);
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) {
        g_earth = !g_earth;
        Vector3 gv = Vector3_new(0.0, g_earth ? -9.81 : -1.62, 0.0);
        World_set_gravity(g_world, gv);
        js_release(gv);
        Vector3 imp = Vector3_new(0.0, 0.25, 0.0);
        for (int i = 0; i < 5; i++) RigidBody_applyImpulse(g_balls[i], imp, true);
        js_release(imp);
        Node_set_textContent(g_grav_btn, g_earth ? "Gravity: Earth" : "Gravity: Moon");
        render();
    } else if (id == 1) {
        for (int i = 0; i < 5; i++) {
            Vector3 imp = Vector3_new(((double)i - 2.0) * 0.28, 3.4, 0.0);
            RigidBody_applyImpulse(g_balls[i], imp, true);
            js_release(imp);
        }
        render();
    } else if (id == 2) {
        g_paused = !g_paused;
        Node_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        Node_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 18; i++) step_sim();
        render();
    }
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx; (void)v;
    if (!ok) return;

    Document doc = get_document();
    HTMLCanvasElement canvas = HTMLCanvasElement_cast(Document_getElementById(doc, "canvas"));
    g_ctx = HTMLCanvasElement_getContext_2d(canvas);
    g_meta = Document_getElementById(doc, "meta");
    g_grav_btn = Document_getElementById(doc, "btn-gravity");
    g_pause_btn = Document_getElementById(doc, "btn-pause");

    g_c_white = mk_color("#ffffff");
    g_c_grid = mk_color("#efefef");
    g_c_f7 = mk_color("#f7f7f7");
    g_c_d4 = mk_color("#d4d4d4");
    g_c_b5 = mk_color("#b5b5b5");
    g_c_73 = mk_color("#737373");
    g_c_11 = mk_color("#111111");
    for (int i = 0; i < 5; i++) g_c_balls[i] = mk_color(BALL_HEX[i]);

    build_scene();
    for (int i = 0; i < 28; i++) step_sim();
    render();

    static const char* ids[4] = { "btn-gravity", "btn-kick", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    g_raf_cb = FrameRequestCallback_from(on_raf, 0);
    requestAnimationFrame(g_raf_cb);
}

__attribute__((export_name("start"))) void start(void) {
    Promise p = init();
    Promise_then(p, on_init, 0);
}
