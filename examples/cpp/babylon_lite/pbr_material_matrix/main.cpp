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

static PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    return createPbrMaterial(
        createPbrMaterial_props::create()
            .baseColorFactor({v, v, v, 1.0})
            .metallicFactor(metallic)
            .roughnessFactor(roughness)
    );
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
    scene.clearColor(dom::GPUColorDict::create().r(0.06).g(0.06).b(0.06).a(1.0));

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.1, 12.0, vec3(0.0, 1.1, 0.0));
    addToScene(scene, camera);
    attachControl(camera, canvas, scene);

    HemisphericLight hemi = createHemisphericLight({0.0, 1.0, 0.2}, 0.65);
    hemi.diffuseColor({0.82, 0.82, 0.82});
    hemi.groundColor({0.12, 0.12, 0.12});
    addToScene(scene, hemi);

    DirectionalLight dir = createDirectionalLight({-1.0, -2.0, -1.0}, 1.8);
    dir.diffuse({0.95, 0.95, 0.95});
    addToScene(scene, dir);

    PointLight orb_light = createPointLight({4.5, 3.0, 0.0}, 12.0);
    orb_light.diffuse({1.0, 1.0, 1.0});
    orb_light.range(18.0);
    addToScene(scene, orb_light);

    Mesh floor_m = createGround(engine, GroundOptions::create().width(14.0).height(14.0));
    floor_m.material(mk_pbr(0.11, 0.08, 0.88));
    addToScene(scene, floor_m);

    Mesh plinth = createBox(engine, BoxOptions::create().width(8.4).height(0.08).depth(8.4));
    plinth.position().set(0.0, 0.04, 0.0);
    plinth.material(mk_pbr(0.17, 0.12, 0.78));
    addToScene(scene, plinth);

    auto sphere_opts = SphereOptions::create().diameter(1.05).segments(24.0);
    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            double roughness = 0.08 + 0.22 * double(row);
            double metallic = 0.25 * double(col);
            Mesh sphere = createSphere(engine, sphere_opts);
            sphere.position().set(double(col - 2) * 1.55, 0.75, double(row - 2) * 1.55);
            sphere.material(mk_pbr(0.85, metallic, roughness));
            addToScene(scene, sphere);
        }
    }

    auto orb_props = createPbrMaterial_props::create()
        .baseColorFactor({0.95, 0.95, 0.95, 1.0})
        .metallicFactor(0.0)
        .roughnessFactor(0.2);
    setPbrEmissive(orb_props, {0.95, 0.95, 0.95});

    Mesh orb = createSphere(engine, SphereOptions::create().diameter(0.36).segments(16.0));
    orb.position().set(4.5, 3.0, 0.0);
    orb.material(createPbrMaterial(orb_props));
    addToScene(scene, orb);

    ObservableVec3 orb_pos = orb.position();
    ObservableVec3 orb_light_pos = orb_light.position();
    double elapsed = 0.0;

    onBeforeRender(scene, [orb_pos, orb_light_pos, elapsed](double delta_ms) mutable {
        elapsed += delta_ms * 0.001;
        double x = std::cos(elapsed * 0.85) * 4.5;
        double y = 3.0 + std::sin(elapsed * 1.4) * 0.65;
        double z = std::sin(elapsed * 0.85) * 4.5;
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
