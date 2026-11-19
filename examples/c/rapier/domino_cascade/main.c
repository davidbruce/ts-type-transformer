#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

#define TWO_PI 6.28318530717958647692
#define DOMINO_COUNT 9
#define DOM_HX 0.13
#define DOM_HY 0.85
#define TRAIL_LEN 22

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }
static double abs_d(double v) { return v < 0.0 ? -v : v; }

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
static string_or_CanvasGradient_or_CanvasPattern g_c_e5;
static string_or_CanvasGradient_or_CanvasPattern g_c_d4;
static string_or_CanvasGradient_or_CanvasPattern g_c_b5;
static string_or_CanvasGradient_or_CanvasPattern g_c_55;
static string_or_CanvasGradient_or_CanvasPattern g_c_26;
static string_or_CanvasGradient_or_CanvasPattern g_c_11;

static World g_world = { 0 };
static RigidBody g_dominoes[DOMINO_COUNT];
static RigidBody g_striker;
static double g_trail_x[TRAIL_LEN];
static double g_trail_y[TRAIL_LEN];
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

static void build_scene(void) {
    if (g_world.h != 0) {
        World_free(g_world);
        js_release(g_world);
    }
    Vector3 gv = Vector3_new(0.0, -9.81, 0.0);
    g_world = World_new(gv);
    js_release(gv);

    RigidBody floor = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, -0.35, 0.0));
    World_createCollider(g_world, ColliderDesc_setFriction(ColliderDesc_cuboid(7.2, 0.35, 2.0), 0.8), floor);

    for (int i = 0; i < DOMINO_COUNT; i++) {
        double x = -2.4 + (double)i * 0.88;
        g_dominoes[i] = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), x, DOM_HY, 0.0));
        ColliderDesc cd = ColliderDesc_setDensity(ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(DOM_HX, DOM_HY, 0.35), 0.55), 0.15), 1.0);
        World_createCollider(g_world, cd, g_dominoes[i]);
    }

    g_striker = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), -5.3, 1.05, 0.0));
    ColliderDesc bcd = ColliderDesc_setFriction(ColliderDesc_setRestitution(ColliderDesc_setDensity(ColliderDesc_ball(0.40), 2.8), 0.45), 0.25);
    World_createCollider(g_world, bcd, g_striker);
    Vector3 imp = Vector3_new(4.8, 0.2, 0.0);
    RigidBody_applyImpulse(g_striker, imp, true);
    js_release(imp);

    g_step_count = 0;
    g_trail_count = 0;
}

static void step_sim(void) {
    World_step(g_world);
    g_step_count++;
    Vector sp = RigidBody_translation(g_striker);
    int idx = (g_step_count - 1) % TRAIL_LEN;
    g_trail_x[idx] = Vector_get_x(sp);
    g_trail_y[idx] = Vector_get_y(sp);
    js_release(sp);
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
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_moveTo(g_ctx, 0.0, 350.0);
    CanvasPath_lineTo(g_ctx, 680.0, 350.0);
    CanvasDrawPath_stroke(g_ctx);

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_b5);
    for (int i = 0; i < g_trail_count; i++) {
        CanvasDrawPath_beginPath(g_ctx);
        CanvasPath_arc(g_ctx, to_cx(g_trail_x[i]), to_cy(g_trail_y[i]), 2.0, 0.0, TWO_PI);
        CanvasDrawPath_fill(g_ctx);
    }

    int toppled = 0;
    for (int i = 0; i < DOMINO_COUNT; i++) {
        Vector p = RigidBody_translation(g_dominoes[i]);
        Rotation q = RigidBody_rotation(g_dominoes[i]);
        double px = Vector_get_x(p), py = Vector_get_y(p);
        double qx = Rotation_get_x(q), qy = Rotation_get_y(q), qz = Rotation_get_z(q), qw = Rotation_get_w(q);
        js_release(p);
        js_release(q);
        bool tilted = abs_d(qz) > 0.25;
        if (tilted) toppled++;
        draw_rot_box(px, py, qx, qy, qz, qw, DOM_HX, DOM_HY, tilted ? g_c_d4 : g_c_f7);
    }

    Vector sp = RigidBody_translation(g_striker);
    double spx = Vector_get_x(sp), spy = Vector_get_y(sp);
    js_release(sp);
    double scx = to_cx(spx), scy = to_cy(spy);

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_26);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.6);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, scx, scy, 0.40 * 46.0, 0.0, TWO_PI);
    CanvasDrawPath_stroke(g_ctx);
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_e5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, scx - 5.0, scy - 5.0, 2.5, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);

    StrBuf cap = { .pos = 0 };
    sb_str(&cap, "Dominoes toppled: ");
    sb_int(&cap, toppled);
    sb_str(&cap, " / 9");
    cap.buf[cap.pos] = 0;
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_55);
    CanvasTextDrawingStyles_set_font(g_ctx, "11px ui-monospace, monospace");
    CanvasText_fillText(g_ctx, cap.buf, 18.0, 24.0);

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "9 cuboids + striker sphere \xC2\xB7 toppled=");
    sb_int(&sb, toppled);
    sb_str(&sb, "/9 \xC2\xB7 striker_x=");
    sb_f2(&sb, spx);
    sb_str(&sb, " m \xC2\xB7 step=");
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
        Vector3 imp = Vector3_new(3.5, 0.6, 0.0);
        RigidBody_applyImpulse(g_striker, imp, true);
        js_release(imp);
        render();
    } else if (id == 1) {
        g_paused = !g_paused;
        Node_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        render();
    } else {
        g_paused = false;
        Node_set_textContent(g_pause_btn, "Pause");
        build_scene();
        for (int i = 0; i < 12; i++) step_sim();
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
    g_c_e5 = mk_color("#e5e5e5");
    g_c_d4 = mk_color("#d4d4d4");
    g_c_b5 = mk_color("#b5b5b5");
    g_c_55 = mk_color("#555555");
    g_c_26 = mk_color("#262626");
    g_c_11 = mk_color("#111111");

    build_scene();
    for (int i = 0; i < 22; i++) step_sim();
    render();

    static const char* ids[3] = { "btn-push", "btn-pause", "btn-reset" };
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
