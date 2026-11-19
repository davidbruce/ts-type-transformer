#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

static constexpr double PI = 3.14159265358979323846;

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
    clear.r(0.06);
    clear.g(0.06);
    clear.b(0.06);
    clear.a(1.0);
    scene.clearColor(clear);

    babylonjs_lite::ArcRotateCamera camera = babylonjs_lite::createArcRotateCamera(
        PI / 4.0,
        PI / 3.1,
        12.0,
        babylonjs_lite::vec3(0.0, 1.1, 0.0)
    );
    babylonjs_lite::addToScene(scene, camera);
    babylonjs_lite::attachControl(camera, canvas, scene);

    babylonjs_lite::HemisphericLight hemi = babylonjs_lite::createHemisphericLight(
        ts::Tuple<double, double, double>(0.0, 1.0, 0.2),
        0.65
    );
    hemi.diffuseColor(ts::Tuple<double, double, double>(0.82, 0.82, 0.82));
    hemi.groundColor(ts::Tuple<double, double, double>(0.12, 0.12, 0.12));
    babylonjs_lite::addToScene(scene, hemi);

    babylonjs_lite::DirectionalLight dir = babylonjs_lite::createDirectionalLight(
        ts::Tuple<double, double, double>(-1.0, -2.0, -1.0),
        1.8
    );
    dir.diffuse(ts::Tuple<double, double, double>(0.95, 0.95, 0.95));
    babylonjs_lite::addToScene(scene, dir);

    babylonjs_lite::PointLight orb_light = babylonjs_lite::createPointLight(
        ts::Tuple<double, double, double>(4.5, 3.0, 0.0),
        12.0
    );
    orb_light.diffuse(ts::Tuple<double, double, double>(1.0, 1.0, 1.0));
    orb_light.range(18.0);
    babylonjs_lite::addToScene(scene, orb_light);

    auto floor_opts = babylonjs_lite::GroundOptions::create();
    floor_opts.width(14.0);
    floor_opts.height(14.0);
    babylonjs_lite::Mesh floor_m = babylonjs_lite::createGround(engine, floor_opts);
    floor_m.material(mk_pbr(0.11, 0.08, 0.88));
    babylonjs_lite::addToScene(scene, floor_m);

    auto plinth_opts = babylonjs_lite::BoxOptions::create();
    plinth_opts.width(8.4);
    plinth_opts.height(0.08);
    plinth_opts.depth(8.4);
    babylonjs_lite::Mesh plinth = babylonjs_lite::createBox(engine, babylonjs_lite::number_or_BoxOptions(plinth_opts));
    plinth.position().set(0.0, 0.04, 0.0);
    plinth.material(mk_pbr(0.17, 0.12, 0.78));
    babylonjs_lite::addToScene(scene, plinth);

    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            double roughness = 0.08 + 0.22 * double(row);
            double metallic = 0.25 * double(col);
            auto sopts = babylonjs_lite::SphereOptions::create();
            sopts.diameter(1.05);
            sopts.segments(24.0);
            babylonjs_lite::Mesh sphere = babylonjs_lite::createSphere(engine, sopts);
            sphere.position().set(double(col - 2) * 1.55, 0.75, double(row - 2) * 1.55);
            sphere.material(mk_pbr(0.85, metallic, roughness));
            babylonjs_lite::addToScene(scene, sphere);
        }
    }

    auto orb_props = babylonjs_lite::createPbrMaterial_props::create();
    orb_props.baseColorFactor(ts::Tuple<double, double, double, double>(0.95, 0.95, 0.95, 1.0));
    orb_props.metallicFactor(0.0);
    orb_props.roughnessFactor(0.2);
    babylonjs_lite::setPbrEmissive(orb_props, ts::Tuple<double, double, double>(0.95, 0.95, 0.95));
    auto orb_mat = babylonjs_lite::createPbrMaterial(orb_props);

    auto orb_opts = babylonjs_lite::SphereOptions::create();
    orb_opts.diameter(0.36);
    orb_opts.segments(16.0);
    babylonjs_lite::Mesh orb = babylonjs_lite::createSphere(engine, orb_opts);
    orb.position().set(4.5, 3.0, 0.0);
    orb.material(orb_mat);
    babylonjs_lite::addToScene(scene, orb);

    babylonjs_lite::ObservableVec3 orb_pos = orb.position();
    babylonjs_lite::ObservableVec3 orb_light_pos = orb_light.position();
    double elapsed = 0.0;

    babylonjs_lite::onBeforeRender(
        scene,
        babylonjs_lite::number_fn([orb_pos, orb_light_pos, elapsed](double delta_ms) mutable {
            elapsed += delta_ms * 0.001;
            double x = std::cos(elapsed * 0.85) * 4.5;
            double y = 3.0 + std::sin(elapsed * 1.4) * 0.65;
            double z = std::sin(elapsed * 0.85) * 4.5;
            orb_pos.set(x, y, z);
            orb_light_pos.set(x, y, z);
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
