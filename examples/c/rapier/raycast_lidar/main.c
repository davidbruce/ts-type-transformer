#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"

static const double PI = 3.14159265358979323846;
static const double TWO_PI = 6.28318530717958647692;
#define RAY_COUNT 37
#define MAX_TOI 16.0
#define EMITTER_X -5.6
#define EMITTER_Y 4.6

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

static CanvasRenderingContext2D g_ctx;
static HTMLElement g_meta;
static HTMLElement g_sweep_btn;
static FrameRequestCallback g_raf_cb;
static Rapier_World g_world;

static bool g_auto_sweep = true;
static double g_phase = 0.0;
static double g_manual_offset = 0.0;

static void draw_box(double cx, double cy, double theta, double hx, double hy, const char* fill) {
    double c = cos(theta), s = sin(theta);
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

static void draw_circle(double wx, double wy, double r, const char* fill) {
    double cx = to_cx(wx), cy = to_cy(wy), rp = r * 46.0;
    CanvasRenderingContext2D_set_fillStyle(g_ctx, fill);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, cx, cy, rp, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, cx, cy, rp, 0.0, TWO_PI);
    CanvasRenderingContext2D_stroke(g_ctx);
}

static void add_box(double cx, double cy, double theta, double hx, double hy) {
    Rapier_Quaternion q = Rapier_Quaternion_new(0.0, 0.0, sin(theta * 0.5), cos(theta * 0.5));
    Rapier_RigidBodyDesc d = Rapier_RigidBodyDesc_setRotation(Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), cx, cy, 0.0), q);
    Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, d);
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_cuboid(hx, hy, 1.0), rb);
}

static void add_sphere(double cx, double cy, double r) {
    Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), cx, cy, 0.0));
    Rapier_World_createCollider(g_world, Rapier_ColliderDesc_ball(r), rb);
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

    draw_box(0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7");
    draw_box(-1.6, 0.85, 0.0, 0.70, 0.85, "#efefef");
    draw_box(2.2, 1.35, 0.0, 0.65, 1.35, "#efefef");
    draw_box(0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4");
    draw_box(0.4, 4.4, 0.0, 1.9, 0.22, "#efefef");
    draw_circle(-2.5, 3.6, 0.65, "#d4d4d4");
    draw_circle(4.2, 3.2, 0.75, "#d4d4d4");

    double base_angle = -0.35 + g_manual_offset + 0.28 * sin(g_phase);
    double ox = to_cx(EMITTER_X), oy = to_cy(EMITTER_Y);
    Rapier_Vector3 origin = Rapier_Vector3_new(EMITTER_X, EMITTER_Y, 0.0);

    int hits = 0;
    double min_toi = MAX_TOI;
    for (int i = 0; i < RAY_COUNT; i++) {
        double frac = (double)i / (double)(RAY_COUNT - 1) - 0.5;
        double angle = base_angle + frac * 1.35;
        double dx = cos(angle), dy = sin(angle);
        Rapier_Ray ray = Rapier_Ray_new(origin, Rapier_Vector3_new(dx, dy, 0.0));

        Rapier_RayColliderIntersection hit = Rapier_World_castRayAndGetNormal(g_world, ray, MAX_TOI, true);
        if (js_not_null(hit)) {
            hits++;
            double toi = Rapier_RayColliderIntersection_get_timeOfImpact(hit);
            if (toi < min_toi) min_toi = toi;
            Rapier_Vector pt = Rapier_Ray_pointAt(ray, toi);
            double ptx = Rapier_Vector_get_x(pt), pty = Rapier_Vector_get_y(pt);
            double hx = to_cx(ptx), hy = to_cy(pty);

            CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#737373");
            CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.0);
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_moveTo(g_ctx, ox, oy);
            CanvasRenderingContext2D_lineTo(g_ctx, hx, hy);
            CanvasRenderingContext2D_stroke(g_ctx);

            Rapier_Vector n = Rapier_RayColliderIntersection_get_normal(hit);
            double nx = Rapier_Vector_get_x(n), ny = Rapier_Vector_get_y(n);
            CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#111111");
            CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.4);
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_moveTo(g_ctx, hx, hy);
            CanvasRenderingContext2D_lineTo(g_ctx, to_cx(ptx + nx * 0.45), to_cy(pty + ny * 0.45));
            CanvasRenderingContext2D_stroke(g_ctx);

            CanvasRenderingContext2D_set_fillStyle(g_ctx, "#111111");
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_arc(g_ctx, hx, hy, 2.8, 0.0, TWO_PI);
            CanvasRenderingContext2D_fill(g_ctx);
        } else {
            CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#e5e5e5");
            CanvasRenderingContext2D_set_lineWidth(g_ctx, 0.9);
            CanvasRenderingContext2D_beginPath(g_ctx);
            CanvasRenderingContext2D_moveTo(g_ctx, ox, oy);
            CanvasRenderingContext2D_lineTo(g_ctx, to_cx(EMITTER_X + dx * MAX_TOI), to_cy(EMITTER_Y + dy * MAX_TOI));
            CanvasRenderingContext2D_stroke(g_ctx);
        }
    }

    CanvasRenderingContext2D_set_fillStyle(g_ctx, "#111111");
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, ox, oy, 6.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_fill(g_ctx);
    CanvasRenderingContext2D_set_strokeStyle(g_ctx, "#ffffff");
    CanvasRenderingContext2D_set_lineWidth(g_ctx, 1.5);
    CanvasRenderingContext2D_beginPath(g_ctx);
    CanvasRenderingContext2D_arc(g_ctx, ox, oy, 3.0, 0.0, TWO_PI);
    CanvasRenderingContext2D_stroke(g_ctx);

    int deg = (int)(base_angle * 180.0 / PI + (base_angle >= 0.0 ? 0.5 : -0.5));
    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "37 rays \xC2\xB7 hits=%d/37 \xC2\xB7 nearest_toi=%.2f m \xC2\xB7 center=%d deg (%s)",
        hits, min_toi, deg, g_auto_sweep ? "auto" : "manual"
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
}

static void on_raf(void* ctx, double ts) {
    (void)ctx; (void)ts;
    js_scope();
    if (g_auto_sweep) {
        g_phase += 0.025;
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
        g_auto_sweep = !g_auto_sweep;
        HTMLElement_set_textContent(g_sweep_btn, g_auto_sweep ? "Sweep: Auto" : "Sweep: Manual");
        render();
    } else if (id == 1) {
        g_manual_offset += PI / 12.0;
        if (g_manual_offset > 0.8) g_manual_offset = -0.6;
        render();
    } else {
        g_auto_sweep = true;
        g_phase = 0.0;
        g_manual_offset = 0.0;
        HTMLElement_set_textContent(g_sweep_btn, "Sweep: Auto");
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
    g_sweep_btn = Document_getElementById(doc, "btn-sweep");
    js_retain(g_sweep_btn);

    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, 0.0, 0.0));
    js_retain(g_world);

    add_box(0.0, -0.35, 0.0, 7.0, 0.35);
    add_box(-1.6, 0.85, 0.0, 0.70, 0.85);
    add_box(2.2, 1.35, 0.0, 0.65, 1.35);
    add_box(0.2, 1.2, -0.42, 1.3, 0.18);
    add_box(0.4, 4.4, 0.0, 1.9, 0.22);
    add_sphere(-2.5, 3.6, 0.65);
    add_sphere(4.2, 3.2, 0.75);
    Rapier_World_step(g_world);

    render();

    static const char* ids[3] = { "btn-sweep", "btn-angle", "btn-reset" };
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
