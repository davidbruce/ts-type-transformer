#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Object3D_add(AS_Object3D(parent), JsArray_Object3D_from((Object3D[]){ AS_Object3D(child) }, 1))

static double wave_height(int mode, double x, double z) {
    if (mode == 1) {
        double r = sqrt(x * x + z * z);
        return 0.78 * cos(r * 2.2) / (1.0 + 0.18 * r * r);
    } else if (mode == 2) {
        return 0.065 * (x * x - z * z) + 0.18 * sin(x * 1.6);
    }
    return 0.55 * cos(x * 1.4) * sin(z * 1.4) + 0.25 * cos((x + z) * 1.1);
}

static const char* wave_name(int mode) {
    if (mode == 1) return "Radial Ripple";
    if (mode == 2) return "Saddle Ridge";
    return "Interference";
}

#define GRID_N 28

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static BufferGeometry g_geo;
static Float32BufferAttribute g_pos_attr;
static Float32BufferAttribute g_pts_attr;
static Mesh g_wire_mesh;
static HTMLElement g_wire_btn;
static HTMLElement g_meta;
static int g_tri_count = 0;
static int g_wave_mode = 0;
static bool g_overlay_on = true;
static double g_azimuth = 0.62;

static void rebuild_field(void) {
    js_scope_open();
    double span = 7.6, half = span / 2.0;
    for (int iz = 0; iz < GRID_N; iz++) {
        for (int ix = 0; ix < GRID_N; ix++) {
            int idx = iz * GRID_N + ix;
            double x = -half + span * (double)ix / (double)(GRID_N - 1);
            double z = -half + span * (double)iz / (double)(GRID_N - 1);
            double y = wave_height(g_wave_mode, x, z);
            Float32BufferAttribute_setXYZ(g_pos_attr, (double)idx, x, y, z);
        }
    }
    int pi = 0;
    for (int iz = 0; iz < GRID_N; iz += 2) {
        for (int ix = 0; ix < GRID_N; ix += 2) {
            double x = -half + span * (double)ix / (double)(GRID_N - 1);
            double z = -half + span * (double)iz / (double)(GRID_N - 1);
            double y = wave_height(g_wave_mode, x, z) + 0.06;
            Float32BufferAttribute_setXYZ(g_pts_attr, (double)pi, x, y, z);
            pi++;
        }
    }
    Float32BufferAttribute_set_needsUpdate(g_pos_attr, true);
    Float32BufferAttribute_set_needsUpdate(g_pts_attr, true);
    BufferGeometry_computeVertexNormals(g_geo);
    BufferGeometry_computeBoundingBox(g_geo);

    double r = 9.2;
    Vector3_set(PerspectiveCamera_get_position(g_camera), r * sin(g_azimuth), 5.4, r * cos(g_azimuth));
    PerspectiveCamera_lookAt(g_camera, Vector3_new(0.0, 0.0, 0.0));

    Mesh_set_visible(g_wire_mesh, g_overlay_on);
    HTMLElement_set_textContent(g_wire_btn, g_overlay_on ? "Wireframe: On" : "Wireframe: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    Box3 bb = BufferGeometry_get_boundingBox(g_geo);
    double min_y = Vector3_get_y(Box3_get_min(bb));
    double max_y = Vector3_get_y(Box3_get_max(bb));

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "profile: %s \xC2\xB7 %d vertices \xC2\xB7 %d triangles \xC2\xB7 y=[%.2f, %.2f] \xC2\xB7 wireframe: %s",
        wave_name(g_wave_mode), GRID_N * GRID_N, g_tri_count, min_y, max_y, g_overlay_on ? "on" : "off"
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id < 3) g_wave_mode = id;
    else if (id == 3) g_overlay_on = !g_overlay_on;
    else g_azimuth += PI / 4.0;
    rebuild_field();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    double width = 680.0, height = 400.0;

    WebGLRendererParameters rp = WebGLRendererParameters_create();
    WebGLRendererParameters_set_antialias(rp, true);
    WebGLRendererParameters_set_preserveDrawingBuffer(rp, true);
    g_renderer = WebGLRenderer_new(rp);
    js_retain(g_renderer);
    WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    WebGLRenderer_setSize(g_renderer, width, height);
    HTMLElement_appendChild(Document_getElementById(doc, "stage"), WebGLRenderer_get_domElement(g_renderer));

    g_scene = Scene_new();
    js_retain(g_scene);
    Scene_set_background(g_scene, Color_or_Texture_of(Color_new(ColorRepresentation_of(0xf7f7f7))));
    g_camera = PerspectiveCamera_new(42.0, width / height, 0.1, 100.0);
    js_retain(g_camera);

    add_obj(g_scene, AmbientLight_new(ColorRepresentation_of(0xffffff), 1.2));
    DirectionalLight key = DirectionalLight_new(ColorRepresentation_of(0xffffff), 2.2);
    Vector3_set(DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    add_obj(g_scene, key);
    DirectionalLight rim = DirectionalLight_new(ColorRepresentation_of(0xffffff), 0.9);
    Vector3_set(DirectionalLight_get_position(rim), -7.0, 5.0, -6.0);
    add_obj(g_scene, rim);

    GridHelper grid = GridHelper_new(9.0, 12.0, ColorRepresentation_of(0x999999), ColorRepresentation_of(0xe5e5e5));
    Vector3_set_y(GridHelper_get_position(grid), -1.1);
    add_obj(g_scene, grid);

    g_geo = BufferGeometry_new();
    js_retain(g_geo);
    g_pos_attr = Float32BufferAttribute_new_with_number((double)(GRID_N * GRID_N * 3), 3.0);
    js_retain(g_pos_attr);
    JsArray_double indices = JsArray_double_new();
    for (int iz = 0; iz < GRID_N - 1; iz++) {
        for (int ix = 0; ix < GRID_N - 1; ix++) {
            double a = (double)(iz * GRID_N + ix);
            double b = a + 1.0;
            double c = a + (double)GRID_N;
            double d = c + 1.0;
            JsArray_double_push(indices, a);
            JsArray_double_push(indices, c);
            JsArray_double_push(indices, b);
            JsArray_double_push(indices, b);
            JsArray_double_push(indices, c);
            JsArray_double_push(indices, d);
        }
    }
    g_tri_count = (GRID_N - 1) * (GRID_N - 1) * 2;
    BufferGeometry_setAttribute(g_geo, "position", g_pos_attr);
    BufferGeometry_setIndex_with_opt_number_array(g_geo, indices);

    MeshStandardMaterialParameters smp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(smp, ColorRepresentation_of(0xb5b5b5));
    MeshStandardMaterialParameters_set_roughness(smp, 0.72);
    MeshStandardMaterialParameters_set_metalness(smp, 0.05);
    MeshStandardMaterialParameters_set_flatShading(smp, true);
    Mesh surface_mesh = Mesh_new(g_geo, MeshStandardMaterial_new(smp));
    add_obj(g_scene, surface_mesh);

    MeshBasicMaterialParameters wmp = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(wmp, ColorRepresentation_of(0x262626));
    MeshBasicMaterialParameters_set_wireframe(wmp, true);
    g_wire_mesh = Mesh_new(g_geo, MeshBasicMaterial_new(wmp));
    js_retain(g_wire_mesh);
    Vector3_set_y(Mesh_get_position(g_wire_mesh), 0.015);
    add_obj(g_scene, g_wire_mesh);

    int pt_count = (GRID_N / 2) * (GRID_N / 2);
    BufferGeometry pts_geo = BufferGeometry_new();
    g_pts_attr = Float32BufferAttribute_new_with_number((double)(pt_count * 3), 3.0);
    js_retain(g_pts_attr);
    BufferGeometry_setAttribute(pts_geo, "position", g_pts_attr);
    PointsMaterialParameters pmp = PointsMaterialParameters_create();
    PointsMaterialParameters_set_color(pmp, ColorRepresentation_of(0x111111));
    PointsMaterialParameters_set_size(pmp, 0.14);
    add_obj(g_scene, Points_new(pts_geo, PointsMaterial_new(pmp)));

    g_wire_btn = Document_getElementById(doc, "btn-wire");
    js_retain(g_wire_btn);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);

    static const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-wire", "btn-orbit" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    rebuild_field();
    js_scope_close();
}
