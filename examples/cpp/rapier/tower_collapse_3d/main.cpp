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

static constexpr int LAYERS = 6;
static constexpr int PER_LAYER = 3;
static constexpr int PLANK_COUNT = LAYERS * PER_LAYER;
static constexpr double HALF_LONG = 0.85;
static constexpr double HALF_HEIGHT = 0.22;
static constexpr double HALF_THIN = 0.26;
static constexpr double SPHERE_RADIUS = 0.65;

struct AppState {
    dom::HTMLElement meta;
    dom::HTMLElement pause_btn;
    std::optional<rp::World> world;
    std::vector<rp::RigidBody> planks;
    std::optional<rp::RigidBody> striker;
    std::vector<b::Mesh> plank_meshes;
    std::optional<b::Mesh> striker_mesh;
    bool paused = false;
    int step_count = 0;
    int shot_idx = 0;
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

    st->planks.clear();
    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        int slot = idx % PER_LAYER;
        double offset = double(slot - 1) * 0.57;
        double y = HALF_HEIGHT + double(layer) * 0.445;
        double x = (layer % 2 == 0) ? 0.0 : offset;
        double z = (layer % 2 == 0) ? offset : 0.0;
        double hx = (layer % 2 == 0) ? HALF_LONG : HALF_THIN;
        double hz = (layer % 2 == 0) ? HALF_THIN : HALF_LONG;
        rp::RigidBody rb = st->world->createRigidBody(rp::RigidBodyDesc::dynamic().setTranslation(x, y, z));
        st->world->createCollider(
            rp::ColliderDesc::cuboid(hx, HALF_HEIGHT, hz).setFriction(0.65).setRestitution(0.12).setDensity(1.0),
            rb
        );
        st->planks.push_back(rb);
    }

    st->striker = st->world->createRigidBody(
        rp::RigidBodyDesc::dynamic().setTranslation(-6.2, 1.55, 0.12).setLinvel(13.5, 1.1, -0.25)
    );
    st->world->createCollider(
        rp::ColliderDesc::ball(SPHERE_RADIUS).setDensity(4.5).setRestitution(0.35).setFriction(0.3),
        *st->striker
    );
    st->step_count = 0;
}

static void sync_meshes(const std::shared_ptr<AppState>& st) {
    for (size_t i = 0; i < st->planks.size() && i < st->plank_meshes.size(); i++) {
        rp::Vector p = st->planks[i].translation();
        rp::Rotation q = st->planks[i].rotation();
        st->plank_meshes[i].position().set(p.x(), p.y(), p.z());
        st->plank_meshes[i].rotationQuaternion().set(q.x(), q.y(), q.z(), q.w());
    }
    if (st->striker && st->striker_mesh) {
        rp::Vector sp = st->striker->translation();
        rp::Rotation sq = st->striker->rotation();
        st->striker_mesh->position().set(sp.x(), sp.y(), sp.z());
        st->striker_mesh->rotationQuaternion().set(sq.x(), sq.y(), sq.z(), sq.w());
    }
}

static void update_meta(const std::shared_ptr<AppState>& st) {
    double max_y = 0.0;
    double ke = 0.0;
    for (const auto& rb : st->planks) {
        rp::Vector p = rb.translation();
        rp::Vector v = rb.linvel();
        if (p.y() > max_y) max_y = p.y();
        ke += 0.5 * (v.x() * v.x() + v.y() * v.y() + v.z() * v.z());
    }
    if (st->striker) {
        rp::Vector sv = st->striker->linvel();
        ke += 2.25 * (sv.x() * sv.x() + sv.y() * sv.y() + sv.z() * sv.z());
    }
    char buf[256];
    std::snprintf(
        buf, sizeof buf,
        "19 bodies (18 planks + sphere) \xC2\xB7 max_y=%.2f m \xC2\xB7 ke=%.1f \xC2\xB7 step=%d (%s)",
        max_y, ke, st->step_count, st->paused ? "paused" : "running"
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
    for (int i = 0; i < 18; i++) {
        st->world->step();
        st->step_count++;
    }
    update_meta(st);

    const char* ids[3] = { "btn-launch", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) {
                st->shot_idx++;
                double angle = double(st->shot_idx) * 0.9;
                double dist = 6.2;
                double sx = -dist * std::cos(angle);
                double sz = dist * std::sin(angle);
                double sy = 1.2 + double(st->shot_idx % 3) * 0.45;
                if (st->striker) {
                    st->striker->setTranslation(rp::Vector3::create(sx, sy, sz), true);
                    st->striker->setLinvel(rp::Vector3::create(14.4 * std::cos(angle), 0.9, -14.4 * std::sin(angle)), true);
                    st->striker->setAngvel(rp::Vector3::create(0.0, 0.0, 0.0), true);
                }
                sync_meshes(st);
                update_meta(st);
            } else if (i == 1) {
                st->paused = !st->paused;
                st->pause_btn.textContent(st->paused ? "Resume" : "Pause");
                update_meta(st);
            } else {
                st->paused = false;
                st->pause_btn.textContent("Pause");
                build_world(st);
                for (int k = 0; k < 12; k++) {
                    st->world->step();
                    st->step_count++;
                }
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

    b::ArcRotateCamera camera = b::createArcRotateCamera(0.85, 1.12, 11.5, b::vec3(0.0, 1.5, 0.0));
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
        return b::createPbrMaterial({{.baseColorFactor = {r, g, bl, 1.0}, .metallicFactor = m, .roughnessFactor = ro}});
    };
    b::PbrMaterialProps palette[3] = {
        mk_mat(0.86, 0.86, 0.86, 0.15, 0.45),
        mk_mat(0.58, 0.58, 0.60, 0.25, 0.38),
        mk_mat(0.30, 0.31, 0.33, 0.35, 0.35),
    };

    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        double w_box = (layer % 2 == 0) ? (HALF_LONG * 2.0) : (HALF_THIN * 2.0);
        double d_box = (layer % 2 == 0) ? (HALF_THIN * 2.0) : (HALF_LONG * 2.0);
        b::Mesh m = b::createBox(engine, b::BoxOptions({.width = w_box, .height = HALF_HEIGHT * 2.0, .depth = d_box}));
        m.material(palette[layer % 3]);
        b::addToScene(scene, m);
        st->plank_meshes.push_back(m);
    }

    b::Mesh sm = b::createSphere(engine, {{.segments = 24.0, .diameter = SPHERE_RADIUS * 2.0}});
    sm.material(mk_mat(0.95, 0.42, 0.28, 0.55, 0.22));
    b::addToScene(scene, sm);
    st->striker_mesh = sm;
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
