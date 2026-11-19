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
        PI / 4.2,
        PI / 2.9,
        8.8,
        babylonjs_lite::vec3(0.0, 2.1, 0.0)
    );
    babylonjs_lite::addToScene(scene, camera);
    babylonjs_lite::attachControl(camera, canvas, scene);

    babylonjs_lite::HemisphericLight hemi = babylonjs_lite::createHemisphericLight(
        ts::Tuple<double, double, double>(0.0, 1.0, 0.2),
        0.72
    );
    hemi.diffuseColor(ts::Tuple<double, double, double>(0.85, 0.85, 0.85));
    hemi.groundColor(ts::Tuple<double, double, double>(0.14, 0.14, 0.14));
    babylonjs_lite::addToScene(scene, hemi);

    babylonjs_lite::DirectionalLight dir = babylonjs_lite::createDirectionalLight(
        ts::Tuple<double, double, double>(-1.2, -2.0, -1.1),
        2.1
    );
    dir.diffuse(ts::Tuple<double, double, double>(1.0, 1.0, 1.0));
    babylonjs_lite::addToScene(scene, dir);

    babylonjs_lite::PointLight key_light = babylonjs_lite::createPointLight(
        ts::Tuple<double, double, double>(3.2, 3.6, 2.4),
        8.0
    );
    key_light.diffuse(ts::Tuple<double, double, double>(0.92, 0.92, 0.92));
    key_light.range(14.0);
    babylonjs_lite::addToScene(scene, key_light);

    auto floor_opts = babylonjs_lite::GroundOptions::create();
    floor_opts.width(16.0);
    floor_opts.height(16.0);
    babylonjs_lite::Mesh floor_m = babylonjs_lite::createGround(engine, floor_opts);
    floor_m.material(mk_pbr(0.09, 0.10, 0.88));
    babylonjs_lite::addToScene(scene, floor_m);

    auto disc_opts = babylonjs_lite::DiscOptions::create();
    disc_opts.radius(2.6);
    disc_opts.tessellation(48.0);
    babylonjs_lite::Mesh disc = babylonjs_lite::createDisc(engine, disc_opts);
    disc.rotation().x(PI / 2.0);
    disc.position().set(0.0, 0.03, 0.0);
    disc.material(mk_pbr(0.22, 0.25, 0.65));
    babylonjs_lite::addToScene(scene, disc);

    auto sbopts = babylonjs_lite::BoxOptions::create();
    sbopts.size(2.1);
    babylonjs_lite::Mesh src_box = babylonjs_lite::createBox(engine, babylonjs_lite::number_or_BoxOptions(sbopts));
    auto copts = babylonjs_lite::CylinderOptions::create();
    copts.height(3.0);
    copts.diameter(1.18);
    copts.tessellation(16.0);
    babylonjs_lite::Mesh cyl_y = babylonjs_lite::createCylinder(engine, copts);
    babylonjs_lite::Mesh cyl_x = babylonjs_lite::createCylinder(engine, copts);
    cyl_x.rotation().z(PI / 2.0);
    babylonjs_lite::Mesh cyl_z = babylonjs_lite::createCylinder(engine, copts);
    cyl_z.rotation().x(PI / 2.0);

    auto s1 = babylonjs_lite::csgSubtract(babylonjs_lite::createCsgFromMesh(src_box), babylonjs_lite::createCsgFromMesh(cyl_y));
    auto s2 = babylonjs_lite::csgSubtract(s1, babylonjs_lite::createCsgFromMesh(cyl_x));
    auto s3 = babylonjs_lite::csgSubtract(s2, babylonjs_lite::createCsgFromMesh(cyl_z));
    babylonjs_lite::Mesh monolith = babylonjs_lite::createMeshFromCsg(engine, s3, "monolith");
    monolith.position().set(0.0, 2.15, 0.0);

    auto mprops = babylonjs_lite::createPbrMaterial_props::create();
    mprops.baseColorFactor(ts::Tuple<double, double, double, double>(0.86, 0.86, 0.86, 1.0));
    mprops.metallicFactor(0.65);
    mprops.roughnessFactor(0.24);
    auto cc = babylonjs_lite::ClearCoatProps::create();
    cc.isEnabled(true);
    cc.intensity(0.90);
    cc.roughness(0.10);
    babylonjs_lite::setPbrClearCoat(mprops, cc);
    monolith.material(babylonjs_lite::createPbrMaterial(mprops));
    babylonjs_lite::addToScene(scene, monolith);

    auto cbopts = babylonjs_lite::BoxOptions::create();
    cbopts.size(0.82);
    babylonjs_lite::Mesh core_box = babylonjs_lite::createBox(engine, babylonjs_lite::number_or_BoxOptions(cbopts));
    auto csopts = babylonjs_lite::SphereOptions::create();
    csopts.diameter(1.02);
    csopts.segments(14.0);
    babylonjs_lite::Mesh core_sphere = babylonjs_lite::createSphere(engine, csopts);
    auto core_solid = babylonjs_lite::csgIntersect(babylonjs_lite::createCsgFromMesh(core_box), babylonjs_lite::createCsgFromMesh(core_sphere));
    babylonjs_lite::Mesh core = babylonjs_lite::createMeshFromCsg(engine, core_solid, "core");
    core.position().set(0.0, 2.15, 0.0);
    core.material(mk_pbr(0.18, 0.85, 0.16));
    babylonjs_lite::addToScene(scene, core);

    auto otopts = babylonjs_lite::TorusOptions::create();
    otopts.diameter(4.2);
    otopts.thickness(0.09);
    otopts.tessellation(48.0);
    babylonjs_lite::Mesh outer_ring = babylonjs_lite::createTorus(engine, otopts);
    outer_ring.position().set(0.0, 2.15, 0.0);
    outer_ring.material(mk_pbr(0.55, 0.80, 0.25));
    babylonjs_lite::addToScene(scene, outer_ring);

    auto itopts = babylonjs_lite::TorusOptions::create();
    itopts.diameter(3.55);
    itopts.thickness(0.07);
    itopts.tessellation(48.0);
    babylonjs_lite::Mesh inner_ring = babylonjs_lite::createTorus(engine, itopts);
    inner_ring.position().set(0.0, 2.15, 0.0);
    inner_ring.material(mk_pbr(0.88, 0.40, 0.30));
    babylonjs_lite::addToScene(scene, inner_ring);

    babylonjs_lite::EulerProxy mono_rot = monolith.rotation();
    babylonjs_lite::EulerProxy core_rot = core.rotation();
    babylonjs_lite::EulerProxy outer_rot = outer_ring.rotation();
    babylonjs_lite::EulerProxy inner_rot = inner_ring.rotation();
    double elapsed = 0.0;

    babylonjs_lite::onBeforeRender(
        scene,
        babylonjs_lite::number_fn([mono_rot, core_rot, outer_rot, inner_rot, elapsed](double delta_ms) mutable {
            elapsed += delta_ms * 0.001;
            mono_rot.y(elapsed * 0.35);
            mono_rot.x(std::sin(elapsed * 0.3) * 0.20);
            core_rot.y(-elapsed * 0.90);
            core_rot.z(elapsed * 0.50);
            outer_rot.x(elapsed * 0.42);
            outer_rot.y(elapsed * 0.25);
            inner_rot.z(-elapsed * 0.58);
            inner_rot.x(elapsed * 0.31);
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
