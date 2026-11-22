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
    return createPbrMaterial({{.baseColorFactor = {v, v, v, 1.0}, .metallicFactor = metallic, .roughnessFactor = roughness}});
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

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.2, PI / 2.9, 8.8, vec3(0.0, 2.1, 0.0));
    addToScene(scene, camera);
    attachControl(camera, canvas, scene);

    HemisphericLight hemi = createHemisphericLight({0.0, 1.0, 0.2}, 0.72);
    hemi.diffuseColor({0.85, 0.85, 0.85});
    hemi.groundColor({0.14, 0.14, 0.14});
    addToScene(scene, hemi);

    DirectionalLight dir = createDirectionalLight({-1.2, -2.0, -1.1}, 2.1);
    dir.diffuse({1.0, 1.0, 1.0});
    addToScene(scene, dir);

    PointLight key_light = createPointLight({3.2, 3.6, 2.4}, 8.0);
    key_light.diffuse({0.92, 0.92, 0.92});
    key_light.range(14.0);
    addToScene(scene, key_light);

    Mesh floor_m = createGround(engine, {{.width = 16.0, .height = 16.0}});
    floor_m.material(mk_pbr(0.09, 0.10, 0.88));
    addToScene(scene, floor_m);

    Mesh disc = createDisc(engine, {{.radius = 2.6, .tessellation = 48.0}});
    disc.rotation().x(PI / 2.0);
    disc.position().set(0.0, 0.03, 0.0);
    disc.material(mk_pbr(0.22, 0.25, 0.65));
    addToScene(scene, disc);

    Mesh src_box = createBox(engine, BoxOptions({.size = 2.1}));
    auto copts = CylinderOptions({.height = 3.0, .diameter = 1.18, .tessellation = 16.0});
    Mesh cyl_y = createCylinder(engine, copts);
    Mesh cyl_x = createCylinder(engine, copts);
    cyl_x.rotation().z(PI / 2.0);
    Mesh cyl_z = createCylinder(engine, copts);
    cyl_z.rotation().x(PI / 2.0);

    auto s1 = csgSubtract(createCsgFromMesh(src_box), createCsgFromMesh(cyl_y));
    auto s2 = csgSubtract(s1, createCsgFromMesh(cyl_x));
    auto s3 = csgSubtract(s2, createCsgFromMesh(cyl_z));
    Mesh monolith = createMeshFromCsg(engine, s3, "monolith");
    monolith.position().set(0.0, 2.15, 0.0);

    auto mprops = createPbrMaterial_props({.baseColorFactor = {0.86, 0.86, 0.86, 1.0}, .metallicFactor = 0.65, .roughnessFactor = 0.24});
    setPbrClearCoat(mprops, {{.isEnabled = true, .intensity = 0.90, .roughness = 0.10}});
    monolith.material(createPbrMaterial(mprops));
    addToScene(scene, monolith);

    Mesh core_box = createBox(engine, BoxOptions({.size = 0.82}));
    Mesh core_sphere = createSphere(engine, {{.segments = 14.0, .diameter = 1.02}});
    auto core_solid = csgIntersect(createCsgFromMesh(core_box), createCsgFromMesh(core_sphere));
    Mesh core = createMeshFromCsg(engine, core_solid, "core");
    core.position().set(0.0, 2.15, 0.0);
    core.material(mk_pbr(0.18, 0.85, 0.16));
    addToScene(scene, core);

    Mesh outer_ring = createTorus(engine, {{.diameter = 4.2, .thickness = 0.09, .tessellation = 48.0}});
    outer_ring.position().set(0.0, 2.15, 0.0);
    outer_ring.material(mk_pbr(0.55, 0.80, 0.25));
    addToScene(scene, outer_ring);

    Mesh inner_ring = createTorus(engine, {{.diameter = 3.55, .thickness = 0.07, .tessellation = 48.0}});
    inner_ring.position().set(0.0, 2.15, 0.0);
    inner_ring.material(mk_pbr(0.88, 0.40, 0.30));
    addToScene(scene, inner_ring);

    EulerProxy mono_rot = monolith.rotation();
    EulerProxy core_rot = core.rotation();
    EulerProxy outer_rot = outer_ring.rotation();
    EulerProxy inner_rot = inner_ring.rotation();
    double elapsed = 0.0;

    onBeforeRender(scene, [mono_rot, core_rot, outer_rot, inner_rot, elapsed](double delta_ms) mutable {
        elapsed += delta_ms * 0.001;
        mono_rot.y(elapsed * 0.35);
        mono_rot.x(std::sin(elapsed * 0.3) * 0.20);
        core_rot.y(-elapsed * 0.90);
        core_rot.z(elapsed * 0.50);
        outer_rot.x(elapsed * 0.42);
        outer_rot.y(elapsed * 0.25);
        inner_rot.z(-elapsed * 0.58);
        inner_rot.x(elapsed * 0.31);
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
