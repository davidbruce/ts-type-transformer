#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)
#define BODY_COUNT 6

static double approx_sin(double x) {
    while (x > PI) x -= TWO_PI;
    while (x < -PI) x += TWO_PI;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0 * (1.0 - x2 / 72.0))));
}
static double approx_cos(double x) { return approx_sin(x + PI * 0.5); }

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

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static FrameRequestCallback g_raf_cb;
static EventQueue_drainCollisionEvents_f g_col_cb;

static string_or_CanvasGradient_or_CanvasPattern g_c_white;
static string_or_CanvasGradient_or_CanvasPattern g_c_grid;
static string_or_CanvasGradient_or_CanvasPattern g_c_f7;
static string_or_CanvasGradient_or_CanvasPattern g_c_e5;
static string_or_CanvasGradient_or_CanvasPattern g_c_d4;
static string_or_CanvasGradient_or_CanvasPattern g_c_73;
static string_or_CanvasGradient_or_CanvasPattern g_c_55;
static string_or_CanvasGradient_or_CanvasPattern g_c_26;
static string_or_CanvasGradient_or_CanvasPattern g_c_11;

static World g_world = { 0 };
static EventQueue g_queue = { 0 };
static RigidBody g_bodies[BODY_COUNT];
static const bool IS_BOX[BODY_COUNT] = { false, false, true, false, true, false };
static double g_sensor_handle = 0.0;
static int g_step_count = 0;
static int g_solid_contacts = 0;
static int g_sensor_crossings = 0;
static int g_active_in_sensor = 0;
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

static void draw_segs(double x1, double y1, double x2, double y2, int n) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < n; i += 2) {
        double t0 = (double)i / (double)n;
        double t1 = (double)(i + 1) / (double)n;
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_moveTo(g_ctx, x1 + dx * t0, y1 + dy * t0);
        CanvasPath_lineTo(g_ctx, x1 + dx * t1, y1 + dy * t1);
        CanvasDrawPath_stroke(g_ctx);
    }
}

static void add_ramp(double cx, double cy, double hx, double hy, double theta) {
    Quaternion q = Quaternion_new(0.0, 0.0, approx_sin(theta * 0.5), approx_cos(theta * 0.5));
    RigidBodyDesc d = RigidBodyDesc_setRotation(RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), cx, cy, 0.0), q);
    js_release(q);
    RigidBody rb = World_createRigidBody(g_world, d);
    ColliderDesc cd = ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(hx, hy, 1.0), 0.15), 0.35);
    World_createCollider(g_world, cd, rb);
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
    if (g_queue.h != 0) {
        EventQueue_free(g_queue);
        js_release(g_queue);
    }
    if (g_world.h != 0) {
        World_free(g_world);
        js_release(g_world);
    }
    Vector3 gv = Vector3_new(0.0, -9.81, 0.0);
    g_world = World_new(gv);
    js_release(gv);
    g_queue = EventQueue_new(true);

    add_ramp(0.0, -0.35, 6.8, 0.35, 0.0);
    add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36);
    add_ramp(1.8, 2.6, 2.2, 0.18, 0.36);

    RigidBody sbody = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, 1.35, 0.0));
    ColliderDesc scd = ColliderDesc_setActiveEvents(ColliderDesc_setSensor(ColliderDesc_cuboid(1.85, 0.30, 1.2), true), ActiveEvents_COLLISION_EVENTS);
    Collider sc = World_createCollider(g_world, scd, sbody);
    g_sensor_handle = Collider_get_handle(sc);

    for (int i = 0; i < BODY_COUNT; i++) {
        double x = -3.7 + (double)i * 0.52;
        double y = 5.4 + (double)(i % 3) * 0.58;
        g_bodies[i] = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), x, y, 0.0));
        ColliderDesc cd = IS_BOX[i] ? ColliderDesc_cuboid(0.28, 0.28, 0.28) : ColliderDesc_ball(0.32);
        cd = ColliderDesc_setActiveEvents(ColliderDesc_setFriction(ColliderDesc_setRestitution(cd, 0.35), 0.25), ActiveEvents_COLLISION_EVENTS);
        World_createCollider(g_world, cd, g_bodies[i]);
    }

    g_step_count = 0;
    g_solid_contacts = 0;
    g_sensor_crossings = 0;
    g_active_in_sensor = 0;
}

static void step_sim(void) {
    World_step(g_world, g_queue);
    g_step_count++;
    EventQueue_drainCollisionEvents(g_queue, g_col_cb);
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

    double sx = to_cx(-1.85), sy = to_cy(1.65), sw = 3.70 * 46.0, sh = 0.60 * 46.0;
    if (g_active_in_sensor > 0) {
        CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_grid);
        CanvasRect_fillRect(g_ctx, sx, sy, sw, sh);
        CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
        CanvasPathDrawingStyles_set_lineWidth(g_ctx, 2.0);
        CanvasRect_strokeRect(g_ctx, sx, sy, sw, sh);
    } else {
        CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_73);
        CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.4);
        draw_segs(sx, sy, sx + sw, sy, 14);
        draw_segs(sx + sw, sy, sx + sw, sy + sh, 4);
        draw_segs(sx + sw, sy + sh, sx, sy + sh, 14);
        draw_segs(sx, sy + sh, sx, sy, 4);
    }

    StrBuf gcap = { .pos = 0 };
    sb_str(&gcap, "SENSOR GATE (");
    sb_int(&gcap, g_active_in_sensor);
    sb_str(&gcap, " inside)");
    gcap.buf[gcap.pos] = 0;
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_55);
    CanvasTextDrawingStyles_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasText_fillText(g_ctx, gcap.buf, sx + 8.0, sy + 17.0);

    draw_rot_box(0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, g_c_f7);
    draw_rot_box(-5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, g_c_grid);
    draw_rot_box(5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, g_c_grid);
    draw_rot_box(-2.1, 4.2, 0.0, 0.0, approx_sin(-0.18), approx_cos(-0.18), 2.2, 0.18, g_c_e5);
    draw_rot_box(1.8, 2.6, 0.0, 0.0, approx_sin(0.18), approx_cos(0.18), 2.2, 0.18, g_c_e5);

    for (int i = 0; i < BODY_COUNT; i++) {
        Vector p = RigidBody_translation(g_bodies[i]);
        Rotation q = RigidBody_rotation(g_bodies[i]);
        double px = Vector_get_x(p), py = Vector_get_y(p);
        double qx = Rotation_get_x(q), qy = Rotation_get_y(q), qz = Rotation_get_z(q), qw = Rotation_get_w(q);
        js_release(p); js_release(q);
        if (IS_BOX[i]) {
            draw_rot_box(px, py, qx, qy, qz, qw, 0.28, 0.28, g_c_d4);
        } else {
            double cx = to_cx(px), cy = to_cy(py);
            CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_26);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_arc(g_ctx, cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            CanvasDrawPath_fill(g_ctx);
            CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
            CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_arc(g_ctx, cx, cy, 0.32 * 46.0, 0.0, TWO_PI);
            CanvasDrawPath_stroke(g_ctx);
        }
    }

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "EventQueue \xC2\xB7 solid_contacts=");
    sb_int(&sb, g_solid_contacts);
    sb_str(&sb, " \xC2\xB7 sensor_crossings=");
    sb_int(&sb, g_sensor_crossings);
    sb_str(&sb, " \xC2\xB7 inside=");
    sb_int(&sb, g_active_in_sensor);
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

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) {
        Vector3 v = Vector3_new(0.4, 0.0, 0.0);
        for (int i = 0; i < BODY_COUNT; i++) {
            Vector3 p = Vector3_new(-3.7 + (double)i * 0.52, 5.4 + (double)(i % 3) * 0.58, 0.0);
            RigidBody_setTranslation(g_bodies[i], p, true);
            RigidBody_setLinvel(g_bodies[i], v, true);
            js_release(p);
        }
        js_release(v);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        Node_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        Node_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 25; i++) step_sim();
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
    g_col_cb = EventQueue_drainCollisionEvents_f_from(on_collision, 0);

    g_c_white = mk_color("#ffffff");
    g_c_grid = mk_color("#efefef");
    g_c_f7 = mk_color("#f7f7f7");
    g_c_e5 = mk_color("#e5e5e5");
    g_c_d4 = mk_color("#d4d4d4");
    g_c_73 = mk_color("#737373");
    g_c_55 = mk_color("#555555");
    g_c_26 = mk_color("#262626");
    g_c_11 = mk_color("#111111");

    build_scene();
    for (int i = 0; i < 40; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-drop", "btn-pause", "btn-reset" };
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
