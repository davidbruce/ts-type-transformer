#include <cmath>
#include <cstdio>
#include <memory>
#include <string>

#include "dom/dom.hpp"
#include "three/three.hpp"

using namespace three;

static constexpr int GRID_N = 28;

static double wave_height(int mode, double x, double z) {
    if (mode == 1) {
        double r = std::sqrt(x * x + z * z);
        return 0.78 * std::cos(r * 2.2) / (1.0 + 0.18 * r * r);
    } else if (mode == 2) {
        return 0.065 * (x * x - z * z) + 0.18 * std::sin(x * 1.6);
    }
    return 0.55 * std::cos(x * 1.4) * std::sin(z * 1.4) + 0.25 * std::cos((x + z) * 1.1);
}

static const char* wave_name(int mode) {
    if (mode == 1) return "Radial Ripple";
    if (mode == 2) return "Saddle Ridge";
    return "Interference";
}

struct AppState {
    WebGLRenderer renderer;
    Scene<> scene;
    PerspectiveCamera camera;
    BufferGeometry<> geo;
    Float32BufferAttribute pos_attr;
    Float32BufferAttribute pts_attr;
    Mesh<> wire_mesh;
    dom::HTMLElement wire_btn;
    dom::HTMLElement meta;
    int tri_count = 0;
    int wave_mode = 0;
    bool overlay_on = true;
    double azimuth = 0.62;
};

static void rebuild_field(const std::shared_ptr<AppState>& st) {
    double span = 7.6, half = span / 2.0;
    for (int iz = 0; iz < GRID_N; iz++) {
        for (int ix = 0; ix < GRID_N; ix++) {
            int idx = iz * GRID_N + ix;
            double x = -half + span * double(ix) / double(GRID_N - 1);
            double z = -half + span * double(iz) / double(GRID_N - 1);
            double y = wave_height(st->wave_mode, x, z);
            st->pos_attr.setXYZ(double(idx), x, y, z);
        }
    }
    int pi = 0;
    for (int iz = 0; iz < GRID_N; iz += 2) {
        for (int ix = 0; ix < GRID_N; ix += 2) {
            double x = -half + span * double(ix) / double(GRID_N - 1);
            double z = -half + span * double(iz) / double(GRID_N - 1);
            double y = wave_height(st->wave_mode, x, z) + 0.06;
            st->pts_attr.setXYZ(double(pi++), x, y, z);
        }
    }
    st->pos_attr.needsUpdate(true);
    st->pts_attr.needsUpdate(true);
    st->geo.computeVertexNormals();
    st->geo.computeBoundingBox();

    double r = 9.2;
    st->camera.position().set(r * std::sin(st->azimuth), 5.4, r * std::cos(st->azimuth));
    st->camera.lookAt(Vector3(0.0, 0.0, 0.0));
    st->wire_mesh.visible(st->overlay_on);
    st->wire_btn.textContent(st->overlay_on ? "Wireframe: On" : "Wireframe: Off");
    st->renderer.render(st->scene, st->camera);

    Box3 bb = *st->geo.boundingBox();
    char buf[256];
    std::snprintf(buf, sizeof buf,
                  "profile: %s \xC2\xB7 %d vertices \xC2\xB7 %d triangles \xC2\xB7 y=[%.2f, %.2f] \xC2\xB7 wireframe: %s",
                  wave_name(st->wave_mode), GRID_N * GRID_N, st->tri_count,
                  bb.min().y(), bb.max().y(), st->overlay_on ? "on" : "off");
    st->meta.textContent(buf);
}

TS_EXPORT(start) void start() {
    dom::Document doc = dom::get_document();
    double width = 680.0, height = 400.0;

    WebGLRenderer renderer = WebGLRenderer::create({{.antialias = true, .preserveDrawingBuffer = true}});
    renderer.setPixelRatio(1.0);
    renderer.setSize(width, height);
    doc.getElementById("stage")->appendChild(renderer.domElement());

    Scene scene = Scene<>::create();
    scene.background(Color(0xf7f7f7));
    PerspectiveCamera camera = PerspectiveCamera(42.0, width / height, 0.1, 100.0);

    DirectionalLight key = DirectionalLight(0xffffff, 2.2);
    key.position().set(6.0, 10.0, 8.0);
    DirectionalLight rim = DirectionalLight(0xffffff, 0.9);
    rim.position().set(-7.0, 5.0, -6.0);

    GridHelper grid = GridHelper(9.0, 12.0, 0x999999, 0xe5e5e5);
    grid.position().y(-1.1);

    BufferGeometry geo = BufferGeometry<>::create();
    Float32BufferAttribute pos_attr = Float32BufferAttribute(GRID_N * GRID_N * 3, 3.0);
    ts::Array<double> indices = ts::Array<double>::create();
    for (int iz = 0; iz < GRID_N - 1; iz++) {
        for (int ix = 0; ix < GRID_N - 1; ix++) {
            double a = double(iz * GRID_N + ix);
            double b = a + 1.0;
            double c = a + double(GRID_N);
            double d = c + 1.0;
            indices.push(a); indices.push(c); indices.push(b);
            indices.push(b); indices.push(c); indices.push(d);
        }
    }
    int tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    geo.setAttribute("position", pos_attr);
    geo.setIndex(indices);

    Mesh surface = Mesh(
        geo,
        MeshStandardMaterial::create(
            {{.color = 0xb5b5b5, .roughness = 0.72, .metalness = 0.05, .flatShading = true}}
        )
    );

    Mesh wire_mesh = Mesh(geo, MeshBasicMaterial::create({{.color = 0x262626, .wireframe = true}}));
    wire_mesh.position().y(0.015);

    int pt_count = (GRID_N / 2) * (GRID_N / 2);
    using PointsGeo = BufferGeometry<NormalOrGLBufferAttributes, BufferGeometryEventMap>;
    PointsGeo pts_geo = PointsGeo::create();
    Float32BufferAttribute pts_attr = Float32BufferAttribute(pt_count * 3, 3.0);
    pts_geo.setAttribute("position", pts_attr);

    scene.add(
        AmbientLight(0xffffff, 1.2), key, rim, grid,
        surface, wire_mesh, three::Points<PointsGeo>(pts_geo, PointsMaterial::create({{.color = 0x111111, .size = 0.14}}))
    );

    auto st = std::make_shared<AppState>(AppState{
        renderer, scene, camera, geo, pos_attr, pts_attr, wire_mesh,
        *doc.getElementById("btn-wire"),
        *doc.getElementById("meta"),
        tri_count, 0, true, 0.62
    });

    const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit" };
    for (int i = 0; i < 5; i++) {
        doc.getElementById(ids[i])->addEventListener(dom::HTMLElementEventMapKeys::click, [st, i](dom::PointerEvent) {
            if (i < 3) st->wave_mode = i;
            else if (i == 3) st->overlay_on = !st->overlay_on;
            else st->azimuth += M_PI / 4.0;
            rebuild_field(st);
        });
    }
    rebuild_field(st);
}
