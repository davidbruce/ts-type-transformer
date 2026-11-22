#include <stdio.h>
#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

static const double TWO_PI = 6.28318530717958647692;
#define TRAIL_LEN 36
#define LINK_HX 0.14
#define LINK_HY 0.62

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;

static Rapier_World g_world = { 0 };
static Rapier_RigidBody g_chain[3];
static Rapier_RigidBody g_arm = { 0 };
static Rapier_RigidBody g_bob = { 0 };
static double g_ax[TRAIL_LEN], g_ay[TRAIL_LEN], g_bx[TRAIL_LEN], g_by[TRAIL_LEN];
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

static void draw_pivot(double wx, double wy) {
    double cx = to_cx(wx), cy = to_cy(wy);
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#ffffff");
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, cx, cy, 4.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.6);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, cx, cy, 4.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_stroke(g_ctx);
}

static void build_scene(void) {
    js_scope();
    for (int i = 0; i < 3; i++) {
        if (g_chain[i].h != 0) {
            js_release(g_chain[i]);
            g_chain[i].h = 0;
        }
    }
    if (g_arm.h != 0) { js_release(g_arm); g_arm.h = 0; }
    if (g_bob.h != 0) { js_release(g_bob); g_bob.h = 0; }
    if (g_world.h != 0) {
        Rapier_World_free(g_world);
        js_release(g_world);
    }
    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    Rapier_RigidBody gantry = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, 6.2, 0.0));
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_cuboid(6.0, 0.22, 0.6), gantry);
    Rapier_Vector3 z_axis = Rapier_Vector3_new(0.0, 0.0, 1.0);

    double cx[3] = { -2.8, -3.3, -3.8 }, cy[3] = { 5.4, 4.1, 2.8 };
    for (int i = 0; i < 3; i++) {
        g_chain[i] = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), cx[i], cy[i], 0.0));
        js_retain(g_chain[i]);
        Rapier_World_createCollider(g_world, Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_cuboid(LINK_HX, LINK_HY, 0.2), i == 2 ? 1.6 : 1.4), g_chain[i]);
    }
    Rapier_World_createImpulseJoint(g_world, Rapier_JointData_revolute(Rapier_Vector3_new(-2.2, -0.22, 0.0), Rapier_Vector3_new(0.0, LINK_HY + 0.06, 0.0), z_axis), gantry, g_chain[0], true);

    Rapier_Vector3 b1 = Rapier_Vector3_new(0.0, -LINK_HY - 0.06, 0.0), b2 = Rapier_Vector3_new(0.0, LINK_HY + 0.06, 0.0);
    Rapier_World_createImpulseJoint(g_world, Rapier_JointData_revolute(b1, b2, z_axis), g_chain[0], g_chain[1], true);
    Rapier_World_createImpulseJoint(g_world, Rapier_JointData_revolute(b1, b2, z_axis), g_chain[1], g_chain[2], true);

    Rapier_RigidBody_applyImpulse(g_chain[2], Rapier_Vector3_new(-1.4, 0.0, 0.0), true);

    g_arm = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), 3.1, 5.2, 0.0));
    js_retain(g_arm);
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_cuboid(0.14, 0.85, 0.2), 1.2), g_arm);
    g_bob = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), 3.8, 3.6, 0.0));
    js_retain(g_bob);
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_ball(0.42), 3.0), g_bob);

    Rapier_World_createImpulseJoint(g_world, Rapier_JointData_revolute(Rapier_Vector3_new(2.3, -0.22, 0.0), Rapier_Vector3_new(0.0, 0.88, 0.0), z_axis), gantry, g_arm, true);
    Rapier_World_createImpulseJoint(g_world, Rapier_JointData_revolute(Rapier_Vector3_new(0.0, -0.88, 0.0), Rapier_Vector3_new(0.0, 0.72, 0.0), z_axis), g_arm, g_bob, true);
    Rapier_RigidBody_applyImpulse(g_bob, Rapier_Vector3_new(1.8, 0.2, 0.0), true);

    g_step_count = 0;
    g_trail_count = 0;
}

static void step_sim(void) {
    js_scope();
    Rapier_World_step(g_world);
    g_step_count++;
    Rapier_Vector pa = Rapier_RigidBody_translation(g_chain[2]);
    Rapier_Vector pb = Rapier_RigidBody_translation(g_bob);
    int idx = (g_step_count - 1) % TRAIL_LEN;
    g_ax[idx] = Rapier_Vector_get_x(pa); g_ay[idx] = Rapier_Vector_get_y(pa);
    g_bx[idx] = Rapier_Vector_get_x(pb); g_by[idx] = Rapier_Vector_get_y(pb);
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
        CanvasRenderingContext2D_lineTo(g_ctx, x, 400.0);
        CanvasRenderingContext2D_stroke(g_ctx);
    }
    for (int gy = 0; gy <= 7; gy++) {
        double y = to_cy((double)gy);
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_moveTo(g_ctx, 0.0, y);
        CanvasRenderingContext2D_lineTo(g_ctx, 680.0, y);
        CanvasRenderingContext2D_stroke(g_ctx);
    }

    draw_rot_box(0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, "#efefef");

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#b5b5b5");
    for (int i = 0; i < g_trail_count; i++) {
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, to_cx(g_ax[i]), to_cy(g_ay[i]), 1.6, 0.0, TWO_PI);
        CanvasRenderingContext2D_fill(g_ctx);
        CanvasRenderingContext2D_beginPath(g_ctx);
        CanvasRenderingContext2D_arc(g_ctx, to_cx(g_bx[i]), to_cy(g_by[i]), 1.6, 0.0, TWO_PI);
        CanvasRenderingContext2D_fill(g_ctx);
    }

    double tip_x = 0.0, tip_y = 0.0;
    double pin_x[2] = { 0.0, 0.0 }, pin_y[2] = { 0.0, 0.0 };
    for (int i = 0; i < 3; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_chain[i]);
        Rapier_Rotation q = Rapier_RigidBody_rotation(g_chain[i]);
        double px = Rapier_Vector_get_x(p), py = Rapier_Vector_get_y(p);
        double qx = Rapier_Rotation_get_x(q), qy = Rapier_Rotation_get_y(q), qz = Rapier_Rotation_get_z(q), qw = Rapier_Rotation_get_w(q);
        draw_rot_box(px, py, qx, qy, qz, qw, LINK_HX, LINK_HY, (i == 2 ? "#d4d4d4" : "#f7f7f7"));
        if (i < 2) {
            double r01 = 2.0 * (qx * qy - qw * qz);
            double r11 = 1.0 - 2.0 * (qx * qx + qz * qz);
            pin_x[i] = px - r01 * (LINK_HY + 0.06);
            pin_y[i] = py - r11 * (LINK_HY + 0.06);
        } else {
            tip_x = px; tip_y = py;
        }
    }

    Rapier_Vector ap = Rapier_RigidBody_translation(g_arm);
    Rapier_Rotation aq = Rapier_RigidBody_rotation(g_arm);
    double apx = Rapier_Vector_get_x(ap), apy = Rapier_Vector_get_y(ap);
    double aqx = Rapier_Rotation_get_x(aq), aqy = Rapier_Rotation_get_y(aq), aqz = Rapier_Rotation_get_z(aq), aqw = Rapier_Rotation_get_w(aq);
    draw_rot_box(apx, apy, aqx, aqy, aqz, aqw, 0.14, 0.85, "#f7f7f7");

    Rapier_Vector bp = Rapier_RigidBody_translation(g_bob);
    double bpx = Rapier_Vector_get_x(bp), bpy = Rapier_Vector_get_y(bp);
    double bcx = to_cx(bpx), bcy = to_cy(bpy);
    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#262626");
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.6);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_stroke(g_ctx);

    draw_pivot(-2.2, 5.98);
    draw_pivot(pin_x[0], pin_y[0]);
    draw_pivot(pin_x[1], pin_y[1]);
    draw_pivot(2.3, 5.98);
    double arm_r01 = 2.0 * (aqx * aqy - aqw * aqz);
    double arm_r11 = 1.0 - 2.0 * (aqx * aqx + aqz * aqz);
    draw_pivot(apx - arm_r01 * 0.88, apy - arm_r11 * 0.88);

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#555555");
    CanvasRenderingContext2D_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasRenderingContext2D_fillText(g_ctx, "3-Link Revolute Chain", to_cx(-3.5), 36.0);
    CanvasRenderingContext2D_fillText(g_ctx, "2-Body Bob Pendulum", to_cx(1.1), 36.0);

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "5 revolute joints \xC2\xB7 chain_tip=(%.2f, %.2f) \xC2\xB7 bob=(%.2f, %.2f) \xC2\xB7 step=%d%s",
        tip_x, tip_y, bpx, bpy, g_step_count, g_paused ? " (paused)" : " (running)"
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
        Rapier_RigidBody_applyImpulse(g_chain[2], Rapier_Vector3_new(1.6, 0.3, 0.0), true);
        Rapier_RigidBody_applyImpulse(g_bob, Rapier_Vector3_new(-2.0, 0.4, 0.0), true);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 15; i++) step_sim();
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
    for (int i = 0; i < 20; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-kick", "btn-pause", "btn-reset" };
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
