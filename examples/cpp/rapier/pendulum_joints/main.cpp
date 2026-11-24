#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"

using namespace dimforge_rapier3d_compat;

static constexpr double TWO_PI = 2.0 * M_PI;
static constexpr int TRAIL_LEN = 36;
static constexpr double LINK_HX = 0.14;
static constexpr double LINK_HY = 0.62;

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

struct AppState {
    dom::CanvasRenderingContext2D ctx;
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<World> world;
    RigidBody chain[3];
    RigidBody arm;
    RigidBody bob;
    double ax[TRAIL_LEN] = {}, ay[TRAIL_LEN] = {};
    double bx[TRAIL_LEN] = {}, by[TRAIL_LEN] = {};
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

static void draw_pivot(const dom::CanvasRenderingContext2D& ctx, double wx, double wy) {
    double cx = to_cx(wx), cy = to_cy(wy);
    ctx.fillStyle("#ffffff");
    ctx.beginPath(); ctx.arc(cx, cy, 4.0, 0.0, TWO_PI); ctx.fill();
    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.6);
    ctx.beginPath(); ctx.arc(cx, cy, 4.0, 0.0, TWO_PI); ctx.stroke();
}

static void build_scene(const std::shared_ptr<AppState>& st) {
    if (st->world) st->world->free();
    st->world = World(Vector3(0.0, -9.81, 0.0));

    RigidBody gantry = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(0.0, 6.2, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(6.0, 0.22, 0.6), gantry);
    Vector3 z_axis = Vector3(0.0, 0.0, 1.0);

    double cx[3] = { -2.8, -3.3, -3.8 }, cy[3] = { 5.4, 4.1, 2.8 };
    for (int i = 0; i < 3; i++) {
        st->chain[i] = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(cx[i], cy[i], 0.0));
        st->world->createCollider(ColliderDesc::cuboid(LINK_HX, LINK_HY, 0.2).setDensity(i == 2 ? 1.6 : 1.4), st->chain[i]);
    }
    st->world->createImpulseJoint(
        JointData::revolute(Vector3(-2.2, -0.22, 0.0), Vector3(0.0, LINK_HY + 0.06, 0.0), z_axis),
        gantry, st->chain[0], true
    );
    st->world->createImpulseJoint(
        JointData::revolute(Vector3(0.0, -LINK_HY - 0.06, 0.0), Vector3(0.0, LINK_HY + 0.06, 0.0), z_axis),
        st->chain[0], st->chain[1], true
    );
    st->world->createImpulseJoint(
        JointData::revolute(Vector3(0.0, -LINK_HY - 0.06, 0.0), Vector3(0.0, LINK_HY + 0.06, 0.0), z_axis),
        st->chain[1], st->chain[2], true
    );
    st->chain[2].applyImpulse(Vector3(-1.4, 0.0, 0.0), true);

    st->arm = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(3.1, 5.2, 0.0));
    st->world->createCollider(ColliderDesc::cuboid(0.14, 0.85, 0.2).setDensity(1.2), st->arm);
    st->bob = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(3.8, 3.6, 0.0));
    st->world->createCollider(ColliderDesc::ball(0.42).setDensity(3.0), st->bob);

    st->world->createImpulseJoint(
        JointData::revolute(Vector3(2.3, -0.22, 0.0), Vector3(0.0, 0.88, 0.0), z_axis),
        gantry, st->arm, true
    );
    st->world->createImpulseJoint(
        JointData::revolute(Vector3(0.0, -0.88, 0.0), Vector3(0.0, 0.72, 0.0), z_axis),
        st->arm, st->bob, true
    );
    st->bob.applyImpulse(Vector3(1.8, 0.2, 0.0), true);

    st->step_count = 0;
    st->trail_count = 0;
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step();
    st->step_count++;
    Vector pa = st->chain[2].translation();
    Vector pb = st->bob.translation();
    int idx = (st->step_count - 1) % TRAIL_LEN;
    st->ax[idx] = pa.x(); st->ay[idx] = pa.y();
    st->bx[idx] = pb.x(); st->by[idx] = pb.y();
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
        ctx.beginPath(); ctx.moveTo(x, 0.0); ctx.lineTo(x, 400.0); ctx.stroke();
    }
    for (int gy = 0; gy <= 7; gy++) {
        double y = to_cy(double(gy));
        ctx.beginPath(); ctx.moveTo(0.0, y); ctx.lineTo(680.0, y); ctx.stroke();
    }

    draw_rot_box(ctx, 0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, "#efefef");

    ctx.fillStyle("#b5b5b5");
    for (int i = 0; i < st->trail_count; i++) {
        ctx.beginPath(); ctx.arc(to_cx(st->ax[i]), to_cy(st->ay[i]), 1.6, 0.0, TWO_PI); ctx.fill();
        ctx.beginPath(); ctx.arc(to_cx(st->bx[i]), to_cy(st->by[i]), 1.6, 0.0, TWO_PI); ctx.fill();
    }

    double pin_x[2], pin_y[2], tip_x = 0.0, tip_y = 0.0;
    for (int i = 0; i < 3; i++) {
        Vector p = st->chain[i].translation();
        Rotation q = st->chain[i].rotation();
        draw_rot_box(ctx, p.x(), p.y(), q.x(), q.y(), q.z(), q.w(), LINK_HX, LINK_HY, i == 2 ? "#d4d4d4" : "#f7f7f7");
        if (i < 2) {
            double r01 = 2.0 * (q.x() * q.y() - q.w() * q.z());
            double r11 = 1.0 - 2.0 * (q.x() * q.x() + q.z() * q.z());
            pin_x[i] = p.x() - r01 * (LINK_HY + 0.06);
            pin_y[i] = p.y() - r11 * (LINK_HY + 0.06);
        } else {
            tip_x = p.x(); tip_y = p.y();
        }
    }

    Vector ap = st->arm.translation();
    Rotation aq = st->arm.rotation();
    Vector bp = st->bob.translation();
    draw_rot_box(ctx, ap.x(), ap.y(), aq.x(), aq.y(), aq.z(), aq.w(), 0.14, 0.85, "#f7f7f7");

    double bcx = to_cx(bp.x()), bcy = to_cy(bp.y());
    ctx.fillStyle("#262626");
    ctx.beginPath(); ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI); ctx.fill();
    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.6);
    ctx.beginPath(); ctx.arc(bcx, bcy, 0.42 * 46.0, 0.0, TWO_PI); ctx.stroke();

    draw_pivot(ctx, -2.2, 5.98);
    draw_pivot(ctx, pin_x[0], pin_y[0]);
    draw_pivot(ctx, pin_x[1], pin_y[1]);
    draw_pivot(ctx, 2.3, 5.98);
    double arm_r01 = 2.0 * (aq.x() * aq.y() - aq.w() * aq.z());
    double arm_r11 = 1.0 - 2.0 * (aq.x() * aq.x() + aq.z() * aq.z());
    draw_pivot(ctx, ap.x() - arm_r01 * 0.88, ap.y() - arm_r11 * 0.88);

    ctx.fillStyle("#555555");
    ctx.font("11px ui-monospace, monospace");
    ctx.fillText("3-Link Revolute Chain", to_cx(-3.5), 36.0);
    ctx.fillText("2-Body Bob Pendulum", to_cx(1.1), 36.0);

    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "5 revolute joints \xC2\xB7 chain_tip=(%.2f, %.2f) \xC2\xB7 bob=(%.2f, %.2f) \xC2\xB7 step=%d (%s)",
        tip_x, tip_y, bp.x(), bp.y(), st->step_count, st->paused ? "paused" : "running"
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
    for (int i = 0; i < 20; i++) step_sim(st);
    render(st);

    const char* ids[3] = { "btn-kick", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->chain[2].applyImpulse(Vector3(1.6, 0.3, 0.0), true);
                st->bob.applyImpulse(Vector3(-2.0, 0.4, 0.0), true);
                render(st);
            } else if (i == 1) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                render(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_scene(st);
                for (int k = 0; k < 15; k++) step_sim(st);
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
