#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

#define TWO_PI 6.28318530717958647692
#define TRAIL_LEN 36
#define LINK_HX 0.14
#define LINK_HY 0.62

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
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;

static string_or_CanvasGradient_or_CanvasPattern g_c_white;
static string_or_CanvasGradient_or_CanvasPattern g_c_grid;
static string_or_CanvasGradient_or_CanvasPattern g_c_f7;
static string_or_CanvasGradient_or_CanvasPattern g_c_d4;
static string_or_CanvasGradient_or_CanvasPattern g_c_b5;
static string_or_CanvasGradient_or_CanvasPattern g_c_55;
static string_or_CanvasGradient_or_CanvasPattern g_c_26;
static string_or_CanvasGradient_or_CanvasPattern g_c_11;

static World g_world = { 0 };
static RigidBody g_chain[3];
static RigidBody g_arm;
static RigidBody g_bob;
static double g_ax[TRAIL_LEN], g_ay[TRAIL_LEN], g_bx[TRAIL_LEN], g_by[TRAIL_LEN];
static int g_trail_count = 0;
static int g_step_count = 0;
static bool g_paused = false;

static string_or_CanvasGradient_or_CanvasPattern mk_color(const char* hex) {
    return string_or_CanvasGradient_or_CanvasPattern_of_string(JsString_from(hex));
}

static void draw_rot_box(double cx, double cy, double qx, double qy, double qz, double qw, double hx, double hy,
                         string_or_CanvasGradient_or_CanvasPattern fill) {
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

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, fill);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_moveTo(g_ctx, xs[0], ys[0]);
    for (int i = 1; i < 4; i++) CanvasPath_lineTo(g_ctx, xs[i], ys[i]);
    CanvasPath_closePath(g_ctx);
    CanvasDrawPath_fill(g_ctx);

    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_moveTo(g_ctx, xs[0], ys[0]);
    for (int i = 1; i < 4; i++) CanvasPath_lineTo(g_ctx, xs[i], ys[i]);
    CanvasPath_closePath(g_ctx);
    CanvasDrawPath_stroke(g_ctx);
}

static void draw_pivot(double wx, double wy) {
    double cx = to_cx(wx), cy = to_cy(wy);
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_white);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, cx, cy, 4.0, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.6);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, cx, cy, 4.0, 0.0, TWO_PI);
    CanvasDrawPath_stroke(g_ctx);
}

static void build_scene(void) {
    if (g_world.h != 0) {
        World_free(g_world);
        js_release(g_world);
    }
    Vector3 gv = Vector3_new(0.0, -9.81, 0.0);
    g_world = World_new(gv);
    js_release(gv);

    RigidBody gantry = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, 6.2, 0.0));
    World_createCollider(g_world, ColliderDesc_cuboid(6.0, 0.22, 0.6), gantry);
    Vector3 z_axis = Vector3_new(0.0, 0.0, 1.0);

    double cx[3] = { -2.8, -3.3, -3.8 }, cy[3] = { 5.4, 4.1, 2.8 };
    for (int i = 0; i < 3; i++) {
        g_chain[i] = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), cx[i], cy[i], 0.0));
        World_createCollider(g_world, ColliderDesc_setDensity(ColliderDesc_cuboid(LINK_HX, LINK_HY, 0.2), i == 2 ? 1.6 : 1.4), g_chain[i]);
    }
    Vector3 a1 = Vector3_new(-2.2, -0.22, 0.0), a2 = Vector3_new(0.0, LINK_HY + 0.06, 0.0);
    World_createImpulseJoint(g_world, JointData_revolute(a1, a2, z_axis), gantry, g_chain[0], true);
    js_release(a1); js_release(a2);

    Vector3 b1 = Vector3_new(0.0, -LINK_HY - 0.06, 0.0), b2 = Vector3_new(0.0, LINK_HY + 0.06, 0.0);
    World_createImpulseJoint(g_world, JointData_revolute(b1, b2, z_axis), g_chain[0], g_chain[1], true);
    World_createImpulseJoint(g_world, JointData_revolute(b1, b2, z_axis), g_chain[1], g_chain[2], true);
    js_release(b1); js_release(b2);

    Vector3 kick_a = Vector3_new(-1.4, 0.0, 0.0);
    RigidBody_applyImpulse(g_chain[2], kick_a, true);
    js_release(kick_a);

    g_arm = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), 3.1, 5.2, 0.0));
    World_createCollider(g_world, ColliderDesc_setDensity(ColliderDesc_cuboid(0.14, 0.85, 0.2), 1.2), g_arm);
    g_bob = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), 3.8, 3.6, 0.0));
    World_createCollider(g_world, ColliderDesc_setDensity(ColliderDesc_ball(0.42), 3.0), g_bob);

    Vector3 c1 = Vector3_new(2.3, -0.22, 0.0), c2 = Vector3_new(0.0, 0.88, 0.0);
    World_createImpulseJoint(g_world, JointData_revolute(c1, c2, z_axis), gantry, g_arm, true);
    js_release(c1); js_release(c2);

    Vector3 d1 = Vector3_new(0.0, -0.88, 0.0), d2 = Vector3_new(0.0, 0.72, 0.0);
    World_createImpulseJoint(g_world, JointData_revolute(d1, d2, z_axis), g_arm, g_bob, true);
    js_release(d1); js_release(d2);
    js_release(z_axis);

    Vector3 kick_b = Vector3_new(1.8, 0.2, 0.0);
    RigidBody_applyImpulse(g_bob, kick_b, true);
    js_release(kick_b);

    g_step_count = 0;
    g_trail_count = 0;
}

static void step_sim(void) {
    World_step(g_world);
    g_step_count++;
    Vector pa = RigidBody_translation(g_chain[2]);
    Vector pb = RigidBody_translation(g_bob);
    int idx = (g_step_count - 1) % TRAIL_LEN;
    g_ax[idx] = Vector_get_x(pa); g_ay[idx] = Vector_get_y(pa);
    g_bx[idx] = Vector_get_x(pb); g_by[idx] = Vector_get_y(pb);
    js_release(pa); js_release(pb);
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
        CanvasPath_lineTo(g_ctx, x, 400.0);
        CanvasDrawPath_stroke(g_ctx);
    }
    for (int gy = 0; gy <= 7; gy++) {
        double y = to_cy((double)gy);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, 0.0, y);
        CanvasPath_lineTo(g_ctx, 680.0, y);
        CanvasDrawPath_stroke(g_ctx);
    }

    draw_rot_box(0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, g_c_grid);

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_b5);
    for (int i = 0; i < g_trail_count; i++) {
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, to_cx(g_ax[i]), to_cy(g_ay[i]), 1.6, 0.0, TWO_PI);
        CanvasDrawPath_fill(g_ctx);
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, to_cx(g_bx[i]), to_cy(g_by[i]), 1.6, 0.0, TWO_PI);
        CanvasDrawPath_fill(g_ctx);
    }

    double tip_x = 0.0, tip_y = 0.0;
    double pin_x[2], pin_y[2];
    for (int i = 0; i < 3; i++) {
        Vector p = RigidBody_translation(g_chain[i]);
        Rotation q = RigidBody_rotation(g_chain[i]);
        double px = Vector_get_x(p), py = Vector_get_y(p);
        double qx = Rotation_get_x(q), qy = Rotation_get_y(q), qz = Rotation_get_z(q), qw = Rotation_get_w(q);
        js_release(p); js_release(q);
        draw_rot_box(px, py, qx, qy, qz, qw, LINK_HX, LINK_HY, i == 2 ? g_c_d4 : g_c_f7);
        if (i < 2) {
            double r01 = 2.0 * (qx * qy - qw * qz);
            double r11 = 1.0 - 2.0 * (qx * qx + qz * qz);
            pin_x[i] = px - r01 * (LINK_HY + 0.06);
            pin_y[i] = py - r11 * (LINK_HY + 0.06);
        } else {
            tip_x = px; tip_y = py;
        }
    }

    Vector ap = RigidBody_translation(g_arm);
    Rotation aq = RigidBody_rotation(g_arm);
    double apx = Vector_get_x(ap), apy = Vector_get_y(ap);
    double aqx = Rotation_get_x(aq), aqy = Rotation_get_y(aq), aqz = Rotation_get_z(aq), aqw = Rotation_get_w(aq);
    js_release(ap); js_release(aq);
    draw_rot_box(apx, apy, aqx, aqy, aqz, aqw, 0.14, 0.85, g_c_f7);

    Vector bp = RigidBody_translation(g_bob);
    double bpx = Vector_get_x(bp), bpy = Vector_get_y(bp);
    js_release(bp);
    double bcx = to_cx(bpx), bcy = to_cy(bpy);
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_26);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.6);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI);
    CanvasDrawPath_stroke(g_ctx);

    draw_pivot(-2.2, 5.98);
    draw_pivot(pin_x[0], pin_y[0]);
    draw_pivot(pin_x[1], pin_y[1]);
    draw_pivot(2.3, 5.98);
    double arm_r01 = 2.0 * (aqx * aqy - aqw * aqz);
    double arm_r11 = 1.0 - 2.0 * (aqx * aqx + aqz * aqz);
    draw_pivot(apx - arm_r01 * 0.88, apy - arm_r11 * 0.88);

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_55);
    CanvasTextDrawingStyles_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasText_fillText(g_ctx, "3-Link Revolute Chain", to_cx(-3.5), 36.0);
    CanvasText_fillText(g_ctx, "2-Body Bob Pendulum", to_cx(1.1), 36.0);

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "5 revolute joints \xC2\xB7 chain_tip=(");
    sb_f2(&sb, tip_x); sb_str(&sb, ", "); sb_f2(&sb, tip_y);
    sb_str(&sb, ") \xC2\xB7 bob=(");
    sb_f2(&sb, bpx); sb_str(&sb, ", "); sb_f2(&sb, bpy);
    sb_str(&sb, ") \xC2\xB7 step=");
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

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) {
        Vector3 k1 = Vector3_new(1.6, 0.3, 0.0);
        Vector3 k2 = Vector3_new(-2.0, 0.4, 0.0);
        RigidBody_applyImpulse(g_chain[2], k1, true);
        RigidBody_applyImpulse(g_bob, k2, true);
        js_release(k1); js_release(k2);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        Node_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        Node_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 15; i++) step_sim();
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
    g_pause_btn = Document_getElementById(doc, "btn-pause");

    g_c_white = mk_color("#ffffff");
    g_c_grid = mk_color("#efefef");
    g_c_f7 = mk_color("#f7f7f7");
    g_c_d4 = mk_color("#d4d4d4");
    g_c_b5 = mk_color("#b5b5b5");
    g_c_55 = mk_color("#555555");
    g_c_26 = mk_color("#262626");
    g_c_11 = mk_color("#111111");

    build_scene();
    for (int i = 0; i < 20; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-kick", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
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
