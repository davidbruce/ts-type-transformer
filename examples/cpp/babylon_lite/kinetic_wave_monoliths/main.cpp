#include <cmath>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "babylonjs_lite/babylonjs_lite.hpp"

using namespace babylonjs_lite;

static constexpr double PI = 3.14159265358979323846;

struct ColumnState {
    ObservableVec3 pos;
    ObservableVec3 scale;
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
    scene.clearColor(dom::GPUColorDict::create().r(0.05).g(0.05).b(0.05).a(1.0));

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.1, 12.8, vec3(0.0, 1.4, 0.0));
    addToScene(scene, camera);
    attachControl(camera, canvas, scene);

    HemisphericLight hemi = createHemisphericLight({0.0, 1.0, 0.15}, 0.55);
    hemi.diffuseColor({0.76, 0.76, 0.76});
    hemi.groundColor({0.10, 0.10, 0.10});
    addToScene(scene, hemi);

    DirectionalLight dir = createDirectionalLight({-1.0, -2.2, -0.8}, 1.35);
    dir.diffuse({0.90, 0.90, 0.90});
    addToScene(scene, dir);

    PointLight light1 = createPointLight({5.2, 4.2, 0.0}, 10.0);
    light1.diffuse({1.0, 1.0, 1.0});
    light1.range(16.0);
    addToScene(scene, light1);

    PointLight light2 = createPointLight({-5.2, 3.4, 0.0}, 7.5);
    light2.diffuse({0.78, 0.78, 0.78});
    light2.range(16.0);
    addToScene(scene, light2);

    Mesh floor_m = createGround(engine, {{.width = 18.0, .height = 18.0}});
    floor_m.material(mk_pbr(0.09, 0.10, 0.88));
    addToScene(scene, floor_m);

    const double shades[4] = { 0.22, 0.44, 0.68, 0.88 };
    std::vector<PbrMaterialProps> col_mats;
    for (double v : shades) col_mats.push_back(mk_pbr(v, 0.35, 0.36));

    auto bopts = BoxOptions({.width = 0.72, .height = 1.0, .depth = 0.72});

    std::vector<ColumnState> columns;
    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 6; col++) {
            double fx = double(col) - 2.5;
            double fz = double(row) - 2.5;
            double x = fx * 1.15;
            double z = fz * 1.15;
            Mesh m = createBox(engine, bopts);
            m.position().set(x, 0.5, z);
            m.material(col_mats[(row + col) % 4]);
            addToScene(scene, m);
            columns.push_back(ColumnState{ m.position(), m.scaling(), x, z, std::sqrt(fx * fx + fz * fz), fx - fz });
        }
    }

    auto cap_props = createPbrMaterial_props({.baseColorFactor = {0.92, 0.92, 0.92, 1.0}, .metallicFactor = 0.50, .roughnessFactor = 0.15});
    setPbrClearCoat(cap_props, {{.isEnabled = true, .intensity = 1.0, .roughness = 0.08}});
    setPbrEmissive(cap_props, {0.16, 0.16, 0.16});
    auto cap_mat = createPbrMaterial(cap_props);

    auto copts = CapsuleOptions({.height = 1.15, .radius = 0.22, .tessellation = 20.0});
    std::vector<ObservableVec3> cap_pos;
    std::vector<EulerProxy> cap_rot;
    for (int k = 0; k < 4; k++) {
        Mesh cap = createCapsule(engine, copts);
        cap.material(cap_mat);
        addToScene(scene, cap);
        cap_pos.push_back(cap.position());
        cap_rot.push_back(cap.rotation());
    }

    ObservableVec3 l1_pos = light1.position();
    ObservableVec3 l2_pos = light2.position();
    double elapsed = 0.0;

    onBeforeRender(scene, [columns, cap_pos, cap_rot, l1_pos, l2_pos, elapsed](double delta_ms) mutable {
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
