#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)
#define RAY_COUNT 37
#define MAX_TOI 16.0
#define EMITTER_X -5.6
#define EMITTER_Y 4.6

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
static HTMLElement g_sweep_btn;
static FrameRequestCallback g_raf_cb;
static World g_world;

static string_or_CanvasGradient_or_CanvasPattern g_c_white;
static string_or_CanvasGradient_or_CanvasPattern g_c_grid;
static string_or_CanvasGradient_or_CanvasPattern g_c_f7;
static string_or_CanvasGradient_or_CanvasPattern g_c_e5;
static string_or_CanvasGradient_or_CanvasPattern g_c_d4;
static string_or_CanvasGradient_or_CanvasPattern g_c_73;
static string_or_CanvasGradient_or_CanvasPattern g_c_11;

static bool g_auto_sweep = true;
static double g_phase = 0.0;
static double g_manual_offset = 0.0;

static string_or_CanvasGradient_or_CanvasPattern mk_color(const char* hex) {
    return string_or_CanvasGradient_or_CanvasPattern_of_string(JsString_from(hex));
}

static void draw_box(double cx, double cy, double theta, double hx, double hy, string_or_CanvasGradient_or_CanvasPattern fill) {
    double c = approx_cos(theta), s = approx_sin(theta);
    double xs[4] = {
        to_cx(cx + c * (-hx) - s * (-hy)),
        to_cx(cx + c * hx - s * (-hy)),
        to_cx(cx + c * hx - s * hy),
        to_cx(cx + c * (-hx) - s * hy)
    };
    double ys[4] = {
        to_cy(cy + s * (-hx) + c * (-hy)),
        to_cy(cy + s * hx + c * (-hy)),
        to_cy(cy + s * hx + c * hy),
        to_cy(cy + s * (-hx) + c * hy)
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

static void draw_circle(double wx, double wy, double r, string_or_CanvasGradient_or_CanvasPattern fill) {
    double cx = to_cx(wx), cy = to_cy(wy), rp = r * 46.0;
    CanvasFillStrokeStyles_set_fillStyle(g_ctx, fill);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, cx, cy, rp, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, cx, cy, rp, 0.0, TWO_PI);
    CanvasDrawPath_stroke(g_ctx);
}

static void add_box(double cx, double cy, double theta, double hx, double hy) {
    Quaternion q = Quaternion_new(0.0, 0.0, approx_sin(theta * 0.5), approx_cos(theta * 0.5));
    RigidBodyDesc d = RigidBodyDesc_setRotation(RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), cx, cy, 0.0), q);
    js_release(q);
    RigidBody rb = World_createRigidBody(g_world, d);
    World_createCollider(g_world, ColliderDesc_cuboid(hx, hy, 1.0), rb);
}

static void add_sphere(double cx, double cy, double r) {
    RigidBody rb = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), cx, cy, 0.0));
    World_createCollider(g_world, ColliderDesc_ball(r), rb);
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

    draw_box(0.0, -0.35, 0.0, 7.0, 0.35, g_c_f7);
    draw_box(-1.6, 0.85, 0.0, 0.70, 0.85, g_c_grid);
    draw_box(2.2, 1.35, 0.0, 0.65, 1.35, g_c_grid);
    draw_box(0.2, 1.2, -0.42, 1.3, 0.18, g_c_d4);
    draw_box(0.4, 4.4, 0.0, 1.9, 0.22, g_c_grid);
    draw_circle(-2.5, 3.6, 0.65, g_c_d4);
    draw_circle(4.2, 3.2, 0.75, g_c_d4);

    double base_angle = -0.35 + g_manual_offset + 0.28 * approx_sin(g_phase);
    double ox = to_cx(EMITTER_X), oy = to_cy(EMITTER_Y);
    Vector3 origin = Vector3_new(EMITTER_X, EMITTER_Y, 0.0);

    int hits = 0;
    double min_toi = MAX_TOI;
    for (int i = 0; i < RAY_COUNT; i++) {
        double frac = (double)i / (double)(RAY_COUNT - 1) - 0.5;
        double angle = base_angle + frac * 1.35;
        double dx = approx_cos(angle), dy = approx_sin(angle);
        Vector3 dir = Vector3_new(dx, dy, 0.0);
        Ray ray = Ray_new(origin, dir);
        js_release(dir);

        RayColliderIntersection hit = World_castRayAndGetNormal(
            g_world, ray, MAX_TOI, true, js_none_double(), js_none_double(),
            (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
        if (hit.h != 0) {
            hits++;
            double toi = RayColliderIntersection_get_timeOfImpact(hit);
            if (toi < min_toi) min_toi = toi;
            Vector pt = Ray_pointAt(ray, toi);
            double ptx = Vector_get_x(pt), pty = Vector_get_y(pt);
            js_release(pt);
            double hx = to_cx(ptx), hy = to_cy(pty);

            CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_73);
            CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.0);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_moveTo(g_ctx, ox, oy);
            CanvasPath_lineTo(g_ctx, hx, hy);
            CanvasDrawPath_stroke(g_ctx);

            Vector n = RayColliderIntersection_get_normal(hit);
            double nx = Vector_get_x(n), ny = Vector_get_y(n);
            js_release(n);
            CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_11);
            CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.4);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_moveTo(g_ctx, hx, hy);
            CanvasPath_lineTo(g_ctx, to_cx(ptx + nx * 0.45), to_cy(pty + ny * 0.45));
            CanvasDrawPath_stroke(g_ctx);

            CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_11);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_arc(g_ctx, hx, hy, 2.8, 0.0, TWO_PI);
            CanvasDrawPath_fill(g_ctx);
            js_release(hit);
        } else {
            CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_e5);
            CanvasPathDrawingStyles_set_lineWidth(g_ctx, 0.9);
            CanvasDrawPath_beginPath(g_ctx);
            CanvasPath_moveTo(g_ctx, ox, oy);
            CanvasPath_lineTo(g_ctx, to_cx(EMITTER_X + dx * MAX_TOI), to_cy(EMITTER_Y + dy * MAX_TOI));
            CanvasDrawPath_stroke(g_ctx);
        }
        js_release(ray);
    }
    js_release(origin);

    CanvasFillStrokeStyles_set_fillStyle(g_ctx, g_c_11);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, ox, oy, 6.0, 0.0, TWO_PI);
    CanvasDrawPath_fill(g_ctx);
    CanvasFillStrokeStyles_set_strokeStyle(g_ctx, g_c_white);
    CanvasPathDrawingStyles_set_lineWidth(g_ctx, 1.5);
    CanvasDrawPath_beginPath(g_ctx);
    CanvasPath_arc(g_ctx, ox, oy, 3.0, 0.0, TWO_PI);
    CanvasDrawPath_stroke(g_ctx);

    int deg = (int)(base_angle * 180.0 / PI + (base_angle >= 0.0 ? 0.5 : -0.5));
    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "37 rays \xC2\xB7 hits=");
    sb_int(&sb, hits);
    sb_str(&sb, "/37 \xC2\xB7 nearest_toi=");
    sb_f2(&sb, min_toi);
    sb_str(&sb, " m \xC2\xB7 center=");
    sb_int(&sb, deg);
    sb_str(&sb, g_auto_sweep ? " deg (auto)" : " deg (manual)");
    sb.buf[sb.pos] = 0;
    Node_set_textContent(g_meta, sb.buf);
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    if (g_auto_sweep) {
        g_phase += 0.025;
        render();
    }
    requestAnimationFrame(g_raf_cb);
}

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) {
        g_auto_sweep = !g_auto_sweep;
        Node_set_textContent(g_sweep_btn, g_auto_sweep ? "Sweep: Auto" : "Sweep: Manual");
        render();
    } else if (id == 1) {
        g_manual_offset += PI / 12.0;
        if (g_manual_offset > 0.8) g_manual_offset = -0.6;
        render();
    } else {
        g_auto_sweep = true;
        g_phase = 0.0;
        g_manual_offset = 0.0;
        Node_set_textContent(g_sweep_btn, "Sweep: Auto");
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
    g_sweep_btn = Document_getElementById(doc, "btn-sweep");

    g_c_white = mk_color("#ffffff");
    g_c_grid = mk_color("#efefef");
    g_c_f7 = mk_color("#f7f7f7");
    g_c_e5 = mk_color("#e5e5e5");
    g_c_d4 = mk_color("#d4d4d4");
    g_c_73 = mk_color("#737373");
    g_c_11 = mk_color("#111111");

    Vector3 gv = Vector3_new(0.0, 0.0, 0.0);
    g_world = World_new(gv);
    js_release(gv);

    add_box(0.0, -0.35, 0.0, 7.0, 0.35);
    add_box(-1.6, 0.85, 0.0, 0.70, 0.85);
    add_box(2.2, 1.35, 0.0, 0.65, 1.35);
    add_box(0.2, 1.2, -0.42, 1.3, 0.18);
    add_box(0.4, 4.4, 0.0, 1.9, 0.22);
    add_sphere(-2.5, 3.6, 0.65);
    add_sphere(4.2, 3.2, 0.75);
    World_step(g_world);

    render();

    static const char* ids[3] = { "btn-sweep", "btn-angle", "btn-reset" };
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
