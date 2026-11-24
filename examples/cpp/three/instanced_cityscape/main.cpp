#include <algorithm>
#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "three/three.hpp"

using namespace three;

static double tower_height(int mode, int gx, int gz) {
    double fx = double(gx) - 4.5;
    double fz = double(gz) - 4.5;
    if (mode == 1) {
        double r = std::sqrt(fx * fx + fz * fz);
        return 0.35 + 0.42 * r + 0.25 * std::cos(fx * 0.8);
    } else if (mode == 2) {
        double diag = std::fabs(fx - fz);
        return 0.3 + 2.8 * std::exp(-diag * diag / 5.5) + 0.25 * std::sin(fz * 0.9);
    }
    double r2 = fx * fx + fz * fz;
    return 0.35 + 3.1 * std::exp(-r2 / 9.5) + 0.2 * std::cos(fx * 1.4) * std::cos(fz * 1.4);
}

static const char* layout_name(int mode) {
    if (mode == 1) return "Amphitheater";
    if (mode == 2) return "Diagonal";
    return "Plaza";
}

struct AppState {
    WebGLRenderer renderer;
    Scene<> scene;
    OrthographicCamera ortho_cam;
    PerspectiveCamera persp_cam;
    InstancedMesh<> inst;
    dom::HTMLElement cam_btn;
    dom::HTMLElement meta;
    bool use_ortho = true;
    int layout_mode = 0;
    double cam_yaw = 0.785398;
};

static void render_city(const std::shared_ptr<AppState>& st) {
    Vector3 pos = Vector3::create();
    Quaternion rot_q = Quaternion::create();
    Vector3 scale = Vector3::create();
    Matrix4 mtx = Matrix4::create();
    Vector3 axis_y = Vector3(0.0, 1.0, 0.0);
    Color col = Color::create();

    double max_h = 0.0;
    int idx = 0;
    for (int gz = 0; gz < 10; gz++) {
        for (int gx = 0; gx < 10; gx++) {
            double x = (double(gx) - 4.5) * 0.78;
            double z = (double(gz) - 4.5) * 0.78;
            double h = tower_height(st->layout_mode, gx, gz);
            if (h > max_h) max_h = h;
            pos.set(x, h * 0.5, z);
            rot_q.setFromAxisAngle(axis_y, 0.08 * double((gx + gz) % 4));
            scale.set(1.0, h, 1.0);
            mtx.compose(pos, rot_q, scale);
            st->inst.setMatrixAt(double(idx), mtx);

            double t = std::clamp((h - 0.3) / 3.2, 0.0, 1.0);
            double shade = 0.82 - 0.70 * t;
            col.setRGB(shade, shade, shade);
            st->inst.setColorAt(double(idx), col);
            idx++;
        }
    }

    st->inst.instanceMatrix().needsUpdate(true);
    if (auto ic = st->inst.instanceColor()) {
        ic->needsUpdate(true);
    }

    double radius = 11.5;
    double cx = radius * std::cos(st->cam_yaw);
    double cz = radius * std::sin(st->cam_yaw);
    Vector3 target = Vector3(0.0, 0.9, 0.0);
    st->ortho_cam.position().set(cx, 8.2, cz);
    st->ortho_cam.lookAt(target);
    st->persp_cam.position().set(cx, 7.6, cz);
    st->persp_cam.lookAt(target);

    if (st->use_ortho) {
        st->renderer.render(st->scene, st->ortho_cam);
    } else {
        st->renderer.render(st->scene, st->persp_cam);
    }

    const char* cname = st->use_ortho ? "Orthographic" : "Perspective";
    st->cam_btn.textContent(std::string("Camera: ") + cname);
    char buf[256];
    std::snprintf(buf, sizeof buf,
                  "100 InstancedMesh towers \xC2\xB7 layout: %s \xC2\xB7 peak height: %.2f \xC2\xB7 camera: %s",
                  layout_name(st->layout_mode), max_h, cname);
    st->meta.textContent(buf);
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    double width = 680.0, height = 400.0, aspect = width / height;

    WebGLRenderer renderer = WebGLRenderer::create({{.antialias = true, .preserveDrawingBuffer = true}});
    renderer.setPixelRatio(1.0);
    renderer.setSize(width, height);
    doc.getElementById("stage")->appendChild(renderer.domElement());

    Scene scene = Scene<>::create();
    scene.background(Color(0xf7f7f7));
    OrthographicCamera ortho_cam = OrthographicCamera(-5.4 * aspect, 5.4 * aspect, 5.4, -5.4, 0.1, 100.0);
    PerspectiveCamera persp_cam = PerspectiveCamera(40.0, aspect, 0.1, 100.0);

    DirectionalLight key = DirectionalLight(0xffffff, 2.1);
    key.position().set(8.0, 12.0, 6.0);
    DirectionalLight rim = DirectionalLight(0xffffff, 0.9);
    rim.position().set(-7.0, 6.0, -8.0);

    Mesh plinth = Mesh(
        PlaneGeometry(8.6, 8.6),
        MeshStandardMaterial::create({{.color = 0xe5e5e5, .roughness = 0.85}})
    );
    plinth.rotation().x(-M_PI / 2.0);
    plinth.position().y(-0.01);

    GridHelper grid = GridHelper(9.0, 10.0, 0x999999, 0xd4d4d4);

    InstancedMesh inst = InstancedMesh(
        BoxGeometry(0.54, 1.0, 0.54),
        MeshStandardMaterial::create({{.color = 0xffffff, .roughness = 0.65, .metalness = 0.08}}),
        100.0
    );
    scene.add(AmbientLight(0xffffff, 1.25), key, rim, plinth, grid, inst);

    auto st = std::make_shared<AppState>(AppState{
        renderer, scene, ortho_cam, persp_cam, inst,
        *doc.getElementById("btn-cam"),
        *doc.getElementById("meta"),
        true, 0, 0.785398
    });

    const char* ids[5] = { "btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            if (i == 0) st->use_ortho = !st->use_ortho;
            else if (i >= 1 && i <= 3) st->layout_mode = i - 1;
            else st->cam_yaw += M_PI / 4.0;
            render_city(st);
        });
    }
    render_city(st);
}
