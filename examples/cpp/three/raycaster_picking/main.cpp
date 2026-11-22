#include <cmath>
#include <cstdio>
#include <memory>
#include <string>
#include <vector>

#include "dom/dom.hpp"
#include "three/three.hpp"

using namespace three;

static const char* NAMES[9] = {
    "Box", "Cone", "Cylinder", "Sphere", "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron"
};

struct AppState {
    WebGLRenderer renderer;
    dom::HTMLCanvasElement canvas;
    Scene<> scene;
    PerspectiveCamera camera;
    std::vector<MeshStandardMaterial> mats;
    ts::Array<Object3D<>> meshes;
    Box3 box;
    Box3Helper helper;
    Raycaster raycaster;
    dom::HTMLElement meta;
    double yaw = 0.45;
    int selected = -1;
    std::string hit_info = "none";
};

static void update_camera(const std::shared_ptr<AppState>& st) {
    double r = 8.4;
    st->camera.position().set(r * std::sin(st->yaw), 5.1, r * std::cos(st->yaw));
    st->camera.lookAt(Vector3::create(0.0, 0.0, 0.0));
    st->camera.updateMatrixWorld(true);
    st->scene.updateMatrixWorld(true);
}

static void render_scene(const std::shared_ptr<AppState>& st) {
    for (int i = 0; i < 9; i++) {
        st->mats[i].color().setHex(i == st->selected ? 0x111111 : 0xd4d4d4);
    }
    if (st->selected >= 0 && st->selected < 9) {
        st->box.setFromObject(st->meshes[st->selected], true);
        st->helper.visible(true);
    } else {
        st->helper.visible(false);
    }
    st->renderer.render(st->scene, st->camera);
    st->meta.textContent("picked: " + st->hit_info + " \xC2\xB7 9 specimens \xC2\xB7 click canvas or buttons");
}

static void cast_at_ndc(const std::shared_ptr<AppState>& st, double nx, double ny) {
    st->raycaster.setFromCamera(Vector2::create(nx, ny), st->camera);
    auto hits = st->raycaster.intersectObjects<Object3D<>>(st->meshes, false);
    if (hits.size() > 0) {
        Intersection<Object3D<>> hit = hits[0];
        double hid = hit.object().id();
        int found = -1;
        for (int i = 0; i < 9; i++) {
            if (st->meshes[i].id() == hid) { found = i; break; }
        }
        st->selected = found;
        if (found >= 0) {
            st->box.setFromObject(st->meshes[found], true);
            Vector3 sz = st->box.getSize(Vector3::create());
            Vector3 pt = hit.point();
            char buf[160];
            std::snprintf(buf, sizeof buf, "%s at d=%.2f (%.2f, %.2f, %.2f) box=%.2fx%.2fx%.2f",
                          NAMES[found], hit.distance(), pt.x(), pt.y(), pt.z(), sz.x(), sz.y(), sz.z());
            st->hit_info = buf;
        } else {
            st->hit_info = "unknown";
        }
    } else {
        st->selected = -1;
        char buf[80];
        std::snprintf(buf, sizeof buf, "miss at ndc (%.2f, %.2f)", nx, ny);
        st->hit_info = buf;
    }
    render_scene(st);
}

static void pick_specimen(const std::shared_ptr<AppState>& st, int idx) {
    update_camera(st);
    Vector3 wp = st->meshes[idx].getWorldPosition(Vector3::create());
    Vector3 proj = wp.project(st->camera);
    cast_at_ndc(st, proj.x(), proj.y());
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    double width = 680.0, height = 400.0;

    WebGLRenderer renderer = WebGLRenderer::create({{.antialias = true, .preserveDrawingBuffer = true}});
    renderer.setPixelRatio(1.0);
    renderer.setSize(width, height);
    dom::HTMLCanvasElement canvas = renderer.domElement();
    doc.getElementById("stage")->appendChild(canvas);

    Scene scene = Scene<>::create();
    scene.background(Color::create(0xf7f7f7));
    PerspectiveCamera camera = PerspectiveCamera::create(42.0, width / height, 0.1, 100.0);

    DirectionalLight key = DirectionalLight::create(0xffffff, 2.2);
    key.position().set(6.0, 10.0, 8.0);
    DirectionalLight rim = DirectionalLight::create(0xffffff, 0.9);
    rim.position().set(-7.0, 5.0, -6.0);

    GridHelper grid = GridHelper::create(9.0, 9.0, 0x999999, 0xd4d4d4);
    grid.position().y(-0.85);
    scene.add(AmbientLight::create(0xffffff, 1.2), key, rim, grid);

    std::vector<BufferGeometry<>> geos{
        BoxGeometry::create(1.05, 1.05, 1.05),
        ConeGeometry::create(0.62, 1.25, 20.0),
        CylinderGeometry::create(0.48, 0.48, 1.2, 24.0),
        SphereGeometry::create(0.62, 24.0, 18.0),
        OctahedronGeometry::create(0.68, 0.0),
        DodecahedronGeometry::create(0.62, 0.0),
        TorusGeometry::create(0.5, 0.18, 14.0, 36.0),
        IcosahedronGeometry::create(0.64, 0.0),
        TetrahedronGeometry::create(0.72, 0.0),
    };

    std::vector<MeshStandardMaterial> mats;
    ts::Array<Object3D<>> meshes = ts::Array<Object3D<>>::create();
    for (int i = 0; i < 9; i++) {
        int gx = (i % 3) - 1;
        int gz = (i / 3) - 1;
        MeshStandardMaterial mat = MeshStandardMaterial::create({{.color = 0xd4d4d4, .roughness = 0.55, .metalness = 0.1, .flatShading = i >= 4}});
        Mesh m = Mesh<>::create(geos[i], mat);
        m.name(NAMES[i]);
        m.position().set(double(gx) * 2.1, 0.0, double(gz) * 2.1);
        m.rotation().set(0.2, 0.35 * double(i), 0.0);
        scene.add(m);
        mats.push_back(mat);
        meshes.push(m);
    }

    Box3 box = Box3::create(Vector3::create(-0.5, -0.5, -0.5), Vector3::create(0.5, 0.5, 0.5));
    Box3Helper helper = Box3Helper::create(box, 0x111111);
    helper.visible(false);
    scene.add(helper);

    auto st = std::make_shared<AppState>(AppState{
        renderer, canvas, scene, camera, std::move(mats), meshes, box, helper,
        Raycaster::create(),
        *doc.getElementById("meta"),
        0.45, -1, "none"
    });

    canvas.addEventListener(dom::HTMLElementEventMapKeys::click, [st](dom::PointerEvent ev) {
        dom::DOMRect rect = st->canvas.getBoundingClientRect();
        double nx = ((ev.clientX() - rect.left()) / rect.width()) * 2.0 - 1.0;
        double ny = -((ev.clientY() - rect.top()) / rect.height()) * 2.0 + 1.0;
        update_camera(st);
        cast_at_ndc(st, nx, ny);
    });

    const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            if (i == 0) pick_specimen(st, 4);
            else if (i == 1) pick_specimen(st, 0);
            else if (i == 2) pick_specimen(st, 8);
            else if (i == 3) { st->yaw += M_PI / 4.0; update_camera(st); render_scene(st); }
            else { st->selected = -1; st->hit_info = "none"; update_camera(st); render_scene(st); }
        });
    }
    pick_specimen(st, 4);
}
