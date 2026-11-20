#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"

using namespace dimforge_rapier3d_compat;

static constexpr double TWO_PI = 2.0 * M_PI;
static constexpr int DOMINO_COUNT = 9;
static constexpr double DOM_HX = 0.13;
static constexpr double DOM_HY = 0.85;
static constexpr int TRAIL_LEN = 22;

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

struct AppState {
    dom::CanvasRenderingContext2D ctx;
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<World> world;
    RigidBody dominoes[DOMINO_COUNT];
    RigidBody striker;
    double trail_x[TRAIL_LEN] = {};
    double trail_y[TRAIL_LEN] = {};
    int trail_count = 0;
    int step_count = 0;
    bool paused = false;
    dom::FrameRequestCallback raf_cb;
};

static void draw_rot_box(const dom::CanvasRenderingContext2D& ctx, double cx, double cy,
                         double qx, double qy, double qz, double qw,
                         double hx, double hy, const char* fill_hex) {
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

    ctx.fillStyle(fill_hex);
    ctx.beginPath();
    ctx.moveTo(xs[0], ys[0]);
    for (int i = 1; i < 4; i++) ctx.lineTo(xs[i], ys[i]);
    ctx.closePath();
    ctx.fill();

    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.5);
    ctx.beginPath();
    ctx.moveTo(xs[0], ys[0]);
    for (int i = 1; i < 4; i++) ctx.lineTo(xs[i], ys[i]);
    ctx.closePath();
    ctx.stroke();
}

static void build_scene(const std::shared_ptr<AppState>& st) {
    if (st->world) st->world->free();
    st->world = World::create(Vector3::create(0.0, -9.81, 0.0));

    RigidBody floor = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(0.0, -0.35, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(7.2, 0.35, 2.0).setFriction(0.8), floor);

    for (int i = 0; i < DOMINO_COUNT; i++) {
        double x = -2.4 + double(i) * 0.88;
        st->dominoes[i] = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(x, DOM_HY, 0.0));
        st->world->createCollider(
            ColliderDesc::cuboid(DOM_HX, DOM_HY, 0.35).setFriction(0.55).setRestitution(0.15).setDensity(1.0),
            st->dominoes[i]
        );
    }

    st->striker = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(-5.3, 1.05, 0.0));
    st->world->createCollider(ColliderDesc::ball(0.40).setDensity(2.8).setRestitution(0.45).setFriction(0.25), st->striker);
    st->striker.applyImpulse(Vector3::create(4.8, 0.2, 0.0), true);

    st->step_count = 0;
    st->trail_count = 0;
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step();
    st->step_count++;
    Vector sp = st->striker.translation();
    int idx = (st->step_count - 1) % TRAIL_LEN;
    st->trail_x[idx] = sp.x();
    st->trail_y[idx] = sp.y();
    if (st->trail_count < TRAIL_LEN) st->trail_count++;
}

static void render(const std::shared_ptr<AppState>& st) {
    const auto& ctx = st->ctx;
    ctx.fillStyle("#ffffff");
    ctx.fillRect(0.0, 0.0, 680.0, 400.0);

    ctx.strokeStyle("#efefef");
    ctx.lineWidth(1.0);
    for (int gx = -6; gx <= 6; gx++) {
        double x = to_cx(double(gx));
        ctx.beginPath(); ctx.moveTo(x, 0.0); ctx.lineTo(x, 350.0); ctx.stroke();
    }
    for (int gy = 1; gy <= 7; gy++) {
        double y = to_cy(double(gy));
        ctx.beginPath(); ctx.moveTo(0.0, y); ctx.lineTo(680.0, y); ctx.stroke();
    }

    ctx.fillStyle("#f7f7f7");
    ctx.fillRect(0.0, 350.0, 680.0, 50.0);
    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.5);
    ctx.beginPath(); ctx.moveTo(0.0, 350.0); ctx.lineTo(680.0, 350.0); ctx.stroke();

    ctx.fillStyle("#b5b5b5");
    for (int i = 0; i < st->trail_count; i++) {
        ctx.beginPath();
        ctx.arc(to_cx(st->trail_x[i]), to_cy(st->trail_y[i]), 2.0, 0.0, TWO_PI);
        ctx.fill();
    }

    int toppled = 0;
    for (int i = 0; i < DOMINO_COUNT; i++) {
        Vector p = st->dominoes[i].translation();
        Rotation q = st->dominoes[i].rotation();
        bool tilted = std::fabs(q.z()) > 0.25;
        if (tilted) toppled++;
        draw_rot_box(ctx, p.x(), p.y(), q.x(), q.y(), q.z(), q.w(), DOM_HX, DOM_HY, tilted ? "#d4d4d4" : "#f7f7f7");
    }

    Vector sp = st->striker.translation();
    double scx = to_cx(sp.x()), scy = to_cy(sp.y());
    ctx.fillStyle("#262626");
    ctx.beginPath(); ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI); ctx.fill();
    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.6);
    ctx.beginPath(); ctx.arc(scx, scy, 0.40 * 46.0, 0.0, TWO_PI); ctx.stroke();
    ctx.fillStyle("#e5e5e5");
    ctx.beginPath(); ctx.arc(scx - 5.0, scy - 5.0, 2.5, 0.0, TWO_PI); ctx.fill();

    char cbuf[64];
    std::snprintf(cbuf, sizeof cbuf, "Dominoes toppled: %d / 9", toppled);
    ctx.fillStyle("#555555");
    ctx.font("11px ui-monospace, monospace");
    ctx.fillText(cbuf, 18.0, 24.0);

    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "9 cuboids + striker sphere \xC2\xB7 toppled=%d/9 \xC2\xB7 striker_x=%.2f m \xC2\xB7 step=%d (%s)",
        toppled, sp.x(), st->step_count, st->paused ? "paused" : "running"
    );
    st->meta.textContent(mbuf);
}

static ts::Task run_app() {
    ts::Result<void> res = co_await init();
    if (!res) co_return;

    dom::Document doc = dom::get_document();
    dom::HTMLCanvasElement canvas = *dom::HTMLCanvasElement::cast(*doc.getElementById("canvas"));
    auto st = std::make_shared<AppState>();
    st->ctx = *canvas.getContext_2d();
    st->meta = *doc.getElementById("meta");
    st->pause_btn = *doc.getElementById("btn-pause");

    build_scene(st);
    for (int i = 0; i < 22; i++) step_sim(st);
    render(st);

    const char* ids[3] = { "btn-push", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->striker.applyImpulse(Vector3::create(3.5, 0.6, 0.0), true);
                render(st);
            } else if (i == 1) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                render(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_scene(st);
                for (int k = 0; k < 12; k++) step_sim(st);
                render(st);
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }

    st->raf_cb = [st](double) {
        if (!st->paused) {
            step_sim(st);
            render(st);
        }
        dom::requestAnimationFrame(st->raf_cb);
    };
    dom::requestAnimationFrame(st->raf_cb);
}

TS_EXPORT(start) void start() {
    run_app();
}
