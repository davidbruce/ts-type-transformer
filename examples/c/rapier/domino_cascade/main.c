#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

static const double TWO_PI = 6.28318530717958647692;
#define DOMINO_COUNT 9
#define DOM_HX 0.13
#define DOM_HY 0.85
#define TRAIL_LEN 22

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;

static Rapier_World g_world = { 0 };
static Rapier_RigidBody g_dominoes[DOMINO_COUNT];
static Rapier_RigidBody g_striker = { 0 };
static double g_trail_x[TRAIL_LEN];
static double g_trail_y[TRAIL_LEN];
static int g_trail_count = 0;
static int g_step_count = 0;
static bool g_paused = false;

static void draw_rot_box(double cx, double cy, double qx, double qy, double qz, double qw, double hx, double hy,
                         const char* fill) {
    double r00 = 1.0 - 2.0 * (qy * qy + qz * qz);
    double r01 = 2.0 * (qx * qy - qw * qz);
    double r10 = 2.0 * (qx * qy + qw * qz);
    double r11 = 1.0 - 2.0 * (qx * qx + qz * qz);

    double xs[4] = {
        to_cx(cx + r00 * (-hx) + r01 * (-hy)),
        to_cx(cx + r00 * hx + r01 * (-hy)),
        to_cx(cx + r00 * hx + r01 * hy),
        to_cx(cx + r00 * (-hx) + r01 * hy)
    };
    double ys[4] = {
        to_cy(cy + r10 * (-hx) + r11 * (-hy)),
        to_cy(cy + r10 * hx + r11 * (-hy)),
        to_cy(cy + r10 * hx + r11 * hy),
        to_cy(cy + r10 * (-hx) + r11 * hy)
    };

    CanvasRenderingContext2D_set_fillStyle(g_ctx, fill);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_moveTo(g_ctx, xs[0], ys[0]);
    for (int i = 1; i < 4; i++) CanvasRenderingContext2D_lineTo(g_ctx, xs[i], ys[i]);
    CanvasRenderingContext2D_closePath(g_ctx);
    CanvasRenderingContext2D_fill(g_ctx);

    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_moveTo(g_ctx, xs[0], ys[0]);
    for (int i = 1; i < 4; i++) CanvasRenderingContext2D_lineTo(g_ctx, xs[i], ys[i]);
    CanvasRenderingContext2D_closePath(g_ctx);
    CanvasRenderingContext2D_stroke(g_ctx);
}

static void build_scene(void) {
    js_scope();
    for (int i = 0; i < DOMINO_COUNT; i++) {
        if (js_not_null(g_dominoes[i])) {
            js_release(g_dominoes[i]);
            g_dominoes[i].h = 0;
        }
    }
    if (js_not_null(g_striker)) {
        js_release(g_striker);
        g_striker.h = 0;
    }
    if (js_not_null(g_world)) {
        Rapier_World_free(g_world);
        js_release(g_world);
    }
    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    Rapier_RigidBody floor = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, -0.35, 0.0));
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(7.2, 0.35, 2.0), 0.8), floor);

    for (int i = 0; i < DOMINO_COUNT; i++) {
        double x = -2.4 + (double)i * 0.88;
        g_dominoes[i] = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), x, DOM_HY, 0.0));
        js_retain(g_dominoes[i]);
        Rapier_ColliderDesc cd = Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(DOM_HX, DOM_HY, 0.35), 0.55), 0.15), 1.0);
        Rapier_World_createCollider(g_world, cd, g_dominoes[i]);
    }

    g_striker = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), -5.3, 1.05, 0.0));
    js_retain(g_striker);
    Rapier_ColliderDesc bcd = Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_ball(0.40), 2.8), 0.45), 0.25);
    Rapier_World_createCollider(g_world, bcd, g_striker);
    Rapier_RigidBody_applyImpulse(g_striker, Rapier_Vector3_new(4.8, 0.2, 0.0), true);

    g_step_count = 0;
    g_trail_count = 0;
}

static void step_sim(void) {
    js_scope();
    Rapier_World_step(g_world);
    g_step_count++;
    Rapier_Vector sp = Rapier_RigidBody_translation(g_striker);
    int idx = (g_step_count - 1) % TRAIL_LEN;
    g_trail_x[idx] = Rapier_Vector_get_x(sp);
    g_trail_y[idx] = Rapier_Vector_get_y(sp);
    if (g_trail_count < TRAIL_LEN) g_trail_count++;
}

static void render(void) {
    js_scope();
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#ffffff");
    CanvasRenderingContext2D_fillRect(g_ctx, 0.0, 0.0, 680.0, 400.0);

    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#efefef");
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

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#f7f7f7");
    CanvasRenderingContext2D_fillRect(g_ctx, 0.0, 350.0, 680.0, 50.0);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_moveTo(g_ctx, 0.0, 350.0);
    CanvasRenderingContext2D_lineTo(g_ctx, 680.0, 350.0);
    CanvasRenderingContext2D_stroke(g_ctx);

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#b5b5b5");
    for (int i = 0; i < g_trail_count; i++) {
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, to_cx(g_trail_x[i]), to_cy(g_trail_y[i]), 2.0, 0.0, TWO_PI);
        CanvasRenderingContext2D_fill(g_ctx);
    }

    int toppled = 0;
    for (int i = 0; i < DOMINO_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_dominoes[i]);
        Rapier_Rotation q = Rapier_RigidBody_rotation(g_dominoes[i]);
        double px = Rapier_Vector_get_x(p), py = Rapier_Vector_get_y(p);
        double qx = Rapier_Rotation_get_x(q), qy = Rapier_Rotation_get_y(q), qz = Rapier_Rotation_get_z(q), qw = Rapier_Rotation_get_w(q);
        bool tilted = fabs(qz) > 0.25;
        if (tilted) toppled++;
        draw_rot_box(px, py, qx, qy, qz, qw, DOM_HX, DOM_HY, (tilted ? "#d4d4d4" : "#f7f7f7"));
    }

    Rapier_Vector sp = Rapier_RigidBody_translation(g_striker);
    double spx = Rapier_Vector_get_x(sp), spy = Rapier_Vector_get_y(sp);
    double scx = to_cx(spx), scy = to_cy(spy);

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#262626");
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.6);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_stroke(g_ctx);
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#e5e5e5");
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, scx - 5.0, scy - 5.0, 2.5, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);

    char cap[64];
    snprintf(cap, sizeof cap, "Dominoes toppled: %d / 9", toppled);
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#555555");
    CanvasRenderingContext2D_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasRenderingContext2D_fillText(g_ctx, cap, 18.0, 24.0);

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "9 cuboids + striker sphere \xC2\xB7 toppled=%d/9 \xC2\xB7 striker_x=%.2f m \xC2\xB7 step=%d%s",
        toppled, spx, g_step_count, g_paused ? " (paused)" : " (running)"
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    js_scope();
    if (!g_paused) {
        step_sim();
        render();
    }
    requestAnimationFrame(g_raf_cb);
}

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int id = *(const int*)ctx;
    if (id == 0) {
        Rapier_RigidBody_applyImpulse(g_striker, Rapier_Vector3_new(3.5, 0.6, 0.0), true);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 12; i++) step_sim();
        render();
    }
}

static void on_init(void* ctx, bool ok, JsValue v) {
    (void)ctx; (void)v;
    if (!ok) return;
    js_scope();

    Document doc = get_document();
    HTMLCanvasElement canvas = HTMLCanvasElement_cast(Document_getElementById(doc, "canvas"));
    g_ctx = HTMLCanvasElement_getContext_2d(canvas);
    js_retain(g_ctx);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);
    g_pause_btn = Document_getElementById(doc, "btn-pause");
    js_retain(g_pause_btn);

    build_scene();
    for (int i = 0; i < 22; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-push", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    g_raf_cb = FrameRequestCallback_from(on_raf, NULL);
    js_retain(g_raf_cb);
    requestAnimationFrame(g_raf_cb);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Promise_then(rapier_init(), on_init, NULL);
}
