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
    clear.g(0.07);
    clear.b(0.11);
    clear.a(1.0);
    scene.clearColor(clear);

    babylonjs_lite::ArcRotateCamera camera = babylonjs_lite::createArcRotateCamera(
        PI / 4.0,
        PI / 3.0,
        8.5,
        babylonjs_lite::vec3(0.0, 1.6, 0.0)
    );
    babylonjs_lite::addToScene(scene, camera);
    babylonjs_lite::attachControl(camera, canvas, scene);

    babylonjs_lite::HemisphericLight hemi = babylonjs_lite::createHemisphericLight(
        ts::Tuple<double, double, double>(0.0, 1.0, 0.2),
        0.75
    );
    hemi.diffuseColor(ts::Tuple<double, double, double>(0.75, 0.85, 1.0));
    hemi.groundColor(ts::Tuple<double, double, double>(0.12, 0.14, 0.22));
    babylonjs_lite::addToScene(scene, hemi);

    babylonjs_lite::DirectionalLight dir = babylonjs_lite::createDirectionalLight(
        ts::Tuple<double, double, double>(-1.0, -2.0, -1.2),
        2.2
    );
    dir.diffuse(ts::Tuple<double, double, double>(1.0, 0.96, 0.9));
    babylonjs_lite::addToScene(scene, dir);

    babylonjs_lite::PointLight orb_light = babylonjs_lite::createPointLight(
        ts::Tuple<double, double, double>(2.8, 1.6, 0.0),
        8.0
    );
    orb_light.diffuse(ts::Tuple<double, double, double>(1.0, 0.45, 0.35));
    orb_light.range(12.0);
    babylonjs_lite::addToScene(scene, orb_light);

    babylonjs_lite::createPbrMaterial_props knot_props = babylonjs_lite::createPbrMaterial_props::create();
    knot_props.baseColorFactor(ts::Tuple<double, double, double, double>(0.18, 0.78, 0.95, 1.0));
    knot_props.metallicFactor(0.75);
    knot_props.roughnessFactor(0.22);
    babylonjs_lite::PbrMaterialProps knot_mat = babylonjs_lite::createPbrMaterial(knot_props);

    babylonjs_lite::TorusKnotOptions knot_opts = babylonjs_lite::TorusKnotOptions::create();
    knot_opts.radius(1.15);
    knot_opts.tube(0.34);
    knot_opts.radialSegments(48.0);
    knot_opts.tubularSegments(160.0);
    babylonjs_lite::Mesh knot = babylonjs_lite::createTorusKnot(engine, knot_opts);
    knot.position().set(0.0, 2.0, 0.0);
    knot.material(knot_mat);
    babylonjs_lite::addToScene(scene, knot);

    babylonjs_lite::createPbrMaterial_props orb_props = babylonjs_lite::createPbrMaterial_props::create();
    orb_props.baseColorFactor(ts::Tuple<double, double, double, double>(1.0, 0.42, 0.32, 1.0));
    orb_props.metallicFactor(0.2);
    orb_props.roughnessFactor(0.35);
    babylonjs_lite::PbrMaterialProps orb_mat = babylonjs_lite::createPbrMaterial(orb_props);

    babylonjs_lite::SphereOptions orb_opts = babylonjs_lite::SphereOptions::create();
    orb_opts.diameter(0.55);
    orb_opts.segments(32.0);
    babylonjs_lite::Mesh orb = babylonjs_lite::createSphere(engine, orb_opts);
    orb.material(orb_mat);
    babylonjs_lite::addToScene(scene, orb);

    babylonjs_lite::ShaderMaterial ground_mat = babylonjs_lite::createGridMaterial(
        babylonjs_lite::GridMaterialOptions::create()
    );
    babylonjs_lite::GroundOptions ground_opts = babylonjs_lite::GroundOptions::create();
    ground_opts.width(16.0);
    ground_opts.height(16.0);
    babylonjs_lite::Mesh ground = babylonjs_lite::createGround(engine, ground_opts);
    ground.material(ground_mat);
    babylonjs_lite::addToScene(scene, ground);

    babylonjs_lite::EulerProxy knot_rot = knot.rotation();
    babylonjs_lite::ObservableVec3 orb_pos = orb.position();
    babylonjs_lite::ObservableVec3 orb_light_pos = orb_light.position();
    double elapsed = 0.0;

    babylonjs_lite::onBeforeRender(
        scene,
        babylonjs_lite::number_fn([knot_rot, orb_pos, orb_light_pos, elapsed](double delta_ms) mutable {
            elapsed += delta_ms * 0.001;
            knot_rot.y(elapsed * 0.55);
            knot_rot.x(std::sin(elapsed * 0.4) * 0.25);
            double x = std::cos(elapsed * 0.9) * 2.8;
            double y = 2.0 + std::sin(elapsed * 1.8) * 0.45;
            double z = std::sin(elapsed * 0.9) * 2.8;
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
