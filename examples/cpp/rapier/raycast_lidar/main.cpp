#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"

using namespace dimforge_rapier3d_compat;

static constexpr double TWO_PI = 2.0 * M_PI;
static constexpr int RAY_COUNT = 37;
static constexpr double MAX_TOI = 16.0;
static constexpr double EMITTER_X = -5.6;
static constexpr double EMITTER_Y = 4.6;

static double to_cx(double wx) { return 340.0 + wx * 46.0; }
static double to_cy(double wy) { return 350.0 - wy * 46.0; }

struct AppState {
    dom::CanvasRenderingContext2D ctx;
    dom::HTMLElement meta;
    dom::HTMLElement sweep_btn;
    std::optional<World> world;
    bool auto_sweep = true;
    double phase = 0.0;
    double manual_offset = 0.0;
    dom::FrameRequestCallback raf_cb;
};

static void draw_box(const dom::CanvasRenderingContext2D& ctx, double cx, double cy, double theta,
                     double hx, double hy, const char* fill_hex) {
    double c = std::cos(theta), s = std::sin(theta);
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

static void draw_circle(const dom::CanvasRenderingContext2D& ctx, double wx, double wy, double r, const char* fill_hex) {
    double cx = to_cx(wx), cy = to_cy(wy), rp = r * 46.0;
    ctx.fillStyle(fill_hex);
    ctx.beginPath(); ctx.arc(cx, cy, rp, 0.0, TWO_PI); ctx.fill();
    ctx.strokeStyle("#111111");
    ctx.lineWidth(1.5);
    ctx.beginPath(); ctx.arc(cx, cy, rp, 0.0, TWO_PI); ctx.stroke();
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

    draw_box(ctx, 0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7");
    draw_box(ctx, -1.6, 0.85, 0.0, 0.70, 0.85, "#efefef");
    draw_box(ctx, 2.2, 1.35, 0.0, 0.65, 1.35, "#efefef");
    draw_box(ctx, 0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4");
    draw_box(ctx, 0.4, 4.4, 0.0, 1.9, 0.22, "#efefef");
    draw_circle(ctx, -2.5, 3.6, 0.65, "#d4d4d4");
    draw_circle(ctx, 4.2, 3.2, 0.75, "#d4d4d4");

    double base_angle = -0.35 + st->manual_offset + 0.28 * std::sin(st->phase);
    double ox = to_cx(EMITTER_X), oy = to_cy(EMITTER_Y);
    Vector3 origin = Vector3(EMITTER_X, EMITTER_Y, 0.0);

    int hits = 0;
    double min_toi = MAX_TOI;
    for (int i = 0; i < RAY_COUNT; i++) {
        double frac = double(i) / double(RAY_COUNT - 1) - 0.5;
        double angle = base_angle + frac * 1.35;
        double dx = std::cos(angle), dy = std::sin(angle);
        Ray ray = Ray(origin, Vector3(dx, dy, 0.0));
        if (auto hit = st->world->castRayAndGetNormal(ray, MAX_TOI, true)) {
            hits++;
            double toi = hit->timeOfImpact();
            if (toi < min_toi) min_toi = toi;
            Vector pt = ray.pointAt(toi);
            double hx = to_cx(pt.x()), hy = to_cy(pt.y());

            ctx.strokeStyle("#737373");
            ctx.lineWidth(1.0);
            ctx.beginPath(); ctx.moveTo(ox, oy); ctx.lineTo(hx, hy); ctx.stroke();

            Vector n = hit->normal();
            ctx.strokeStyle("#111111");
            ctx.lineWidth(1.4);
            ctx.beginPath(); ctx.moveTo(hx, hy); ctx.lineTo(to_cx(pt.x() + n.x() * 0.45), to_cy(pt.y() + n.y() * 0.45)); ctx.stroke();

            ctx.fillStyle("#111111");
            ctx.beginPath(); ctx.arc(hx, hy, 2.8, 0.0, TWO_PI); ctx.fill();
        } else {
            ctx.strokeStyle("#e5e5e5");
            ctx.lineWidth(0.9);
            ctx.beginPath();
            ctx.moveTo(ox, oy);
            ctx.lineTo(to_cx(EMITTER_X + dx * MAX_TOI), to_cy(EMITTER_Y + dy * MAX_TOI));
            ctx.stroke();
        }
    }

    ctx.fillStyle("#111111");
    ctx.beginPath(); ctx.arc(ox, oy, 6.0, 0.0, TWO_PI); ctx.fill();
    ctx.strokeStyle("#ffffff");
    ctx.lineWidth(1.5);
    ctx.beginPath(); ctx.arc(ox, oy, 3.0, 0.0, TWO_PI); ctx.stroke();

    long deg = std::lround(base_angle * 180.0 / M_PI);
    char mbuf[256];
    std::snprintf(
        mbuf, sizeof mbuf,
        "37 rays \xC2\xB7 hits=%d/37 \xC2\xB7 nearest_toi=%.2f m \xC2\xB7 center=%ld deg (%s)",
        hits, min_toi, deg, st->auto_sweep ? "auto" : "manual"
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
    st->sweep_btn = *doc.getElementById("btn-sweep");
    st->world = World(Vector3(0.0, 0.0, 0.0));

    auto add_box = [&](double cx, double cy, double theta, double hx, double hy) {
        RigidBody rb = st->world->createRigidBody(
            RigidBodyDesc::fixed().setTranslation(cx, cy, 0.0).setRotation(Quaternion(0.0, 0.0, std::sin(theta * 0.5), std::cos(theta * 0.5)))
        );
        st->world->createCollider(ColliderDesc::cuboid(hx, hy, 1.0), rb);
    };
    auto add_sphere = [&](double cx, double cy, double r) {
        RigidBody rb = st->world->createRigidBody(RigidBodyDesc::fixed().setTranslation(cx, cy, 0.0));
        st->world->createCollider(ColliderDesc::ball(r), rb);
    };

    add_box(0.0, -0.35, 0.0, 7.0, 0.35);
    add_box(-1.6, 0.85, 0.0, 0.70, 0.85);
    add_box(2.2, 1.35, 0.0, 0.65, 1.35);
    add_box(0.2, 1.2, -0.42, 1.3, 0.18);
    add_box(0.4, 4.4, 0.0, 1.9, 0.22);
    add_sphere(-2.5, 3.6, 0.65);
    add_sphere(4.2, 3.2, 0.75);
    st->world->step();

    render(st);

    const char* ids[3] = { "btn-sweep", "btn-angle", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->auto_sweep = !st->auto_sweep;
                st->sweep_btn.textContent(st->auto_sweep ? "Sweep: Auto" : "Sweep: Manual");
                render(st);
            } else if (i == 1) {
                st->manual_offset += M_PI / 12.0;
                if (st->manual_offset > 0.8) st->manual_offset = -0.6;
                render(st);
            } else {
                st->auto_sweep = true;
                st->phase = 0.0;
                st->manual_offset = 0.0;
                st->sweep_btn.textContent("Sweep: Auto");
                render(st);
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }

    st->raf_cb = [st](double) {
        if (st->auto_sweep) {
            st->phase += 0.025;
            render(st);
        }
        dom::requestAnimationFrame(st->raf_cb);
    };
    dom::requestAnimationFrame(st->raf_cb);
}

TS_EXPORT(start) void start() {
    run_app();
}
