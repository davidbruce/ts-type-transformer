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

static constexpr int DOM_COUNT = 14;
static constexpr int CYL_COUNT = 3;
static constexpr double DOM_HX = 0.09;
static constexpr double DOM_HY = 0.65;
static constexpr double DOM_HZ = 0.34;
static constexpr double MARBLE_RADIUS = 0.36;
static constexpr double RAMP_ANGLE = -0.38;
static constexpr double RAMP_X = -3.55;
static constexpr double RAMP_Y = 1.02;
static constexpr double RAMP_Z = -2.5;

struct Pose { double x, y, z, yaw; };

static Pose spiral_pose(int i) {
    double t = double(i) * 0.255;
    double r = 2.6 - double(i) * 0.045;
    double x = -1.35 + r * std::sin(t);
    double z = -2.5 + 2.6 - r * std::cos(t);
    return { x, DOM_HY, z, -t };
}

struct AppState {
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<rp::World> world;
    std::optional<rp::RigidBody> marble;
    std::vector<rp::RigidBody> doms;
    std::vector<rp::RigidBody> cyls;
    std::optional<b::Mesh> marble_mesh;
    std::vector<b::Mesh> dom_meshes;
    std::vector<b::Mesh> cyl_meshes;
    bool paused = false;
    int step_count = 0;
};

static void set_status_error(std::string_view message) {
    if (auto el = dom::get_document().getElementById("status")) {
        el->textContent("WebGPU / Babylon Lite error: " + std::string(message));
    }
}

static void build_world(const std::shared_ptr<AppState>& st) {
    if (st->world) st->world->free();
    st->world = rp::World::create(rp::Vector3::create(0.0, -9.81, 0.0));

    rp::RigidBody floor_body = st->world->createRigidBody(rp::RigidBodyDesc::fixed().setTranslation(0.0, -0.3, 0.0));
    st->world->createCollider(rp::ColliderDesc::cuboid(10.0, 0.3, 10.0).setFriction(0.75).setRestitution(0.1), floor_body);

    rp::Quaternion ramp_rot = rp::Quaternion::create(0.0, 0.0, std::sin(RAMP_ANGLE * 0.5), std::cos(RAMP_ANGLE * 0.5));
    rp::RigidBody ramp_body = st->world->createRigidBody(
        rp::RigidBodyDesc::fixed().setTranslation(RAMP_X, RAMP_Y, RAMP_Z).setRotation(ramp_rot)
    );
    st->world->createCollider(rp::ColliderDesc::cuboid(1.75, 0.10, 0.65).setFriction(0.15).setRestitution(0.2), ramp_body);

    st->marble = st->world->createRigidBody(
        rp::RigidBodyDesc::dynamic().setTranslation(-4.75, 2.02, -2.5).setLinvel(3.0, 0.0, 0.0)
    );
    st->world->createCollider(
        rp::ColliderDesc::ball(MARBLE_RADIUS).setDensity(3.2).setRestitution(0.32).setFriction(0.18),
        *st->marble
    );

    st->doms.clear();
    for (int i = 0; i < DOM_COUNT; i++) {
        Pose pose = spiral_pose(i);
        rp::Quaternion q = rp::Quaternion::create(0.0, std::sin(pose.yaw * 0.5), 0.0, std::cos(pose.yaw * 0.5));
        rp::RigidBody rb = st->world->createRigidBody(
            rp::RigidBodyDesc::dynamic().setTranslation(pose.x, pose.y, pose.z).setRotation(q)
        );
        st->world->createCollider(
            rp::ColliderDesc::cuboid(DOM_HX, DOM_HY, DOM_HZ).setFriction(0.55).setRestitution(0.18).setDensity(1.0),
            rb
        );
        st->doms.push_back(rb);
    }

    st->cyls.clear();
    Pose end_pose = spiral_pose(DOM_COUNT - 1);
    double fwd_x = std::cos(-end_pose.yaw);
    double fwd_z = std::sin(-end_pose.yaw);
    double perp_x = -fwd_z;
    double perp_z = fwd_x;
    for (int j = 0; j < CYL_COUNT; j++) {
        double spread = double(j - 1) * 0.52;
        double cx = end_pose.x + fwd_x * 0.72 + perp_x * spread;
        double cz = end_pose.z + fwd_z * 0.72 + perp_z * spread;
        rp::RigidBody rb = st->world->createRigidBody(rp::RigidBodyDesc::dynamic().setTranslation(cx, 0.42, cz));
        st->world->createCollider(
            rp::ColliderDesc::cylinder(0.42, 0.25).setFriction(0.55).setRestitution(0.25).setDensity(1.0),
            rb
        );
        st->cyls.push_back(rb);
    }

    st->step_count = 0;
}

static void sync_pair(const rp::RigidBody& rb, const b::Mesh& mesh) {
    rp::Vector p = rb.translation();
    rp::Rotation q = rb.rotation();
    mesh.position().set(p.x(), p.y(), p.z());
    mesh.rotationQuaternion().set(q.x(), q.y(), q.z(), q.w());
}

static void sync_meshes(const std::shared_ptr<AppState>& st) {
    if (st->marble && st->marble_mesh) sync_pair(*st->marble, *st->marble_mesh);
    for (size_t i = 0; i < st->doms.size() && i < st->dom_meshes.size(); i++) {
        sync_pair(st->doms[i], st->dom_meshes[i]);
    }
    for (size_t j = 0; j < st->cyls.size() && j < st->cyl_meshes.size(); j++) {
        sync_pair(st->cyls[j], st->cyl_meshes[j]);
    }
}

static void update_meta(const std::shared_ptr<AppState>& st) {
    int fallen = 0;
    double ke = 0.0;
    if (st->marble) {
        rp::Vector mv = st->marble->linvel();
        ke += 0.5 * (mv.x() * mv.x() + mv.y() * mv.y() + mv.z() * mv.z());
    }
    for (const auto& rb : st->doms) {
        rp::Vector p = rb.translation();
        rp::Vector v = rb.linvel();
        if (p.y() < 0.48) fallen++;
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    for (const auto& rb : st->cyls) {
        rp::Vector v = rb.linvel();
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    char buf[256];
    std::snprintf(
        buf, sizeof buf,
        "18 bodies (marble + 14 dominoes + 3 cylinders) \xC2\xB7 fallen=%d \xC2\xB7 ke=%.1f \xC2\xB7 step=%d (%s)",
        fallen, ke, st->step_count, st->paused ? "paused" : "running"
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

    build_world(st);
    for (int i = 0; i < 20; i++) {
        st->world->step();
        st->step_count++;
    }
    update_meta(st);

    const char* ids[4] = { "btn-launch", "btn-nudge", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                if (st->marble) {
                    st->marble->setTranslation(rp::Vector3::create(-4.75, 2.02, -2.5), true);
                    st->marble->setLinvel(rp::Vector3::create(3.0, 0.0, 0.0), true);
                    st->marble->setAngvel(rp::Vector3::create(0.0, 0.0, 0.0), true);
                }
                sync_meshes(st);
                update_meta(st);
            } else if (i == 1) {
                if (!st->doms.empty()) {
                    st->doms[0].applyImpulse(rp::Vector3::create(1.6, 0.0, 0.0), true);
                }
                update_meta(st);
            } else if (i == 2) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                update_meta(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_world(st);
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
    scene.clearColor(dom::GPUColorDict::create().r(0.06).g(0.07).b(0.09).a(1.0));

    b::ArcRotateCamera camera = b::createArcRotateCamera(0.65, 1.05, 12.0, b::vec3(-0.4, 0.8, 0.0));
    b::addToScene(scene, camera);
    b::attachControl(camera, canvas, scene);

    b::HemisphericLight hemi = b::createHemisphericLight({0.0, 1.0, 0.2}, 0.72);
    hemi.diffuseColor({0.85, 0.88, 0.95});
    hemi.groundColor({0.14, 0.14, 0.16});
    b::addToScene(scene, hemi);

    b::DirectionalLight dir = b::createDirectionalLight({-1.0, -2.0, -1.2}, 2.1);
    dir.diffuse({1.0, 0.97, 0.92});
    b::addToScene(scene, dir);

    b::Mesh ground = b::createGround(engine, {{.width = 20.0, .height = 20.0}});
    ground.material(b::createGridMaterial(b::GridMaterialOptions::create()));
    b::addToScene(scene, ground);

    auto mk_mat = [](double r, double g, double bl, double m, double ro) {
        return b::createPbrMaterial({{.baseColorFactor = ts::Tuple(r, g, bl, 1.0), .metallicFactor = m, .roughnessFactor = ro}});
    };

    b::Mesh ramp_mesh = b::createBox(engine, b::BoxOptions({.width = 3.5, .height = 0.2, .depth = 1.3}));
    ramp_mesh.position().set(RAMP_X, RAMP_Y, RAMP_Z);
    ramp_mesh.rotationQuaternion().set(0.0, 0.0, std::sin(RAMP_ANGLE * 0.5), std::cos(RAMP_ANGLE * 0.5));
    ramp_mesh.material(mk_mat(0.28, 0.30, 0.34, 0.35, 0.48));
    b::addToScene(scene, ramp_mesh);

    b::Mesh mb = b::createSphere(engine, {{.segments = 24.0, .diameter = MARBLE_RADIUS * 2.0}});
    mb.material(mk_mat(0.95, 0.42, 0.28, 0.55, 0.2));
    b::addToScene(scene, mb);
    st->marble_mesh = mb;

    b::PbrMaterialProps mat_a = mk_mat(0.88, 0.88, 0.88, 0.15, 0.4);
    b::PbrMaterialProps mat_b = mk_mat(0.45, 0.46, 0.48, 0.25, 0.38);
    b::BoxOptions dom_opts = b::BoxOptions({.width = DOM_HX * 2.0, .height = DOM_HY * 2.0, .depth = DOM_HZ * 2.0});
    for (int i = 0; i < DOM_COUNT; i++) {
        b::Mesh m = b::createBox(engine, dom_opts);
        m.material((i % 2 == 0) ? mat_a : mat_b);
        b::addToScene(scene, m);
        st->dom_meshes.push_back(m);
    }

    b::PbrMaterialProps cyl_mat = mk_mat(0.26, 0.62, 0.88, 0.45, 0.25);
    b::CylinderOptions cyl_opts = b::CylinderOptions({.height = 0.84, .diameter = 0.5, .tessellation = 24.0});
    for (int j = 0; j < CYL_COUNT; j++) {
        b::Mesh m = b::createCylinder(engine, cyl_opts);
        m.material(cyl_mat);
        b::addToScene(scene, m);
        st->cyl_meshes.push_back(m);
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
