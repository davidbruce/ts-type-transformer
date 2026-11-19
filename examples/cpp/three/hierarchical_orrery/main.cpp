#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "three/three.hpp"

using namespace three;

static MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters::create();
    mp.color(ColorRepresentation(double(hex)));
    mp.roughness(rough);
    mp.metalness(metal);
    mp.flatShading(flat);
    return MeshStandardMaterial::create(mp);
}

struct AppState {
    WebGLRenderer renderer;
    Scene<> scene;
    PerspectiveCamera camera;
    Group<> inner_pivot;
    Group<> moon_pivot;
    Group<> outer_pivot;
    Group<> outer_station;
    Mesh<> moon;
    Mesh<> station_body;
    dom::HTMLElement tilt_btn;
    dom::HTMLElement meta;
    double epoch = 0.6;
    bool tilted = false;
};

static void update_orrery(const std::shared_ptr<AppState>& st) {
    st->inner_pivot.rotation().y(st->epoch);
    st->moon_pivot.rotation().y(st->epoch * 2.5);
    st->outer_pivot.rotation().y(-st->epoch * 0.55);
    st->outer_pivot.rotation().z(st->tilted ? 0.38 : 0.0);
    st->station_body.rotation().y(st->epoch * 1.4);

    st->scene.updateMatrixWorld(true);
    Vector3 mpos = st->moon.getWorldPosition(Vector3::create());
    Vector3 spos = st->outer_station.getWorldPosition(Vector3::create());

    st->tilt_btn.textContent(st->tilted ? "Outer Tilt: On" : "Outer Tilt: Off");
    st->renderer.render(st->scene, st->camera);

    long deg = std::lround(st->epoch * 180.0 / M_PI);
    char buf[256];
    std::snprintf(buf, sizeof buf,
                  "epoch: %ld deg \xC2\xB7 moon world: (%.2f, %.2f, %.2f) \xC2\xB7 station world: (%.2f, %.2f, %.2f)",
                  deg, mpos.x(), mpos.y(), mpos.z(), spos.x(), spos.y(), spos.z());
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
    camera.position().set(5.6, 5.2, 7.6);
    camera.lookAt(Vector3::create(0.0, -0.1, 0.0));

    scene.add({AmbientLight::create(ColorRepresentation(double(0xffffff)), 1.2)});
    DirectionalLight key = DirectionalLight::create(ColorRepresentation(double(0xffffff)), 2.2);
    key.position().set(6.0, 10.0, 8.0);
    scene.add({key});
    DirectionalLight rim = DirectionalLight::create(ColorRepresentation(double(0xffffff)), 0.9);
    rim.position().set(-7.0, 5.0, -6.0);
    scene.add({rim});

    GridHelper grid = GridHelper::create(12.0, 12.0, ColorRepresentation(double(0x999999)), ColorRepresentation(double(0xe5e5e5)));
    grid.position().y(-1.5);
    scene.add({grid});

    Mesh<> pedestal = Mesh<>::create(CylinderGeometry::create(0.55, 0.75, 0.35, 24.0), Material_or_Material_array(std_mat(0x262626, 0.7, 0.05, false)));
    pedestal.position().y(-1.3);
    scene.add({pedestal});

    Mesh<> core = Mesh<>::create(SphereGeometry::create(0.65, 24.0, 18.0), Material_or_Material_array(std_mat(0x111111, 0.45, 0.2, false)));
    scene.add({core});

    Group<> inner_pivot = Group<>::create();
    scene.add({inner_pivot});
    MeshBasicMaterialParameters tp1 = MeshBasicMaterialParameters::create();
    tp1.color(ColorRepresentation(double(0x999999)));
    Mesh<> inner_track = Mesh<>::create(TorusGeometry::create(2.1, 0.025, 8.0, 64.0), Material_or_Material_array(MeshBasicMaterial::create(tp1)));
    inner_track.rotation().x(M_PI / 2.0);
    inner_pivot.add({inner_track});

    Group<> inner_planet = Group<>::create();
    inner_planet.position().set(2.1, 0.0, 0.0);
    inner_pivot.add({inner_planet});
    inner_planet.add({Mesh<>::create(SphereGeometry::create(0.36, 20.0, 16.0), Material_or_Material_array(std_mat(0x555555, 0.55, 0.1, false)))});

    Group<> moon_pivot = Group<>::create();
    inner_planet.add({moon_pivot});
    Mesh<> moon = Mesh<>::create(OctahedronGeometry::create(0.18, 0.0), Material_or_Material_array(std_mat(0xd4d4d4, 0.4, 0.1, true)));
    moon.position().set(0.75, 0.0, 0.0);
    moon_pivot.add({moon});

    Group<> outer_pivot = Group<>::create();
    scene.add({outer_pivot});
    MeshBasicMaterialParameters tp2 = MeshBasicMaterialParameters::create();
    tp2.color(ColorRepresentation(double(0xb5b5b5)));
    Mesh<> outer_track = Mesh<>::create(TorusGeometry::create(3.8, 0.025, 8.0, 80.0), Material_or_Material_array(MeshBasicMaterial::create(tp2)));
    outer_track.rotation().x(M_PI / 2.0);
    outer_pivot.add({outer_track});

    Group<> outer_station = Group<>::create();
    outer_station.position().set(3.8, 0.0, 0.0);
    outer_pivot.add({outer_station});
    Mesh<> station_body = Mesh<>::create(IcosahedronGeometry::create(0.42, 0.0), Material_or_Material_array(std_mat(0x404040, 0.5, 0.1, true)));
    outer_station.add({station_body});

    Mesh<> halo = Mesh<>::create(TorusGeometry::create(0.68, 0.06, 10.0, 32.0), Material_or_Material_array(std_mat(0x111111, 0.4, 0.1, false)));
    halo.rotation().x(0.8);
    outer_station.add({halo});

    auto st = std::make_shared<AppState>(AppState{
        renderer, scene, camera, inner_pivot, moon_pivot, outer_pivot, outer_station, moon, station_body,
        *doc.getElementById("btn-tilt"),
        *doc.getElementById("meta"),
        0.6, false
    });

    const char* ids[3] = { "btn-step", "btn-tilt", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        dom::HTMLElement btn = *doc.getElementById(ids[i]);
        ts::Listener<dom::PointerEvent> cb = [st, i](dom::PointerEvent) {
            if (i == 0) st->epoch += M_PI / 6.0;
            else if (i == 1) st->tilted = !st->tilted;
            else { st->epoch = 0.6; st->tilted = false; }
            update_orrery(st);
        };
        btn.addEventListener(dom::HTMLElementEventMapKeys::click, cb);
    }
    update_orrery(st);
}
