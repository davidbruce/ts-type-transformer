#include <cmath>
#include <cstdio>
#include <memory>
#include <optional>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

namespace rp = dimforge_rapier3d_compat;
namespace b = babylonjs_lite;

static constexpr int RACK_COUNT = 10;
static constexpr int BALL_COUNT = 11;
static constexpr double BALL_RADIUS = 0.36;

struct AppState {
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<rp::World> world;
    std::vector<rp::RigidBody> balls;
    std::vector<b::Mesh> ball_meshes;
    bool paused = false;
    int step_count = 0;
};

static void set_status_error(std::string_view message) {
    if (auto el = dom::get_document().getElementById("status")) {
        el->textContent("WebGPU / Babylon Lite error: " + std::string(message));
    }
}

static void build_world(const std::shared_ptr<AppState>& st, bool shoot_cue) {
    if (st->world) st->world->free();
    st->world = rp::World::create(rp::Vector3::create(0.0, -9.81, 0.0));

    rp::RigidBody bed = st->world->createRigidBody(rp::RigidBodyDesc::fixed().setTranslation(0.0, -0.25, 0.0));
    st->world->createCollider(rp::ColliderDesc::cuboid(4.6, 0.25, 7.6).setFriction(0.14).setRestitution(0.35), bed);

    struct BoxSpec { double x, y, z, hx, hy, hz; };
    BoxSpec rails[4] = {
        { -4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 0.0, 0.45, -7.35, 4.6, 0.45, 0.25 },
        { 0.0, 0.45, 7.35, 4.6, 0.45, 0.25 },
    };
    for (const auto& r : rails) {
        rp::RigidBody rb = st->world->createRigidBody(rp::RigidBodyDesc::fixed().setTranslation(r.x, r.y, r.z));
        st->world->createCollider(rp::ColliderDesc::cuboid(r.hx, r.hy, r.hz).setFriction(0.12).setRestitution(0.88), rb);
    }

    st->balls.clear();
    for (int row = 0; row < 4; row++) {
        for (int col = 0; col <= row; col++) {
            double x = (double(col) - double(row) * 0.5) * 0.74;
            double z = -1.6 - double(row) * 0.645;
            rp::RigidBody rb = st->world->createRigidBody(
                rp::RigidBodyDesc::dynamic().setTranslation(x, BALL_RADIUS, z).setLinearDamping(0.30).setAngularDamping(0.30)
            );
            st->world->createCollider(
                rp::ColliderDesc::ball(BALL_RADIUS).setRestitution(0.92).setFriction(0.10).setDensity(1.0),
                rb
            );
            st->balls.push_back(rb);
        }
    }

    rp::RigidBodyDesc cue_desc = rp::RigidBodyDesc::dynamic()
        .setTranslation(0.04, BALL_RADIUS, 4.4)
        .setLinearDamping(0.22)
        .setAngularDamping(0.22);
    if (shoot_cue) cue_desc = cue_desc.setLinvel(-0.18, 0.0, -16.0);
    rp::RigidBody cue = st->world->createRigidBody(cue_desc);
    st->world->createCollider(
        rp::ColliderDesc::ball(BALL_RADIUS).setRestitution(0.94).setFriction(0.10).setDensity(1.25),
        cue
    );
    st->balls.push_back(cue);
    st->step_count = 0;
}

static void sync_meshes(const std::shared_ptr<AppState>& st) {
    for (size_t i = 0; i < st->balls.size() && i < st->ball_meshes.size(); i++) {
        rp::Vector p = st->balls[i].translation();
        rp::Rotation q = st->balls[i].rotation();
        st->ball_meshes[i].position().set(p.x(), p.y(), p.z());
        st->ball_meshes[i].rotationQuaternion().set(q.x(), q.y(), q.z(), q.w());
    }
}

static void update_meta(const std::shared_ptr<AppState>& st) {
    int moving = 0;
    double ke = 0.0;
    for (const auto& rb : st->balls) {
        rp::Vector v = rb.linvel();
        double s2 = v.x() * v.x() + v.y() * v.y() + v.z() * v.z();
        if (s2 > 0.05) moving++;
        ke += 0.5 * s2;
    }
    char buf[256];
    std::snprintf(
        buf, sizeof buf,
        "11 spheres \xC2\xB7 moving=%d \xC2\xB7 ke=%.1f \xC2\xB7 step=%d (%s)",
        moving, ke, st->step_count, st->paused ? "paused" : "running"
    );
    st->meta.textContent(buf);
}

static void step_sim(const std::shared_ptr<AppState>& st) {
    st->world->step();
    st->step_count++;
    sync_meshes(st);
    update_meta(st);
}

static ts::Task run_app() {
    if (auto res = co_await rp::init(); !res) co_return;

    dom::Document doc = dom::get_document();
    auto st = std::make_shared<AppState>();
    st->meta = *doc.getElementById("meta");
    st->pause_btn = *doc.getElementById("btn-pause");

    build_world(st, true);
    for (int i = 0; i < 26; i++) {
        st->world->step();
        st->step_count++;
    }
    update_meta(st);

    const char* ids[4] = { "btn-break", "btn-scatter", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_world(st, true);
                sync_meshes(st);
                update_meta(st);
            } else if (i == 1) {
                for (size_t idx = 0; idx < st->balls.size(); idx++) {
                    double ang = double(idx) * 1.15;
                    st->balls[idx].applyImpulse(rp::Vector3::create(std::cos(ang) * 1.4, 0.25, std::sin(ang) * 1.4), true);
                }
                update_meta(st);
            } else if (i == 2) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                update_meta(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_world(st, false);
                sync_meshes(st);
                update_meta(st);
            }
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }

    auto maybe_canvas = doc.querySelector(dom::HTMLElementTagNameMapKeys::canvas);
    if (!maybe_canvas) {
        set_status_error("missing <canvas> element");
        co_return;
    }
    dom::HTMLCanvasElement canvas = *maybe_canvas;

    auto eng_res = co_await b::createEngine(canvas);
    if (!eng_res) {
        set_status_error(eng_res.error().message());
        co_return;
    }
    b::EngineContext engine = *eng_res;

    b::SceneContext scene = b::createSceneContext(engine);
    scene.clearColor(dom::GPUColorDict::create().r(0.05).g(0.07).b(0.09).a(1.0));

    b::ArcRotateCamera camera = b::createArcRotateCamera(1.18, 0.92, 13.5, b::vec3(0.0, 0.3, -0.4));
    b::addToScene(scene, camera);
    b::attachControl(camera, canvas, scene);

    b::HemisphericLight hemi = b::createHemisphericLight({0.0, 1.0, 0.2}, 0.72);
    hemi.diffuseColor({0.85, 0.9, 0.95});
    hemi.groundColor({0.12, 0.14, 0.16});
    b::addToScene(scene, hemi);

    b::DirectionalLight dir = b::createDirectionalLight({-1.0, -2.0, -1.0}, 2.1);
    dir.diffuse({1.0, 0.97, 0.92});
    b::addToScene(scene, dir);

    auto mk_mat = [](double r, double g, double bl, double m, double ro) {
        return b::createPbrMaterial(
            b::createPbrMaterial_props::create()
                .baseColorFactor({r, g, bl, 1.0})
                .metallicFactor(m)
                .roughnessFactor(ro)
        );
    };

    b::Mesh table = b::createBox(engine, b::BoxOptions::create().width(9.2).height(0.5).depth(15.2));
    table.position().set(0.0, -0.25, 0.0);
    table.material(mk_mat(0.08, 0.24, 0.18, 0.05, 0.78));
    b::addToScene(scene, table);

    b::PbrMaterialProps rail_mat = mk_mat(0.18, 0.18, 0.20, 0.35, 0.42);
    struct RailMesh { double x, y, z, w, h, d; };
    RailMesh rail_meshes[4] = {
        { -4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 0.0, 0.45, -7.35, 9.2, 0.9, 0.5 },
        { 0.0, 0.45, 7.35, 9.2, 0.9, 0.5 },
    };
    for (const auto& rm : rail_meshes) {
        b::Mesh m = b::createBox(engine, b::BoxOptions::create().width(rm.w).height(rm.h).depth(rm.d));
        m.position().set(rm.x, rm.y, rm.z);
        m.material(rail_mat);
        b::addToScene(scene, m);
    }

    b::PbrMaterialProps cue_mat = mk_mat(0.96, 0.96, 0.95, 0.15, 0.18);
    b::PbrMaterialProps rack_mats[4] = {
        mk_mat(0.85, 0.24, 0.22, 0.2, 0.22),
        mk_mat(0.22, 0.48, 0.85, 0.2, 0.22),
        mk_mat(0.92, 0.68, 0.18, 0.2, 0.22),
        mk_mat(0.20, 0.20, 0.22, 0.25, 0.2),
    };

    b::SphereOptions sopts = b::SphereOptions::create().diameter(BALL_RADIUS * 2.0).segments(24.0);
    for (int i = 0; i < BALL_COUNT; i++) {
        b::Mesh m = b::createSphere(engine, sopts);
        m.material(i == RACK_COUNT ? cue_mat : rack_mats[i % 4]);
        b::addToScene(scene, m);
        st->ball_meshes.push_back(m);
    }
    sync_meshes(st);

    b::onBeforeRender(scene, [st](double) {
        if (!st->paused) step_sim(st);
    });

    if (auto reg_res = co_await b::registerScene(scene); !reg_res) {
        set_status_error(reg_res.error().message());
        co_return;
    }
    if (auto start_res = co_await b::startEngine(engine); !start_res) {
        set_status_error(start_res.error().message());
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
