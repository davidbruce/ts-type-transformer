#include <cmath>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

static constexpr double PI = 3.14159265358979323846;

struct ColumnState {
    babylonjs_lite::ObservableVec3 pos;
    babylonjs_lite::ObservableVec3 scale;
    double x;
    double z;
    double dist;
    double diag;
};

static void set_status_error(std::string_view message) {
    if (auto el = dom::get_document().getElementById("status")) {
        el->textContent("WebGPU / Babylon Lite error: " + std::string(message));
    }
}

static babylonjs_lite::PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    auto props = babylonjs_lite::createPbrMaterial_props::create();
    props.baseColorFactor(ts::Tuple<double, double, double, double>(v, v, v, 1.0));
    props.metallicFactor(metallic);
    props.roughnessFactor(roughness);
    return babylonjs_lite::createPbrMaterial(props);
}

static ts::Task run_app() {
    auto maybe_canvas = dom::get_document().querySelector(dom::HTMLElementTagNameMapKeys::canvas);
    if (!maybe_canvas) {
        set_status_error("missing <canvas> element");
        co_return;
    }
    dom::HTMLCanvasElement canvas = *maybe_canvas;

    ts::Result<babylonjs_lite::EngineContext> eng_res = co_await babylonjs_lite::createEngine(canvas);
    if (!eng_res) {
        set_status_error(eng_res.error().message());
        co_return;
    }
    babylonjs_lite::EngineContext engine = *eng_res;

    babylonjs_lite::SceneContext scene = babylonjs_lite::createSceneContext(engine);
    dom::GPUColorDict clear = dom::GPUColorDict::create();
    clear.r(0.05);
    clear.g(0.05);
    clear.b(0.05);
    clear.a(1.0);
    scene.clearColor(clear);

    babylonjs_lite::ArcRotateCamera camera = babylonjs_lite::createArcRotateCamera(
        PI / 4.0,
        PI / 3.1,
        12.8,
        babylonjs_lite::vec3(0.0, 1.4, 0.0)
    );
    babylonjs_lite::addToScene(scene, camera);
    babylonjs_lite::attachControl(camera, canvas, scene);

    babylonjs_lite::HemisphericLight hemi = babylonjs_lite::createHemisphericLight(
        ts::Tuple<double, double, double>(0.0, 1.0, 0.15),
        0.55
    );
    hemi.diffuseColor(ts::Tuple<double, double, double>(0.76, 0.76, 0.76));
    hemi.groundColor(ts::Tuple<double, double, double>(0.10, 0.10, 0.10));
    babylonjs_lite::addToScene(scene, hemi);

    babylonjs_lite::DirectionalLight dir = babylonjs_lite::createDirectionalLight(
        ts::Tuple<double, double, double>(-1.0, -2.2, -0.8),
        1.35
    );
    dir.diffuse(ts::Tuple<double, double, double>(0.90, 0.90, 0.90));
    babylonjs_lite::addToScene(scene, dir);

    babylonjs_lite::PointLight light1 = babylonjs_lite::createPointLight(
        ts::Tuple<double, double, double>(5.2, 4.2, 0.0),
        10.0
    );
    light1.diffuse(ts::Tuple<double, double, double>(1.0, 1.0, 1.0));
    light1.range(16.0);
    babylonjs_lite::addToScene(scene, light1);

    babylonjs_lite::PointLight light2 = babylonjs_lite::createPointLight(
        ts::Tuple<double, double, double>(-5.2, 3.4, 0.0),
        7.5
    );
    light2.diffuse(ts::Tuple<double, double, double>(0.78, 0.78, 0.78));
    light2.range(16.0);
    babylonjs_lite::addToScene(scene, light2);

    auto floor_opts = babylonjs_lite::GroundOptions::create();
    floor_opts.width(18.0);
    floor_opts.height(18.0);
    babylonjs_lite::Mesh floor_m = babylonjs_lite::createGround(engine, floor_opts);
    floor_m.material(mk_pbr(0.09, 0.10, 0.88));
    babylonjs_lite::addToScene(scene, floor_m);

    const double shades[4] = { 0.22, 0.44, 0.68, 0.88 };
    std::vector<babylonjs_lite::PbrMaterialProps> col_mats;
    for (double v : shades) col_mats.push_back(mk_pbr(v, 0.35, 0.36));

    auto bopts = babylonjs_lite::BoxOptions::create();
    bopts.width(0.72);
    bopts.height(1.0);
    bopts.depth(0.72);

    std::vector<ColumnState> columns;
    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 6; col++) {
            double fx = double(col) - 2.5;
            double fz = double(row) - 2.5;
            double x = fx * 1.15;
            double z = fz * 1.15;
            babylonjs_lite::Mesh m = babylonjs_lite::createBox(engine, babylonjs_lite::number_or_BoxOptions(bopts));
            m.position().set(x, 0.5, z);
            m.material(col_mats[(row + col) % 4]);
            babylonjs_lite::addToScene(scene, m);
            columns.push_back(ColumnState{ m.position(), m.scaling(), x, z, std::sqrt(fx * fx + fz * fz), fx - fz });
        }
    }

    auto cap_props = babylonjs_lite::createPbrMaterial_props::create();
    cap_props.baseColorFactor(ts::Tuple<double, double, double, double>(0.92, 0.92, 0.92, 1.0));
    cap_props.metallicFactor(0.50);
    cap_props.roughnessFactor(0.15);
    auto cc = babylonjs_lite::ClearCoatProps::create();
    cc.isEnabled(true);
    cc.intensity(1.0);
    cc.roughness(0.08);
    babylonjs_lite::setPbrClearCoat(cap_props, cc);
    babylonjs_lite::setPbrEmissive(cap_props, ts::Tuple<double, double, double>(0.16, 0.16, 0.16));
    auto cap_mat = babylonjs_lite::createPbrMaterial(cap_props);

    auto copts = babylonjs_lite::CapsuleOptions::create();
    copts.height(1.15);
    copts.radius(0.22);
    copts.tessellation(20.0);
    std::vector<babylonjs_lite::ObservableVec3> cap_pos;
    std::vector<babylonjs_lite::EulerProxy> cap_rot;
    for (int k = 0; k < 4; k++) {
        babylonjs_lite::Mesh cap = babylonjs_lite::createCapsule(engine, copts);
        cap.material(cap_mat);
        babylonjs_lite::addToScene(scene, cap);
        cap_pos.push_back(cap.position());
        cap_rot.push_back(cap.rotation());
    }

    babylonjs_lite::ObservableVec3 l1_pos = light1.position();
    babylonjs_lite::ObservableVec3 l2_pos = light2.position();
    double elapsed = 0.0;

    babylonjs_lite::onBeforeRender(
        scene,
        babylonjs_lite::number_fn([columns, cap_pos, cap_rot, l1_pos, l2_pos, elapsed](double delta_ms) mutable {
            elapsed += delta_ms * 0.001;
            for (const auto& c : columns) {
                double h = 0.55
                    + 1.15 * (0.5 + 0.5 * std::sin(elapsed * 2.2 - c.dist * 1.15))
                    + 0.30 * std::cos(elapsed * 1.3 + c.diag * 0.7);
                c.scale.set(1.0, h, 1.0);
                c.pos.set(c.x, h * 0.5, c.z);
            }
            for (size_t k = 0; k < cap_pos.size(); k++) {
                double fk = double(k);
                double theta = elapsed * 0.7 + fk * (PI * 0.5);
                cap_pos[k].set(
                    std::cos(theta) * 4.9,
                    2.6 + std::sin(elapsed * 1.6 + fk * 1.57) * 0.55,
                    std::sin(theta) * 4.9
                );
                cap_rot[k].z(std::sin(elapsed + fk) * 0.35);
                cap_rot[k].y(-theta);
            }
            l1_pos.set(std::cos(elapsed * 0.65) * 5.2, 4.2, std::sin(elapsed * 0.65) * 5.2);
            l2_pos.set(std::cos(-elapsed * 0.52) * 5.2, 3.4, std::sin(-elapsed * 0.52) * 5.2);
        })
    );

    ts::Result<void> reg_res = co_await babylonjs_lite::registerScene(scene);
    if (!reg_res) {
        set_status_error(reg_res.error().message());
        co_return;
    }
    ts::Result<void> start_res = co_await babylonjs_lite::startEngine(engine);
    if (!start_res) {
        set_status_error(start_res.error().message());
    }
}

extern "C" __attribute__((export_name("start"))) void start() {
    run_app();
}
