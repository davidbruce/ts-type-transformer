#include <cmath>
#include <string>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

using namespace babylonjs_lite;

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

    auto eng_res = co_await createEngine(canvas);
    if (!eng_res) {
        set_status_error(eng_res.error().message());
        co_return;
    }
    EngineContext engine = *eng_res;

    SceneContext scene = createSceneContext(engine);
    scene.clearColor(dom::GPUColorDict::create().r(0.05).g(0.07).b(0.11).a(1.0));

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.0, 8.5, vec3(0.0, 1.6, 0.0));
    addToScene(scene, camera);
    attachControl(camera, canvas, scene);

    HemisphericLight hemi = createHemisphericLight({0.0, 1.0, 0.2}, 0.75);
    hemi.diffuseColor({0.75, 0.85, 1.0});
    hemi.groundColor({0.12, 0.14, 0.22});
    addToScene(scene, hemi);

    DirectionalLight dir = createDirectionalLight({-1.0, -2.0, -1.2}, 2.2);
    dir.diffuse({1.0, 0.96, 0.9});
    addToScene(scene, dir);

    PointLight orb_light = createPointLight({2.8, 1.6, 0.0}, 8.0);
    orb_light.diffuse({1.0, 0.45, 0.35});
    orb_light.range(12.0);
    addToScene(scene, orb_light);

    PbrMaterialProps knot_mat = createPbrMaterial({{.baseColorFactor = ts::Tuple(0.18, 0.78, 0.95, 1.0), .metallicFactor = 0.75, .roughnessFactor = 0.22}});
    Mesh knot = createTorusKnot(engine, {{.radius = 1.15, .tube = 0.34, .radialSegments = 48.0, .tubularSegments = 160.0}});
    knot.position().set(0.0, 2.0, 0.0);
    knot.material(knot_mat);
    addToScene(scene, knot);

    PbrMaterialProps orb_mat = createPbrMaterial({{.baseColorFactor = ts::Tuple(1.0, 0.42, 0.32, 1.0), .metallicFactor = 0.2, .roughnessFactor = 0.35}});
    Mesh orb = createSphere(engine, {{.segments = 32.0, .diameter = 0.55}});
    orb.material(orb_mat);
    addToScene(scene, orb);

    ShaderMaterial ground_mat = createGridMaterial(GridMaterialOptions::create());
    Mesh ground = createGround(engine, {{.width = 16.0, .height = 16.0}});
    ground.material(ground_mat);
    addToScene(scene, ground);

    EulerProxy knot_rot = knot.rotation();
    ObservableVec3 orb_pos = orb.position();
    ObservableVec3 orb_light_pos = orb_light.position();
    double elapsed = 0.0;

    onBeforeRender(scene, [knot_rot, orb_pos, orb_light_pos, elapsed](double delta_ms) mutable {
        elapsed += delta_ms * 0.001;
        knot_rot.y(elapsed * 0.55);
        knot_rot.x(std::sin(elapsed * 0.4) * 0.25);
        double x = std::cos(elapsed * 0.9) * 2.8;
        double y = 2.0 + std::sin(elapsed * 1.8) * 0.45;
        double z = std::sin(elapsed * 0.9) * 2.8;
        orb_pos.set(x, y, z);
        orb_light_pos.set(x, y, z);
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
