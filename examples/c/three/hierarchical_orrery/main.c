#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Object3D_add(AS_Object3D(parent), JsArray_Object3D_from((Object3D[]){ AS_Object3D(child) }, 1))

static MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of(hex));
    MeshStandardMaterialParameters_set_roughness(mp, rough);
    MeshStandardMaterialParameters_set_metalness(mp, metal);
    MeshStandardMaterialParameters_set_flatShading(mp, flat);
    return MeshStandardMaterial_new(mp);
}

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static Group g_inner_pivot;
static Group g_moon_pivot;
static Group g_outer_pivot;
static Group g_outer_station;
static Mesh g_moon;
static Mesh g_station_body;
static HTMLElement g_tilt_btn;
static HTMLElement g_meta;
static double g_epoch = 0.6;
static bool g_tilted = false;

static void update_orrery(void) {
    js_scope_open();
    Euler_set_y(Group_get_rotation(g_inner_pivot), g_epoch);
    Euler_set_y(Group_get_rotation(g_moon_pivot), g_epoch * 2.5);
    Euler_set_y(Group_get_rotation(g_outer_pivot), -g_epoch * 0.55);
    Euler_set_z(Group_get_rotation(g_outer_pivot), g_tilted ? 0.38 : 0.0);
    Euler_set_y(Mesh_get_rotation(g_station_body), g_epoch * 1.4);

    Scene_updateMatrixWorld(g_scene, true);
    Vector3 mpos = Mesh_getWorldPosition(g_moon, Vector3_new());
    Vector3 spos = Group_getWorldPosition(g_outer_station, Vector3_new());

    HTMLElement_set_textContent(g_tilt_btn, g_tilted ? "Outer Tilt: On" : "Outer Tilt: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_epoch * 180.0 / PI + 0.5);
    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "epoch: %d deg \xC2\xB7 moon world: (%.2f, %.2f, %.2f) \xC2\xB7 station world: (%.2f, %.2f, %.2f)",
        deg,
        Vector3_get_x(mpos), Vector3_get_y(mpos), Vector3_get_z(mpos),
        Vector3_get_x(spos), Vector3_get_y(spos), Vector3_get_z(spos)
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
}

static const int BTN_TAGS[3] = { 0, 1, 2 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_epoch += PI / 6.0;
    else if (id == 1) g_tilted = !g_tilted;
    else { g_epoch = 0.6; g_tilted = false; }
    update_orrery();
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
    Vector3_set(PerspectiveCamera_get_position(g_camera), 5.6, 5.2, 7.6);
    PerspectiveCamera_lookAt(g_camera, Vector3_new(0.0, -0.1, 0.0));

    add_obj(g_scene, AmbientLight_new(ColorRepresentation_of(0xffffff), 1.2));
    DirectionalLight key = DirectionalLight_new(ColorRepresentation_of(0xffffff), 2.2);
    Vector3_set(DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    add_obj(g_scene, key);
    DirectionalLight rim = DirectionalLight_new(ColorRepresentation_of(0xffffff), 0.9);
    Vector3_set(DirectionalLight_get_position(rim), -7.0, 5.0, -6.0);
    add_obj(g_scene, rim);

    GridHelper grid = GridHelper_new(12.0, 12.0, ColorRepresentation_of(0x999999), ColorRepresentation_of(0xe5e5e5));
    Vector3_set_y(GridHelper_get_position(grid), -1.5);
    add_obj(g_scene, grid);

    Mesh pedestal = Mesh_new(CylinderGeometry_new(0.55, 0.75, 0.35, 24.0), std_mat(0x262626, 0.7, 0.05, false));
    Vector3_set_y(Mesh_get_position(pedestal), -1.3);
    add_obj(g_scene, pedestal);

    Mesh core = Mesh_new(SphereGeometry_new(0.65, 24.0, 18.0), std_mat(0x111111, 0.45, 0.2, false));
    add_obj(g_scene, core);

    g_inner_pivot = Group_new();
    js_retain(g_inner_pivot);
    add_obj(g_scene, g_inner_pivot);

    MeshBasicMaterialParameters tp1 = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(tp1, ColorRepresentation_of(0x999999));
    Mesh inner_track = Mesh_new(TorusGeometry_new(2.1, 0.025, 8.0, 64.0), MeshBasicMaterial_new(tp1));
    Euler_set_x(Mesh_get_rotation(inner_track), PI / 2.0);
    add_obj(g_inner_pivot, inner_track);

    Group inner_planet = Group_new();
    Vector3_set(Group_get_position(inner_planet), 2.1, 0.0, 0.0);
    add_obj(g_inner_pivot, inner_planet);

    Mesh planet_mesh = Mesh_new(SphereGeometry_new(0.36, 20.0, 16.0), std_mat(0x555555, 0.55, 0.1, false));
    add_obj(inner_planet, planet_mesh);

    g_moon_pivot = Group_new();
    js_retain(g_moon_pivot);
    add_obj(inner_planet, g_moon_pivot);

    g_moon = Mesh_new(OctahedronGeometry_new(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
    js_retain(g_moon);
    Vector3_set(Mesh_get_position(g_moon), 0.75, 0.0, 0.0);
    add_obj(g_moon_pivot, g_moon);

    g_outer_pivot = Group_new();
    js_retain(g_outer_pivot);
    add_obj(g_scene, g_outer_pivot);

    MeshBasicMaterialParameters tp2 = MeshBasicMaterialParameters_create();
    MeshBasicMaterialParameters_set_color(tp2, ColorRepresentation_of(0xb5b5b5));
    Mesh outer_track = Mesh_new(TorusGeometry_new(3.8, 0.025, 8.0, 80.0), MeshBasicMaterial_new(tp2));
    Euler_set_x(Mesh_get_rotation(outer_track), PI / 2.0);
    add_obj(g_outer_pivot, outer_track);

    g_outer_station = Group_new();
    js_retain(g_outer_station);
    Vector3_set(Group_get_position(g_outer_station), 3.8, 0.0, 0.0);
    add_obj(g_outer_pivot, g_outer_station);

    g_station_body = Mesh_new(IcosahedronGeometry_new(0.42, 0.0), std_mat(0x404040, 0.5, 0.1, true));
    js_retain(g_station_body);
    add_obj(g_outer_station, g_station_body);

    Mesh halo = Mesh_new(TorusGeometry_new(0.68, 0.06, 10.0, 32.0), std_mat(0x111111, 0.4, 0.1, false));
    Euler_set_x(Mesh_get_rotation(halo), 0.8);
    add_obj(g_outer_station, halo);

    g_tilt_btn = Document_getElementById(doc, "btn-tilt");
    js_retain(g_tilt_btn);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);

    static const char* ids[3] = { "btn-step", "btn-tilt", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    update_orrery();
    js_scope_close();
}
