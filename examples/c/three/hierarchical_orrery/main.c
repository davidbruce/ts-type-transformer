#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Three_Object3D_add(AS_Three_Object3D(parent), JsArray_Three_Object3D_from((Three_Object3D[]){ AS_Three_Object3D(child) }, 1))

static Three_MeshStandardMaterial std_mat(uint32_t hex, double rough, double metal, bool flat) {
    return Three_MeshStandardMaterial_new(Three_MeshStandardMaterialParameters_create(
        .color_number = hex, .roughness = rough, .metalness = metal, .flatShading = flat));
}

static Three_WebGLRenderer g_renderer;
static Three_Scene g_scene;
static Three_PerspectiveCamera g_camera;
static Three_Group g_inner_pivot;
static Three_Group g_moon_pivot;
static Three_Group g_outer_pivot;
static Three_Group g_outer_station;
static Three_Mesh g_moon;
static Three_Mesh g_station_body;
static HTMLElement g_tilt_btn;
static HTMLElement g_meta;
static double g_epoch = 0.6;
static bool g_tilted = false;

static void update_orrery(void) {
    js_scope();
    Three_Euler_set_y(Three_Group_get_rotation(g_inner_pivot), g_epoch);
    Three_Euler_set_y(Three_Group_get_rotation(g_moon_pivot), g_epoch * 2.5);
    Three_Euler_set_y(Three_Group_get_rotation(g_outer_pivot), -g_epoch * 0.55);
    Three_Euler_set_z(Three_Group_get_rotation(g_outer_pivot), g_tilted ? 0.38 : 0.0);
    Three_Euler_set_y(Three_Mesh_get_rotation(g_station_body), g_epoch * 1.4);

    Three_Scene_updateMatrixWorld(g_scene, true);
    Three_Vector3 mpos = Three_Mesh_getWorldPosition(g_moon, Three_Vector3_new());
    Three_Vector3 spos = Three_Group_getWorldPosition(g_outer_station, Three_Vector3_new());

    HTMLElement_set_textContent(g_tilt_btn, g_tilted ? "Outer Tilt: On" : "Outer Tilt: Off");
    Three_WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_epoch * 180.0 / PI + 0.5);
    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "epoch: %d deg \xC2\xB7 moon world: (%.2f, %.2f, %.2f) \xC2\xB7 station world: (%.2f, %.2f, %.2f)",
        deg,
        Three_Vector3_get_x(mpos), Three_Vector3_get_y(mpos), Three_Vector3_get_z(mpos),
        Three_Vector3_get_x(spos), Three_Vector3_get_y(spos), Three_Vector3_get_z(spos)
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
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
    js_scope();
    Document doc = get_document();
    double width = 680.0, height = 400.0;

    g_renderer = Three_WebGLRenderer_new(Three_WebGLRendererParameters_create(.antialias = true, .preserveDrawingBuffer = true));
    js_retain(g_renderer);
    Three_WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    Three_WebGLRenderer_setSize(g_renderer, width, height);
    HTMLElement_appendChild(Document_getElementById(doc, "stage"), Three_WebGLRenderer_get_domElement(g_renderer));

    g_scene = Three_Scene_new();
    js_retain(g_scene);
    Three_Scene_set_background(g_scene, Three_Color_new(0xf7f7f7));
    g_camera = Three_PerspectiveCamera_new(42.0, width / height, 0.1, 100.0);
    js_retain(g_camera);
    Three_Vector3_set(Three_PerspectiveCamera_get_position(g_camera), 5.6, 5.2, 7.6);
    Three_PerspectiveCamera_lookAt(g_camera, Three_Vector3_new(0.0, -0.1, 0.0));

    add_obj(g_scene, Three_AmbientLight_new(0xffffff, 1.2));
    Three_DirectionalLight key = Three_DirectionalLight_new(0xffffff, 2.2);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    add_obj(g_scene, key);
    Three_DirectionalLight rim = Three_DirectionalLight_new(0xffffff, 0.9);
    Three_Vector3_set(Three_DirectionalLight_get_position(rim), -7.0, 5.0, -6.0);
    add_obj(g_scene, rim);

    Three_GridHelper grid = Three_GridHelper_new(12.0, 12.0, 0x999999, 0xe5e5e5);
    Three_Vector3_set_y(Three_GridHelper_get_position(grid), -1.5);
    add_obj(g_scene, grid);

    Three_Mesh pedestal = Three_Mesh_new(Three_CylinderGeometry_new(0.55, 0.75, 0.35, 24.0), std_mat(0x262626, 0.7, 0.05, false));
    Three_Vector3_set_y(Three_Mesh_get_position(pedestal), -1.3);
    add_obj(g_scene, pedestal);

    Three_Mesh core = Three_Mesh_new(Three_SphereGeometry_new(0.65, 24.0, 18.0), std_mat(0x111111, 0.45, 0.2, false));
    add_obj(g_scene, core);

    g_inner_pivot = Three_Group_new();
    js_retain(g_inner_pivot);
    add_obj(g_scene, g_inner_pivot);

    Three_Mesh inner_track = Three_Mesh_new(Three_TorusGeometry_new(2.1, 0.025, 8.0, 64.0), Three_MeshBasicMaterial_new(Three_MeshBasicMaterialParameters_create(
        .color_number = 0x999999)));
    Three_Euler_set_x(Three_Mesh_get_rotation(inner_track), PI / 2.0);
    add_obj(g_inner_pivot, inner_track);

    Three_Group inner_planet = Three_Group_new();
    Three_Vector3_set(Three_Group_get_position(inner_planet), 2.1, 0.0, 0.0);
    add_obj(g_inner_pivot, inner_planet);

    Three_Mesh planet_mesh = Three_Mesh_new(Three_SphereGeometry_new(0.36, 20.0, 16.0), std_mat(0x555555, 0.55, 0.1, false));
    add_obj(inner_planet, planet_mesh);

    g_moon_pivot = Three_Group_new();
    js_retain(g_moon_pivot);
    add_obj(inner_planet, g_moon_pivot);

    g_moon = Three_Mesh_new(Three_OctahedronGeometry_new(0.18, 0.0), std_mat(0xd4d4d4, 0.4, 0.1, true));
    js_retain(g_moon);
    Three_Vector3_set(Three_Mesh_get_position(g_moon), 0.75, 0.0, 0.0);
    add_obj(g_moon_pivot, g_moon);

    g_outer_pivot = Three_Group_new();
    js_retain(g_outer_pivot);
    add_obj(g_scene, g_outer_pivot);

    Three_Mesh outer_track = Three_Mesh_new(Three_TorusGeometry_new(3.8, 0.025, 8.0, 80.0), Three_MeshBasicMaterial_new(Three_MeshBasicMaterialParameters_create(
        .color_number = 0xb5b5b5)));
    Three_Euler_set_x(Three_Mesh_get_rotation(outer_track), PI / 2.0);
    add_obj(g_outer_pivot, outer_track);

    g_outer_station = Three_Group_new();
    js_retain(g_outer_station);
    Three_Vector3_set(Three_Group_get_position(g_outer_station), 3.8, 0.0, 0.0);
    add_obj(g_outer_pivot, g_outer_station);

    g_station_body = Three_Mesh_new(Three_IcosahedronGeometry_new(0.42, 0.0), std_mat(0x404040, 0.5, 0.1, true));
    js_retain(g_station_body);
    add_obj(g_outer_station, g_station_body);

    Three_Mesh halo = Three_Mesh_new(Three_TorusGeometry_new(0.68, 0.06, 10.0, 32.0), std_mat(0x111111, 0.4, 0.1, false));
    Three_Euler_set_x(Three_Mesh_get_rotation(halo), 0.8);
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
}
