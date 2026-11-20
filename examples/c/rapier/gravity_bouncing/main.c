#include <stdio.h>
#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

static const double TWO_PI = 6.28318530717958647692;
#define TRAIL_LEN 20

static const double RESTITUTIONS[5] = { 0.00, 0.35, 0.60, 0.80, 0.95 };
static const double X_COORDS[5] = { -4.2, -2.1, 0.0, 2.1, 4.2 };
static const char* BALL_HEX[5] = { "#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626" };
static const char* REST_LABELS[5] = { "e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95" };

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

static string_or_CanvasGradient_or_CanvasPattern C(const char* hex) {
    return string_or_CanvasGradient_or_CanvasPattern_of(hex);
}

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_grav_btn;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;

static World g_world = { 0 };
static RigidBody g_balls[5];
static double g_trail_x[5][TRAIL_LEN];
static double g_trail_y[5][TRAIL_LEN];
static int g_trail_count = 0;
static int g_step_count = 0;
static bool g_earth = true;
static bool g_paused = false;

static void draw_dashed(double x1, double y1, double x2, double y2, int segs) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < segs; i += 2) {
        double t0 = (double)i / (double)segs;
        double t1 = (double)(i + 1) / (double)segs;
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, x1 + dx * t0, y1 + dy * t0);
        CanvasRenderingContext2D_lineTo(g_ctx, x1 + dx * t1, y1 + dy * t1);
        CanvasRenderingContext2D_stroke(g_ctx);
    }
}

static void build_scene(void) {
    js_scope_open();
    for (int i = 0; i < 5; i++) {
        if (g_balls[i].h != 0) {
            js_release(g_balls[i]);
            g_balls[i].h = 0;
        }
    }
    if (g_world.h != 0) {
        World_free(g_world);
        js_release(g_world);
    }
    g_world = World_new(Vector3_new(0.0, g_earth ? -9.81 : -1.62, 0.0));
    js_retain(g_world);

    RigidBody ground = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, -0.4, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(7.0, 0.4, 2.0), ground);
    RigidBody lw = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), -6.8, 3.5, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(0.25, 4.0, 2.0), lw);
    RigidBody rw = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 6.8, 3.5, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(0.25, 4.0, 2.0), rw);

    for (int i = 0; i < 5; i++) {
        g_balls[i] = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), X_COORDS[i], 5.6, 0.0));
        js_retain(g_balls[i]);
        ColliderDesc cd = ColliderDesc_setDensity(ColliderDesc_setFriction(ColliderDesc_setRestitution(ColliderDesc_ball(0.42), RESTITUTIONS[i]), 0.2), 1.2);
        World_createCollider(g_world, cd, g_balls[i]);
    }
    g_step_count = 0;
    g_trail_count = 0;
    js_scope_close();
}

static void step_sim(void) {
    js_scope_open();
    World_step(g_world);
    g_step_count++;
    int idx = (g_step_count - 1) % TRAIL_LEN;
    for (int i = 0; i < 5; i++) {
        Vector p = RigidBody_translation(g_balls[i]);
        g_trail_x[i][idx] = Vector_get_x(p);
        g_trail_y[i][idx] = Vector_get_y(p);
    }
    if (g_trail_count < TRAIL_LEN) g_trail_count++;
    js_scope_close();
}

static void render(void) {
    js_scope_open();
    CanvasRenderingContext2D_set_fillStyle(g_ctx, C("#ffffff"));
    CanvasRenderingContext2D_fillRect(g_ctx, 0.0, 0.0, 680.0, 400.0);

    CanvasRenderingContext2D_set_strokeStyle(g_ctx, C("#efefef"));
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.0);
    for (int gx = -6; gx <= 6; gx++) {
        double x = to_cx((double)gx);
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, x, 0.0);
        CanvasRenderingContext2D_lineTo(g_ctx, x, 350.0);
        CanvasRenderingContext2D_stroke(g_ctx);
    }
    for (int gy = 1; gy <= 7; gy++) {
        double y = to_cy((double)gy);
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, 0.0, y);
        CanvasRenderingContext2D_lineTo(g_ctx, 680.0, y);
        CanvasRenderingContext2D_stroke(g_ctx);
    }

    CanvasRenderingContext2D_set_fillStyle(g_ctx, C("#f7f7f7"));
    CanvasRenderingContext2D_fillRect(g_ctx, 0.0, 350.0, 680.0, 50.0);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, C("#d4d4d4"));
    for (double hx = -20.0; hx < 700.0; hx += 18.0) {
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, hx, 350.0);
        CanvasRenderingContext2D_lineTo(g_ctx, hx - 24.0, 400.0);
        CanvasRenderingContext2D_stroke(g_ctx);
    }
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, C("#111111"));
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_moveTo(g_ctx, 0.0, 350.0);
    CanvasRenderingContext2D_lineTo(g_ctx, 680.0, 350.0);
    CanvasRenderingContext2D_stroke(g_ctx);

    CanvasRenderingContext2D_set_strokeStyle(g_ctx, C("#d4d4d4"));
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.0);
    draw_dashed(to_cx(-5.0), to_cy(5.6), to_cx(5.0), to_cy(5.6), 40);
    for (int i = 0; i < 5; i++) {
        double cx = to_cx(X_COORDS[i]);
        draw_dashed(cx, to_cy(5.6), cx, 350.0, 20);
    }

    double energy = 0.0, max_y = 0.0;
    CanvasRenderingContext2D_set_font(g_ctx, "11px ui-monospace, monospace");
    for (int i = 0; i < 5; i++) {
        CanvasRenderingContext2D_set_fillStyle(g_ctx, C("#b5b5b5"));
        for (int k = 0; k < g_trail_count; k++) {
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_arc(g_ctx, to_cx(g_trail_x[i][k]), to_cy(g_trail_y[i][k]), 1.6, 0.0, TWO_PI);
            CanvasRenderingContext2D_fill(g_ctx);
        }

        Vector p = RigidBody_translation(g_balls[i]);
        Vector v = RigidBody_linvel(g_balls[i]);
        double px = Vector_get_x(p), py = Vector_get_y(p);
        double vx = Vector_get_x(v), vy = Vector_get_y(v);

        energy += 0.5 * (vx * vx + vy * vy);
        if (py > max_y) max_y = py;

        double cx = to_cx(px), cy = to_cy(py), rpx = 0.42 * 46.0;
        CanvasRenderingContext2D_set_fillStyle(g_ctx, C(BALL_HEX[i]));
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, cx, cy, rpx, 0.0, TWO_PI);
        CanvasRenderingContext2D_fill(g_ctx);

        CanvasRenderingContext2D_set_strokeStyle(g_ctx, C("#111111"));
        CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.6);
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, cx, cy, rpx, 0.0, TWO_PI);
        CanvasRenderingContext2D_stroke(g_ctx);

        CanvasRenderingContext2D_set_fillStyle(g_ctx, C(i >= 3 ? "#d4d4d4" : "#737373"));
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, cx - 5.0, cy - 5.0, 2.8, 0.0, TWO_PI);
        CanvasRenderingContext2D_fill(g_ctx);

        CanvasRenderingContext2D_set_fillStyle(g_ctx, C("#111111"));
        CanvasRenderingContext2D_fillText(g_ctx, REST_LABELS[i], cx - 22.0, 368.0);
    }

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "5 spheres \xC2\xB7 g=%s m/s\xC2\xB2 \xC2\xB7 max_y=%.2f m \xC2\xB7 ke=%.1f \xC2\xB7 step=%d%s",
        g_earth ? "Earth (-9.81)" : "Moon (-1.62)",
        max_y, energy, g_step_count, g_paused ? " (paused)" : " (running)"
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    js_scope_open();
    if (!g_paused) {
        step_sim();
        render();
    }
    requestAnimationFrame(g_raf_cb);
    js_scope_close();
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int id = *(const int*)ctx;
    if (id == 0) {
        g_earth = !g_earth;
        World_set_gravity(g_world, Vector3_new(0.0, g_earth ? -9.81 : -1.62, 0.0));
        Vector3 imp = Vector3_new(0.0, 0.25, 0.0);
        for (int i = 0; i < 5; i++) RigidBody_applyImpulse(g_balls[i], imp, true);
        HTMLElement_set_textContent(g_grav_btn, g_earth ? "Gravity: Earth" : "Gravity: Moon");
        render();
    } else if (id == 1) {
        for (int i = 0; i < 5; i++) {
            RigidBody_applyImpulse(g_balls[i], Vector3_new(((double)i - 2.0) * 0.28, 3.4, 0.0), true);
        }
        render();
    } else if (id == 2) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 18; i++) step_sim();
        render();
    }
    js_scope_close();
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx; (void)v;
    if (!ok) return;
    js_scope_open();

    Document doc = get_document();
    HTMLCanvasElement canvas = HTMLCanvasElement_cast(Document_getElementById(doc, "canvas"));
    g_ctx = HTMLCanvasElement_getContext_2d(canvas);
    js_retain(g_ctx);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);
    g_grav_btn = Document_getElementById(doc, "btn-gravity");
    js_retain(g_grav_btn);
    g_pause_btn = Document_getElementById(doc, "btn-pause");
    js_retain(g_pause_btn);

    build_scene();
    for (int i = 0; i < 28; i++) step_sim();
    render();

    static const char* ids[4] = { "btn-gravity", "btn-kick", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    g_raf_cb = FrameRequestCallback_from(on_raf, NULL);
    js_retain(g_raf_cb);
    requestAnimationFrame(g_raf_cb);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Promise_then(init(), on_init, NULL);
    js_scope_close();
}
