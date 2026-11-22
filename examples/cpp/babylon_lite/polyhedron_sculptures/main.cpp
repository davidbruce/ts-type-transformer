#include <cmath>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

using namespace babylonjs_lite;

static constexpr double PI = 3.14159265358979323846;

static void set_status_error(std::string_view message) {
    if (auto el = dom::get_document().getElementById("status")) {
        el->textContent("WebGPU / Babylon Lite error: " + std::string(message));
    }
}

static StandardMaterialProps mk_std(double diff, double spec, double power) {
    return createStandardMaterial()
        .diffuseColor({diff, diff, diff})
        .specularColor({spec, spec, spec})
        .specularPower(power);
}

static ts::Task run_app() {
    auto maybe_canvas = dom::get_document().querySelector(dom::HTMLElementTagNameMapKeys::canvas);
    if (!maybe_canvas) {
        set_status_error("missing <canvas> element");
        co_return;
    }
    dom::HTMLCanvasElement canvas = *maybe_canvas;

    auto eng_res = co_await createEngine(canvas);
    if (!eng_res) {
        set_status_error(eng_res.error().message());
        co_return;
    }
    EngineContext engine = *eng_res;

    SceneContext scene = createSceneContext(engine);
    scene.clearColor(dom::GPUColorDict::create().r(0.05).g(0.05).b(0.05).a(1.0));

    ArcRotateCamera camera = createArcRotateCamera(PI / 2.0, PI / 2.8, 11.5, vec3(0.0, 1.6, 0.0));
    addToScene(scene, camera);
    attachControl(camera, canvas, scene);

    HemisphericLight hemi = createHemisphericLight({0.0, 1.0, 0.0}, 0.42);
    hemi.diffuseColor({0.70, 0.70, 0.70});
    hemi.groundColor({0.08, 0.08, 0.08});
    addToScene(scene, hemi);

    SpotLight spot = createSpotLight({0.0, 6.2, 3.0}, {0.0, -1.0, -0.55}, PI / 3.2, 3.0, 26.0);
    spot.diffuse({1.0, 1.0, 1.0});
    spot.range(22.0);
    addToScene(scene, spot);

    DirectionalLight fill = createDirectionalLight({1.2, -1.5, -1.0}, 0.65);
    fill.diffuse({0.80, 0.80, 0.80});
    addToScene(scene, fill);

    Mesh floor_m = createGround(engine, {{.width = 18.0, .height = 18.0}});
    floor_m.material(mk_std(0.10, 0.25, 24.0));
    addToScene(scene, floor_m);

    auto plinth_mat = mk_std(0.18, 0.30, 32.0);
    auto plinth_opts = CylinderOptions({.height = 1.1, .diameter = 1.15, .tessellation = 28.0});
    std::vector<ObservableVec3> poly_pos;
    std::vector<EulerProxy> poly_rot;

    for (int i = 0; i < 5; i++) {
        double fi = double(i);
        double x = (fi - 2.0) * 2.25;
        double z = -std::fabs(fi - 2.0) * 0.55;

        Mesh plinth = createCylinder(engine, plinth_opts);
        plinth.position().set(x, 0.55, z);
        plinth.material(plinth_mat);
        addToScene(scene, plinth);

        double v = 0.90 - fi * 0.13;
        Mesh mesh = createPolyhedron(engine, {{.type = fi, .size = 0.62, .flat = true}});
        mesh.position().set(x, 2.05, z);
        mesh.material(mk_std(v, 0.95, 18.0 + fi * 24.0));
        addToScene(scene, mesh);
        poly_pos.push_back(mesh.position());
        poly_rot.push_back(mesh.rotation());
    }

    ObservableVec3 spot_pos = spot.position();
    ObservableVec3 spot_dir = spot.direction();
    double elapsed = 0.0;

    onBeforeRender(scene, [spot_pos, spot_dir, poly_pos, poly_rot, elapsed](double delta_ms) mutable {
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
    });

    if (auto reg_res = co_await registerScene(scene); !reg_res) {
        set_status_error(reg_res.error().message());
        co_return;
    }
    if (auto start_res = co_await startEngine(engine); !start_res) {
        set_status_error(start_res.error().message());
    }
}

TS_EXPORT(start) void start() {
    run_app();
}
