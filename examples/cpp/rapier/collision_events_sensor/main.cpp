#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"

using namespace dimforge_rapier3d_compat;

static constexpr double TWO_PI = 2.0 * M_PI;
static constexpr int BODY_COUNT = 6;
static constexpr bool IS_BOX[BODY_COUNT] = { false, false, true, false, true, false };

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

struct AppState {
    dom::CanvasRenderingContext2D ctx;
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<World> world;
    std::optional<EventQueue> queue;
    RigidBody bodies[BODY_COUNT];
    double sensor_handle = 0.0;
    int step_count = 0;
    int solid_contacts = 0;
    int sensor_crossings = 0;
    int active_in_sensor = 0;
    bool paused = false;
    EventQueue_drainCollisionEvents_f col_cb;
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
    ctx.beginPath(); ctx.moveTo(xs[0], ys[0]);
    for (int i = 1; i < 4; i++) ctx.lineTo(xs[i], ys[i]);
    ctx.closePath(); ctx.fill();

    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.5);
    ctx.beginPath(); ctx.moveTo(xs[0], ys[0]);
    for (int i = 1; i < 4; i++) ctx.lineTo(xs[i], ys[i]);
    ctx.closePath(); ctx.stroke();
}

static void draw_segs(const dom::CanvasRenderingContext2D& ctx, double x1, double y1, double x2, double y2, int n) {
    double dx = x2 - x1, dy = y2 - y1;
    for (int i = 0; i < n; i += 2) {
        double t0 = double(i) / double(n), t1 = double(i + 1) / double(n);
        ctx.beginPath();
        ctx.moveTo(x1 + dx * t0, y1 + dy * t0);
        ctx.lineTo(x1 + dx * t1, y1 + dy * t1);
        ctx.stroke();
    }
}

static void build_scene(const std::shared_ptr<AppState>& st) {
    if (st->queue) st->queue->free();
    if (st->world) st->world->free();
    st->world = World::create(Vector3::create(0.0, -9.81, 0.0));
    st->queue = EventQueue::create(true);

    auto add_ramp = [&](double cx, double cy, double hx, double hy, double theta) {
        RigidBody rb = st->world->createRigidBody(
            RigidBodyDesc::fixed().setTranslation(cx, cy, 0.0).setRotation(Quaternion::create(0.0, 0.0, std::sin(theta * 0.5), std::cos(theta * 0.5)))
        );
        st->world->createCollider(ColliderDesc::cuboid(hx, hy, 1.0).setFriction(0.15).setRestitution(0.35), rb);
    };

    add_ramp(0.0, -0.35, 6.8, 0.35, 0.0);
    add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(5.4, 2.8, 0.25, 3.2, 0.0);
    add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36);
    add_ramp(1.8, 2.6, 2.2, 0.18, 0.36);

    RigidBody sbody = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(0.0, 1.35, 0.0));
    Collider sc = st->world->createCollider(
        ColliderDesc::cuboid(1.85, 0.30, 1.2).setSensor(true).setActiveEvents(ActiveEvents::COLLISION_EVENTS),
        sbody
    );
    st->sensor_handle = sc.handle();

    for (int i = 0; i < BODY_COUNT; i++) {
        double x = -3.7 + double(i) * 0.52;
        double y = 5.4 + double(i % 3) * 0.58;
        st->bodies[i] = st->world->createRigidBody(RigidBodyDesc::dynamic().setTranslation(x, y, 0.0));
        ColliderDesc cd = IS_BOX[i] ? ColliderDesc::cuboid(0.28, 0.28, 0.28) : ColliderDesc::ball(0.32);
        st->world->createCollider(
            cd.setRestitution(0.35).setFriction(0.25).setActiveEvents(ActiveEvents::COLLISION_EVENTS),
            st->bodies[i]
        );
    }

    st->step_count = 0;
    st->solid_contacts = 0;
    st->sensor_crossings = 0;
    st->active_in_sensor = 0;
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step(*st->queue);
    st->step_count++;
    st->queue->drainCollisionEvents(st->col_cb);
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

    double sx = to_cx(-1.85), sy = to_cy(1.65), sw = 3.70 * 46.0, sh = 0.60 * 46.0;
    if (st->active_in_sensor > 0) {
        ctx.fillStyle("#efefef");
        ctx.fillRect(sx, sy, sw, sh);
        ctx.strokeStyle("#111111");
        ctx.lineWidth(2.0);
        ctx.strokeRect(sx, sy, sw, sh);
    } else {
        ctx.strokeStyle("#737373");
        ctx.lineWidth(1.4);
        draw_segs(ctx, sx, sy, sx + sw, sy, 14);
        draw_segs(ctx, sx + sw, sy, sx + sw, sy + sh, 4);
        draw_segs(ctx, sx + sw, sy + sh, sx, sy + sh, 14);
        draw_segs(ctx, sx, sy + sh, sx, sy, 4);
    }

    char gcap[64];
    std::snprintf(gcap, sizeof gcap, "SENSOR GATE (%d inside)", st->active_in_sensor);
    ctx.fillStyle("#555555");
    ctx.font("11px ui-monospace, monospace");
    ctx.fillText(gcap, sx + 8.0, sy + 17.0);

    draw_rot_box(ctx, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, "#f7f7f7");
    draw_rot_box(ctx, -5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(ctx, 5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef");
    draw_rot_box(ctx, -2.1, 4.2, 0.0, 0.0, std::sin(-0.18), std::cos(-0.18), 2.2, 0.18, "#e5e5e5");
    draw_rot_box(ctx, 1.8, 2.6, 0.0, 0.0, std::sin(0.18), std::cos(0.18), 2.2, 0.18, "#e5e5e5");

    for (int i = 0; i < BODY_COUNT; i++) {
        Vector p = st->bodies[i].translation();
        Rotation q = st->bodies[i].rotation();
        if (IS_BOX[i]) {
            draw_rot_box(ctx, p.x(), p.y(), q.x(), q.y(), q.z(), q.w(), 0.28, 0.28, "#d4d4d4");
        } else {
            double cx = to_cx(p.x()), cy = to_cy(p.y());
            ctx.fillStyle("#262626");
            ctx.beginPath(); ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI); ctx.fill();
            ctx.strokeStyle("#111111");
            ctx.lineWidth(1.5);
            ctx.beginPath(); ctx.arc(cx, cy, 0.32 * 46.0, 0.0, TWO_PI); ctx.stroke();
        }
    }

    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "EventQueue \xC2\xB7 solid_contacts=%d \xC2\xB7 sensor_crossings=%d \xC2\xB7 inside=%d \xC2\xB7 step=%d (%s)",
        st->solid_contacts, st->sensor_crossings, st->active_in_sensor, st->step_count, st->paused ? "paused" : "running"
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
    st->col_cb = [st](double h1, double h2, bool started) {
        if (h1 == st->sensor_handle || h2 == st->sensor_handle) {
            if (started) {
                st->sensor_crossings++;
                st->active_in_sensor++;
            } else if (st->active_in_sensor > 0) {
                st->active_in_sensor--;
            }
        } else if (started) {
            st->solid_contacts++;
        }
    };

    build_scene(st);
    for (int i = 0; i < 40; i++) step_sim(st);
    render(st);

    const char* ids[3] = { "btn-drop", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                Vector3 v = Vector3::create(0.4, 0.0, 0.0);
                for (int j = 0; j < BODY_COUNT; j++) {
                    st->bodies[j].setTranslation(Vector3::create(-3.7 + double(j) * 0.52, 5.4 + double(j % 3) * 0.58, 0.0), true);
                    st->bodies[j].setLinvel(v, true);
                }
                render(st);
            } else if (i == 1) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                render(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_scene(st);
                for (int k = 0; k < 25; k++) step_sim(st);
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
