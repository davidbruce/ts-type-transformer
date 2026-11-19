#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "three/three.hpp"

using namespace three;

struct AppState {
    WebGLRenderer renderer;
    Scene<> scene;
    PerspectiveCamera camera;
    Group<> wire_group;
    dom::HTMLElement wire_btn;
    dom::HTMLElement meta;
    bool wire_visible = true;
    double azimuth = 0.35;
};

static void add_solid(const std::shared_ptr<AppState>& st,
                      const BufferGeometry<>& geo,
                      uint32_t hex, double rough, double metal, bool flat,
                      double x, double y, double z, double thresh,
                      const LineBasicMaterial& line_mat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters::create();
    mp.color(ColorRepresentation(double(hex)));
    mp.roughness(rough);
    mp.metalness(metal);
    mp.flatShading(flat);
    MeshStandardMaterial mat = MeshStandardMaterial::create(mp);

    Mesh<> mesh = Mesh<>::create(geo, Material_or_Material_array(mat));
    mesh.position().set(x, y, z);
    mesh.rotation().set(0.25, 0.45, 0.0);
    st->scene.add({mesh});

    EdgesGeometry<> egeo = EdgesGeometry<>::create(std::optional<BufferGeometry<>>(geo), thresh);
    LineSegments<> segs = LineSegments<>::create(egeo, Material_or_Material_array(line_mat));
    segs.position().set(x, y, z);
    segs.rotation().set(0.25, 0.45, 0.0);
    st->wire_group.add({segs});
}

static void update_view(const std::shared_ptr<AppState>& st) {
    double r = 8.6;
    st->camera.position().set(r * std::sin(st->azimuth), 3.6, r * std::cos(st->azimuth));
    st->camera.lookAt(Vector3::create(0.0, 0.0, 0.0));
    st->wire_group.visible(st->wire_visible);
    st->wire_btn.textContent(st->wire_visible ? "Wireframes: On" : "Wireframes: Off");
    st->renderer.render(st->scene, st->camera);

    long deg = std::lround(st->azimuth * 180.0 / M_PI);
    char buf[256];
    std::snprintf(buf, sizeof buf,
                  "5 monochrome solids \xC2\xB7 EdgesGeometry wireframes: %s \xC2\xB7 azimuth: %ld deg \xC2\xB7 680x400 WebGL",
                  st->wire_visible ? "on" : "off", deg);
    st->meta.textContent(std::string(buf));
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    double width = 680.0, height = 400.0;

    WebGLRendererParameters rp = WebGLRendererParameters::create();
    rp.antialias(true);
    rp.preserveDrawingBuffer(true);
    WebGLRenderer renderer = WebGLRenderer::create(rp);
    renderer.setPixelRatio(1.0);
    renderer.setSize(width, height);
    doc.getElementById("stage")->appendChild(renderer.domElement());

    Scene<> scene = Scene<>::create();
    scene.background(Color_or_Texture(Color::create(ColorRepresentation(double(0xf7f7f7)))));
    PerspectiveCamera camera = PerspectiveCamera::create(42.0, width / height, 0.1, 100.0);

    scene.add({AmbientLight::create(ColorRepresentation(double(0xffffff)), 1.2)});
    DirectionalLight key = DirectionalLight::create(ColorRepresentation(double(0xffffff)), 2.2);
    key.position().set(6.0, 10.0, 8.0);
    scene.add({key});
    DirectionalLight rim = DirectionalLight::create(ColorRepresentation(double(0xffffff)), 0.9);
    rim.position().set(-7.0, 5.0, -6.0);
    scene.add({rim});

    GridHelper grid = GridHelper::create(12.0, 12.0, ColorRepresentation(double(0x737373)), ColorRepresentation(double(0xd4d4d4)));
    grid.position().y(-1.2);
    scene.add({grid});

    Group<> wire_group = Group<>::create();
    scene.add({wire_group});

    auto st = std::make_shared<AppState>(AppState{
        renderer, scene, camera, wire_group,
        *doc.getElementById("btn-wire"),
        *doc.getElementById("meta"),
        true, 0.35
    });

    LineBasicMaterialParameters lmp = LineBasicMaterialParameters::create();
    lmp.color(ColorRepresentation(double(0x111111)));
    LineBasicMaterial line_mat = LineBasicMaterial::create(lmp);

    add_solid(st, BoxGeometry::create(1.45, 1.45, 1.45), 0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    add_solid(st, IcosahedronGeometry::create(0.95, 0.0), 0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    add_solid(st, TorusKnotGeometry::create(0.72, 0.24, 96.0, 16.0), 0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    add_solid(st, OctahedronGeometry::create(0.95, 0.0), 0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    add_solid(st, TorusGeometry::create(0.72, 0.23, 16.0, 48.0), 0x404040, 0.55, 0.12, false, 3.2, 0.05, -0.5, 22.0, line_mat);

    const char* ids[4] = { "btn-wire", "btn-left", "btn-right", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) st->wire_visible = !st->wire_visible;
            else if (i == 1) st->azimuth -= M_PI / 6.0;
            else if (i == 2) st->azimuth += M_PI / 6.0;
            else { st->azimuth = 0.35; st->wire_visible = true; }
            update_view(st);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
    update_view(st);
}
