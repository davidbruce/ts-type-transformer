#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

static const double TWO_PI = 6.28318530717958647692;
#define BODY_COUNT 6

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;
static Rapier_DrainCollisionEventsFn g_col_cb;

static Rapier_World g_world = { 0 };
static Rapier_EventQueue g_queue = { 0 };
static Rapier_RigidBody g_bodies[BODY_COUNT];
static const bool IS_BOX[BODY_COUNT] = { false, false, true, false, true, false };
static double g_sensor_handle = 0.0;
static int g_step_count = 0;
static int g_solid_contacts = 0;
static int g_sensor_crossings = 0;
static int g_active_in_sensor = 0;
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

static void draw_segs(double x1, double y1, double x2, double y2, int n) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < n; i += 2) {
        double t0 = (double)i / (double)n;
        double t1 = (double)(i + 1) / (double)n;
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, x1 + dx * t0, y1 + dy * t0);
        CanvasRenderingContext2D_lineTo(g_ctx, x1 + dx * t1, y1 + dy * t1);
        CanvasRenderingContext2D_stroke(g_ctx);
    }
}

static void add_ramp(double cx, double cy, double hx, double hy, double theta) {
    Rapier_Quaternion q = Rapier_Quaternion_new(0.0, 0.0, sin(theta * 0.5), cos(theta * 0.5));
    Rapier_RigidBodyDesc d = Rapier_RigidBodyDesc_setRotation(Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), cx, cy, 0.0), q);
    Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, d);
    Rapier_ColliderDesc cd = Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(hx, hy, 1.0), 0.15), 0.35);
    Rapier_World_createCollider(g_world, cd, rb);
}

static void on_collision(void* ctx, double h1, double h2, bool started) {
    (void)ctx;
    if (h1 == g_sensor_handle || h2 == g_sensor_handle) {
        if (started) {
            g_sensor_crossings++;
            g_active_in_sensor++;
        } else if (g_active_in_sensor > 0) {
            g_active_in_sensor--;
        }
    } else if (started) {
        g_solid_contacts++;
    }
}

static void build_scene(void) {
    js_scope();
    for (int i = 0; i < BODY_COUNT; i++) {
        if (js_not_null(g_bodies[i])) {
            js_release(g_bodies[i]);
            g_bodies[i].h = 0;
        }
    }
    if (js_not_null(g_queue)) {
        Rapier_EventQueue_free(g_queue);
        js_release(g_queue);
    }
    if (js_not_null(g_world)) {
        Rapier_World_free(g_world);
        js_release(g_world);
    }
    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);
    g_queue = Rapier_EventQueue_new(true);
    js_retain(g_queue);

    add_ramp(0.0, -0.35, 6.8, 0.35, 0.0);
    add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36);
    add_ramp(1.8, 2.6, 2.2, 0.18, 0.36);

    Rapier_RigidBody sbody = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, 1.35, 0.0));
    Rapier_ColliderDesc scd = Rapier_ColliderDesc_setActiveEvents(Rapier_ColliderDesc_setSensor(Rapier_ColliderDesc_cuboid(1.85, 0.30, 1.2), true), Rapier_ActiveEvents_COLLISION_EVENTS);
    Rapier_Collider sc = Rapier_World_createCollider(g_world, scd, sbody);
    g_sensor_handle = Rapier_Collider_get_handle(sc);

    for (int i = 0; i < BODY_COUNT; i++) {
        double x = -3.7 + (double)i * 0.52;
        double y = 5.4 + (double)(i % 3) * 0.58;
        g_bodies[i] = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), x, y, 0.0));
        js_retain(g_bodies[i]);
        Rapier_ColliderDesc cd = IS_BOX[i] ? Rapier_ColliderDesc_cuboid(0.28, 0.28, 0.28) : Rapier_ColliderDesc_ball(0.32);
        cd = Rapier_ColliderDesc_setActiveEvents(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_setRestitution(cd, 0.35), 0.25), Rapier_ActiveEvents_COLLISION_EVENTS);
        Rapier_World_createCollider(g_world, cd, g_bodies[i]);
    }

    g_step_count = 0;
    g_solid_contacts = 0;
    g_sensor_crossings = 0;
    g_active_in_sensor = 0;
}

static void step_sim(void) {
    Rapier_World_step(g_world, g_queue);
    g_step_count++;
    Rapier_EventQueue_drainCollisionEvents(g_queue, g_col_cb);
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

    double sx = to_cx(-1.85), sy = to_cy(1.65), sw = 3.70 * 46.0, sh = 0.60 * 46.0;
    if (g_active_in_sensor > 0) {
        CanvasRenderingContext2D_set_fillStyle(g_ctx, "#efefef");
        CanvasRenderingContext2D_fillRect(g_ctx, sx, sy, sw, sh);
        CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
        CanvasRenderingContext2D_set_lineWidth(g_ctx, 2.0);
        CanvasRenderingContext2D_strokeRect(g_ctx, sx, sy, sw, sh);
    } else {
        CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#737373");
        CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.4);
        draw_segs(sx, sy, sx + sw, sy, 14);
        draw_segs(sx + sw, sy, sx + sw, sy + sh, 4);
        draw_segs(sx + sw, sy + sh, sx, sy + sh, 14);
        draw_segs(sx, sy + sh, sx, sy, 4);
    }

    char gcap[64];
    snprintf(gcap, sizeof gcap, "SENSOR GATE (%d inside)", g_active_in_sensor);
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#555555");
    CanvasRenderingContext2D_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasRenderingContext2D_fillText(g_ctx, gcap, sx + 8.0, sy + 17.0);

    draw_rot_box(0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, "#f7f7f7");
    draw_rot_box(-5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(-2.1, 4.2, 0.0, 0.0, sin(-0.18), cos(-0.18), 2.2, 0.18, "#e5e5e5");
    draw_rot_box(1.8, 2.6, 0.0, 0.0, sin(0.18), cos(0.18), 2.2, 0.18, "#e5e5e5");

    for (int i = 0; i < BODY_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_bodies[i]);
        Rapier_Rotation q = Rapier_RigidBody_rotation(g_bodies[i]);
        double px = Rapier_Vector_get_x(p), py = Rapier_Vector_get_y(p);
        double qx = Rapier_Rotation_get_x(q), qy = Rapier_Rotation_get_y(q), qz = Rapier_Rotation_get_z(q), qw = Rapier_Rotation_get_w(q);
        if (IS_BOX[i]) {
            draw_rot_box(px, py, qx, qy, qz, qw, 0.28, 0.28, "#d4d4d4");
        } else {
            double cx = to_cx(px), cy = to_cy(py);
            CanvasRenderingContext2D_set_fillStyle(g_ctx, "#262626");
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_arc(g_ctx, cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            CanvasRenderingContext2D_fill(g_ctx);
            CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
            CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_arc(g_ctx, cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            CanvasRenderingContext2D_stroke(g_ctx);
        }
    }

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "EventQueue \xC2\xB7 solid_contacts=%d \xC2\xB7 sensor_crossings=%d \xC2\xB7 inside=%d \xC2\xB7 step=%d%s",
        g_solid_contacts, g_sensor_crossings, g_active_in_sensor, g_step_count,
        g_paused ? " (paused)" : " (running)"
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
        Rapier_Vector3 v = Rapier_Vector3_new(0.4, 0.0, 0.0);
        for (int i = 0; i < BODY_COUNT; i++) {
            Rapier_Vector3 p = Rapier_Vector3_new(-3.7 + (double)i * 0.52, 5.4 + (double)(i % 3) * 0.58, 0.0);
            Rapier_RigidBody_setTranslation(g_bodies[i], p, true);
            Rapier_RigidBody_setLinvel(g_bodies[i], v, true);
        }
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 25; i++) step_sim();
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
    g_col_cb = Rapier_DrainCollisionEventsFn_from(on_collision, NULL);
    js_retain(g_col_cb);

    build_scene();
    for (int i = 0; i < 40; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-drop", "btn-pause", "btn-reset" };
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
