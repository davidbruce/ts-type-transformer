#include <cmath>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

static constexpr double PI = 3.14159265358979323846;

static void set_status_error(std::string_view message) {
    if (auto el = dom::get_document().getElementById("status")) {
        el->textContent("WebGPU / Babylon Lite error: " + std::string(message));
    }
}

static babylonjs_lite::StandardMaterialProps mk_std(double diff, double spec, double power) {
    auto mat = babylonjs_lite::createStandardMaterial();
    mat.diffuseColor(ts::Tuple<double, double, double>(diff, diff, diff));
    mat.specularColor(ts::Tuple<double, double, double>(spec, spec, spec));
    mat.specularPower(power);
    return mat;
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
        PI / 2.0,
        PI / 2.8,
        11.5,
        babylonjs_lite::vec3(0.0, 1.6, 0.0)
    );
    babylonjs_lite::addToScene(scene, camera);
    babylonjs_lite::attachControl(camera, canvas, scene);

    babylonjs_lite::HemisphericLight hemi = babylonjs_lite::createHemisphericLight(
        ts::Tuple<double, double, double>(0.0, 1.0, 0.0),
        0.42
    );
    hemi.diffuseColor(ts::Tuple<double, double, double>(0.70, 0.70, 0.70));
    hemi.groundColor(ts::Tuple<double, double, double>(0.08, 0.08, 0.08));
    babylonjs_lite::addToScene(scene, hemi);

    babylonjs_lite::SpotLight spot = babylonjs_lite::createSpotLight(
        ts::Tuple<double, double, double>(0.0, 6.2, 3.0),
        ts::Tuple<double, double, double>(0.0, -1.0, -0.55),
        PI / 3.2,
        3.0,
        26.0
    );
    spot.diffuse(ts::Tuple<double, double, double>(1.0, 1.0, 1.0));
    spot.range(22.0);
    babylonjs_lite::addToScene(scene, spot);

    babylonjs_lite::DirectionalLight fill = babylonjs_lite::createDirectionalLight(
        ts::Tuple<double, double, double>(1.2, -1.5, -1.0),
        0.65
    );
    fill.diffuse(ts::Tuple<double, double, double>(0.80, 0.80, 0.80));
    babylonjs_lite::addToScene(scene, fill);

    auto floor_opts = babylonjs_lite::GroundOptions::create();
    floor_opts.width(18.0);
    floor_opts.height(18.0);
    babylonjs_lite::Mesh floor_m = babylonjs_lite::createGround(engine, floor_opts);
    floor_m.material(mk_std(0.10, 0.25, 24.0));
    babylonjs_lite::addToScene(scene, floor_m);

    auto plinth_mat = mk_std(0.18, 0.30, 32.0);
    std::vector<babylonjs_lite::ObservableVec3> poly_pos;
    std::vector<babylonjs_lite::EulerProxy> poly_rot;

    for (int i = 0; i < 5; i++) {
        double fi = double(i);
        double x = (fi - 2.0) * 2.25;
        double z = -std::fabs(fi - 2.0) * 0.55;

        auto copts = babylonjs_lite::CylinderOptions::create();
        copts.height(1.1);
        copts.diameter(1.15);
        copts.tessellation(28.0);
        babylonjs_lite::Mesh plinth = babylonjs_lite::createCylinder(engine, copts);
        plinth.position().set(x, 0.55, z);
        plinth.material(plinth_mat);
        babylonjs_lite::addToScene(scene, plinth);

        double v = 0.90 - fi * 0.13;
        auto popts = babylonjs_lite::PolyhedronOptions::create();
        popts.type(fi);
        popts.size(0.62);
        popts.flat(true);
        babylonjs_lite::Mesh mesh = babylonjs_lite::createPolyhedron(engine, popts);
        mesh.position().set(x, 2.05, z);
        mesh.material(mk_std(v, 0.95, 18.0 + fi * 24.0));
        babylonjs_lite::addToScene(scene, mesh);
        poly_pos.push_back(mesh.position());
        poly_rot.push_back(mesh.rotation());
    }

    babylonjs_lite::ObservableVec3 spot_pos = spot.position();
    babylonjs_lite::ObservableVec3 spot_dir = spot.direction();
    double elapsed = 0.0;

    babylonjs_lite::onBeforeRender(
        scene,
        babylonjs_lite::number_fn([spot_pos, spot_dir, poly_pos, poly_rot, elapsed](double delta_ms) mutable {
            elapsed += delta_ms * 0.001;
            double sx = std::sin(elapsed * 0.65) * 5.2;
            spot_pos.set(sx, 6.2, 3.0);
            spot_dir.set(-sx * 0.15, -1.0, -0.55);
            for (size_t i = 0; i < poly_pos.size(); i++) {
                double fi = double(i);
                poly_pos[i].y(2.05 + std::sin(elapsed * 1.3 + fi * 1.1) * 0.18);
                poly_rot[i].y(elapsed * (0.45 + 0.12 * fi));
                poly_rot[i].x(std::sin(elapsed * 0.7 + fi) * 0.35);
            }
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
