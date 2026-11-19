#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"

using namespace dimforge_rapier3d_compat;

static constexpr double TWO_PI = 2.0 * M_PI;
static constexpr int TRAIL_LEN = 20;
static constexpr double RESTITUTIONS[5] = { 0.00, 0.35, 0.60, 0.80, 0.95 };
static constexpr double X_COORDS[5] = { -4.2, -2.1, 0.0, 2.1, 4.2 };
static const char* BALL_HEX[5] = { "#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626" };

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

struct AppState {
    dom::CanvasRenderingContext2D ctx;
    dom::HTMLElement meta;
    dom::HTMLElement grav_btn;
    dom::HTMLElement pause_btn;
    std::optional<World> world;
    RigidBody balls[5];
    double trail_x[5][TRAIL_LEN] = {};
    double trail_y[5][TRAIL_LEN] = {};
    int trail_count = 0;
    int step_count = 0;
    bool earth = true;
    bool paused = false;
    dom::FrameRequestCallback raf_cb;
};

static void draw_dashed(const dom::CanvasRenderingContext2D& ctx, double x1, double y1, double x2, double y2, int segs) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < segs; i += 2) {
        double t0 = double(i) / double(segs);
        double t1 = double(i + 1) / double(segs);
        ctx.beginPath();
        ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

static void build_scene(const std::shared_ptr<AppState>& st) {
    if (st->world) st->world->free();
    st->world = World::create(Vector3::create(0.0, st->earth ? -9.81 : -1.62, 0.0));

    RigidBody ground = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(0.0, -0.4, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(7.0, 0.4, 2.0), ground);
    RigidBody lw = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(-6.8, 3.5, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(0.25, 4.0, 2.0), lw);
    RigidBody rw = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(6.8, 3.5, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(0.25, 4.0, 2.0), rw);

    for (int i = 0; i < 5; i++) {
        st->balls[i] = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(X_COORDS[i], 5.6, 0.0));
        st->world->createCollider(
            ColliderDesc::ball(0.42).setRestitution(RESTITUTIONS[i]).setFriction(0.2).setDensity(1.2),
            st->balls[i]
        );
    }
    st->step_count = 0;
    st->trail_count = 0;
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step();
    st->step_count++;
    int idx = (st->step_count - 1) % TRAIL_LEN;
    for (int i = 0; i < 5; i++) {
        Vector p = st->balls[i].translation();
        st->trail_x[i][idx] = p.x();
        st->trail_y[i][idx] = p.y();
    }
    if (st->trail_count < TRAIL_LEN) st->trail_count++;
}

static void render(const std::shared_ptr<AppState>& st) {
    const auto& ctx = st->ctx;
    ctx.fillStyle(std::string("#ffffff"));
    ctx.fillRect(0.0, 0.0, 680.0, 400.0);

    ctx.strokeStyle(std::string("#efefef"));
    ctx.lineWidth(1.0);
    for (int gx = -6; gx <= 6; gx++) {
        double x = to_cx(double(gx));
        ctx.beginPath(); ctx.moveTo(x, 0.0); ctx.lineTo(x, 350.0); ctx.stroke();
    }
    for (int gy = 1; gy <= 7; gy++) {
        double y = to_cy(double(gy));
        ctx.beginPath(); ctx.moveTo(0.0, y); ctx.lineTo(680.0, y); ctx.stroke();
    }

    ctx.fillStyle(std::string("#f7f7f7"));
    ctx.fillRect(0.0, 350.0, 680.0, 50.0);
    ctx.strokeStyle(std::string("#d4d4d4"));
    for (double hx = -20.0; hx < 700.0; hx += 18.0) {
        ctx.beginPath(); ctx.moveTo(hx, 350.0); ctx.lineTo(hx - 24.0, 400.0); ctx.stroke();
    }
    ctx.strokeStyle(std::string("#111111"));
    ctx.lineWidth(1.5);
    ctx.beginPath(); ctx.moveTo(0.0, 350.0); ctx.lineTo(680.0, 350.0); ctx.stroke();

    ctx.strokeStyle(std::string("#d4d4d4"));
    ctx.lineWidth(1.0);
    draw_dashed(ctx, to_cx(-5.0), to_cy(5.6), to_cx(5.0), to_cy(5.6), 40);
    for (int i = 0; i < 5; i++) {
        double cx = to_cx(X_COORDS[i]);
        draw_dashed(ctx, cx, to_cy(5.6), cx, 350.0, 20);
    }

    double energy = 0.0, max_y = 0.0;
    ctx.font("11px ui-monospace, monospace");
    for (int i = 0; i < 5; i++) {
        ctx.fillStyle(std::string("#b5b5b5"));
        for (int k = 0; k < st->trail_count; k++) {
            ctx.beginPath();
            ctx.arc(to_cx(st->trail_x[i][k]), to_cy(st->trail_y[i][k]), 1.6, 0.0, TWO_PI);
            ctx.fill();
        }
        Vector p = st->balls[i].translation();
        Vector v = st->balls[i].linvel();
        energy += 0.5 * (v.x() * v.x() + v.y() * v.y());
        if (p.y() > max_y) max_y = p.y();

        double cx = to_cx(p.x()), cy = to_cy(p.y()), rpx = 0.42 * 46.0;
        ctx.fillStyle(std::string(BALL_HEX[i]));
        ctx.beginPath(); ctx.arc(cx, cy, rpx, 0.0, TWO_PI); ctx.fill();
        ctx.strokeStyle(std::string("#111111"));
        ctx.lineWidth(1.6);
        ctx.beginPath(); ctx.arc(cx, cy, rpx, 0.0, TWO_PI); ctx.stroke();

        ctx.fillStyle(std::string(i >= 3 ? "#d4d4d4" : "#737373"));
        ctx.beginPath(); ctx.arc(cx - 5.0, cy - 5.0, 2.8, 0.0, TWO_PI); ctx.fill();

        char rbuf[16];
        std::snprintf(rbuf, sizeof rbuf, "e=%.2f", RESTITUTIONS[i]);
        ctx.fillStyle(std::string("#111111"));
        ctx.fillText(rbuf, cx - 22.0, 368.0);
    }

    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "5 spheres \xC2\xB7 g=%s m/s\xC2\xB2 \xC2\xB7 max_y=%.2f m \xC2\xB7 ke=%.1f \xC2\xB7 step=%d (%s)",
        st->earth ? "Earth (-9.81)" : "Moon (-1.62)",
        max_y, energy, st->step_count, st->paused ? "paused" : "running"
    );
    st->meta.textContent(std::string(mbuf));
}

static ts::Task run_app() {
    ts::Result<void> res = co_await init();
    if (!res) co_return;

    dom::Document doc = dom::get_document();
    dom::HTMLCanvasElement canvas = *dom::HTMLCanvasElement::cast(*doc.getElementById("canvas"));
    auto st = std::make_shared<AppState>();
    st->ctx = *canvas.getContext_2d();
    st->meta = *doc.getElementById("meta");
    st->grav_btn = *doc.getElementById("btn-gravity");
    st->pause_btn = *doc.getElementById("btn-pause");

    build_scene(st);
    for (int i = 0; i < 28; i++) step_sim(st);
    render(st);

    const char* ids[4] = { "btn-gravity", "btn-kick", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->earth = !st->earth;
                st->world->gravity(Vector3::create(0.0, st->earth ? -9.81 : -1.62, 0.0));
                Vector3 imp = Vector3::create(0.0, 0.25, 0.0);
                for (int j = 0; j < 5; j++) st->balls[j].applyImpulse(imp, true);
                st->grav_btn.textContent(st->earth ? "Gravity: Earth" : "Gravity: Moon");
                render(st);
            } else if (i == 1) {
                for (int j = 0; j < 5; j++) {
                    st->balls[j].applyImpulse(Vector3::create((double(j) - 2.0) * 0.28, 3.4, 0.0), true);
                }
                render(st);
            } else if (i == 2) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                render(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_scene(st);
                for (int k = 0; k < 18; k++) step_sim(st);
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

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
