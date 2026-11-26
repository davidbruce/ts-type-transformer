#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

static Three_WebGLRenderer g_renderer;
static Three_Scene g_scene;
static Three_PerspectiveCamera g_camera;
static Three_Group g_wire_group;
static HTMLElement g_wire_btn;
static HTMLElement g_meta;
static bool g_wire_visible = true;
static double g_azimuth = 0.35;

static void add_solid(Three_BufferGeometry geo, uint32_t hex, double rough, double metal, bool flat,
                      double x, double y, double z, double thresh, Three_LineBasicMaterial line_mat) {
    Three_MeshStandardMaterial mat = Three_MeshStandardMaterial_new(Three_MeshStandardMaterialParameters_create(
        .color_number = hex, .roughness = rough, .metalness = metal, .flatShading = flat));

    Three_Mesh mesh = Three_Mesh_new(geo, mat);
    Three_Vector3_set(Three_Mesh_get_position(mesh), x, y, z);
    Three_Euler_set_x(Three_Mesh_get_rotation(mesh), 0.25);
    Three_Euler_set_y(Three_Mesh_get_rotation(mesh), 0.45);
    Three_Scene_add(g_scene, mesh);

    Three_EdgesGeometry egeo = Three_EdgesGeometry_new(geo, thresh);
    Three_LineSegments segs = Three_LineSegments_new(egeo, line_mat);
    Three_Vector3_set(Three_LineSegments_get_position(segs), x, y, z);
    Three_Euler_set_x(Three_LineSegments_get_rotation(segs), 0.25);
    Three_Euler_set_y(Three_LineSegments_get_rotation(segs), 0.45);
    Three_Group_add(g_wire_group, segs);
}

static void update_view(void) {
    js_scope();
    double r = 8.6;
    Three_Vector3_set(Three_PerspectiveCamera_get_position(g_camera), r * sin(g_azimuth), 3.6, r * cos(g_azimuth));
    Three_PerspectiveCamera_lookAt(g_camera, Three_Vector3_new(0.0, 0.0, 0.0));

    Three_Group_set_visible(g_wire_group, g_wire_visible);
    HTMLElement_set_textContent(g_wire_btn, g_wire_visible ? "Wireframes: On" : "Wireframes: Off");
    Three_WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_azimuth * 180.0 / PI + (g_azimuth >= 0.0 ? 0.5 : -0.5));
    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "5 monochrome solids \xC2\xB7 EdgesGeometry wireframes: %s \xC2\xB7 azimuth: %d deg \xC2\xB7 680x400 WebGL",
        g_wire_visible ? "on" : "off", deg
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_wire_visible = !g_wire_visible;
    else if (id == 1) g_azimuth -= PI / 6.0;
    else if (id == 2) g_azimuth += PI / 6.0;
    else { g_azimuth = 0.35; g_wire_visible = true; }
    update_view();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    double width = 680.0, height = 400.0;

    g_renderer = Three_WebGLRenderer_new(Three_WebGLRendererParameters_create(.antialias = true, .preserveDrawingBuffer = true));
    js_retain(g_renderer);
    Three_WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    Three_WebGLRenderer_setSize(g_renderer, width, height);

    HTMLElement stage = Document_getElementById(doc, "stage");
    HTMLElement_appendChild(stage, Three_WebGLRenderer_get_domElement(g_renderer));

    g_scene = Three_Scene_new();
    js_retain(g_scene);
    Three_Scene_set_background(g_scene, Three_Color_new(0xf7f7f7));
    g_camera = Three_PerspectiveCamera_new(42.0, width / height, 0.1, 100.0);
    js_retain(g_camera);

    Three_DirectionalLight key = Three_DirectionalLight_new(0xffffff, 2.2);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    Three_DirectionalLight rim = Three_DirectionalLight_new(0xffffff, 0.9);
    Three_Vector3_set(Three_DirectionalLight_get_position(rim), -7.0, 5.0, -6.0);

    Three_GridHelper grid = Three_GridHelper_new(12.0, 12.0, 0x737373, 0xd4d4d4);
    Three_Vector3_set_y(Three_GridHelper_get_position(grid), -1.2);

    g_wire_group = Three_Group_new();
    js_retain(g_wire_group);
    Three_Scene_add(g_scene, Three_AmbientLight_new(0xffffff, 1.2), key, rim, grid, g_wire_group);

    Three_LineBasicMaterial line_mat = Three_LineBasicMaterial_new(Three_LineBasicMaterialParameters_create(
        .color_number = 0x111111));

    add_solid(AS_Three_BufferGeometry(Three_BoxGeometry_new(1.45, 1.45, 1.45)),
              0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    add_solid(AS_Three_BufferGeometry(Three_IcosahedronGeometry_new(0.95, 0.0)),
              0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    add_solid(AS_Three_BufferGeometry(Three_TorusKnotGeometry_new(0.72, 0.24, 96.0, 16.0)),
              0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    add_solid(AS_Three_BufferGeometry(Three_OctahedronGeometry_new(0.95, 0.0)),
              0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    add_solid(AS_Three_BufferGeometry(Three_TorusGeometry_new(0.72, 0.23, 16.0, 48.0)),
              0x404040, 0.55, 0.12, false, 3.2, 0.05, -0.5, 22.0, line_mat);

    g_wire_btn = Document_getElementById(doc, "btn-wire");
    js_retain(g_wire_btn);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);

    static const char* ids[4] = { "btn-wire", "btn-left", "btn-right", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    update_view();
}
