#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Object3D_add(AS_Object3D(parent), JsArray_Object3D_from((Object3D[]){ AS_Object3D(child) }, 1))

static WebGLRenderer g_renderer;
static Scene g_scene;
static PerspectiveCamera g_camera;
static Group g_wire_group;
static HTMLElement g_wire_btn;
static HTMLElement g_meta;
static bool g_wire_visible = true;
static double g_azimuth = 0.35;

static void add_solid(BufferGeometry geo, uint32_t hex, double rough, double metal, bool flat,
                      double x, double y, double z, double thresh, LineBasicMaterial line_mat) {
    MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
    MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of(hex));
    MeshStandardMaterialParameters_set_roughness(mp, rough);
    MeshStandardMaterialParameters_set_metalness(mp, metal);
    MeshStandardMaterialParameters_set_flatShading(mp, flat);
    MeshStandardMaterial mat = MeshStandardMaterial_new(mp);

    Mesh mesh = Mesh_new(geo, mat);
    Vector3_set(Mesh_get_position(mesh), x, y, z);
    Euler_set_x(Mesh_get_rotation(mesh), 0.25);
    Euler_set_y(Mesh_get_rotation(mesh), 0.45);
    add_obj(g_scene, mesh);

    EdgesGeometry egeo = EdgesGeometry_new(geo, thresh);
    LineSegments segs = LineSegments_new(egeo, line_mat);
    Vector3_set(LineSegments_get_position(segs), x, y, z);
    Euler_set_x(LineSegments_get_rotation(segs), 0.25);
    Euler_set_y(LineSegments_get_rotation(segs), 0.45);
    add_obj(g_wire_group, segs);
}

static void update_view(void) {
    js_scope_open();
    double r = 8.6;
    Vector3_set(PerspectiveCamera_get_position(g_camera), r * sin(g_azimuth), 3.6, r * cos(g_azimuth));
    PerspectiveCamera_lookAt(g_camera, Vector3_new(0.0, 0.0, 0.0));

    Group_set_visible(g_wire_group, g_wire_visible);
    HTMLElement_set_textContent(g_wire_btn, g_wire_visible ? "Wireframes: On" : "Wireframes: Off");
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    int deg = (int)(g_azimuth * 180.0 / PI + (g_azimuth >= 0.0 ? 0.5 : -0.5));
    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "5 monochrome solids \xC2\xB7 EdgesGeometry wireframes: %s \xC2\xB7 azimuth: %d deg \xC2\xB7 680x400 WebGL",
        g_wire_visible ? "on" : "off", deg
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
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

    HTMLElement stage = Document_getElementById(doc, "stage");
    HTMLElement_appendChild(stage, WebGLRenderer_get_domElement(g_renderer));

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

    GridHelper grid = GridHelper_new(12.0, 12.0, ColorRepresentation_of(0x737373), ColorRepresentation_of(0xd4d4d4));
    Vector3_set_y(GridHelper_get_position(grid), -1.2);
    add_obj(g_scene, grid);

    g_wire_group = Group_new();
    js_retain(g_wire_group);
    add_obj(g_scene, g_wire_group);

    LineBasicMaterialParameters lmp = LineBasicMaterialParameters_create();
    LineBasicMaterialParameters_set_color(lmp, ColorRepresentation_of(0x111111));
    LineBasicMaterial line_mat = LineBasicMaterial_new(lmp);

    add_solid(AS_BufferGeometry(BoxGeometry_new(1.45, 1.45, 1.45)),
              0x262626, 0.65, 0.1, false, -3.2, 0.0, -0.6, 1.0, line_mat);
    add_solid(AS_BufferGeometry(IcosahedronGeometry_new(0.95, 0.0)),
              0xd4d4d4, 0.55, 0.08, true, -1.55, 0.1, 0.55, 1.0, line_mat);
    add_solid(AS_BufferGeometry(TorusKnotGeometry_new(0.72, 0.24, 96.0, 16.0)),
              0x737373, 0.45, 0.15, false, 0.0, 0.25, -0.2, 24.0, line_mat);
    add_solid(AS_BufferGeometry(OctahedronGeometry_new(0.95, 0.0)),
              0xefefef, 0.5, 0.05, true, 1.6, 0.1, 0.55, 1.0, line_mat);
    add_solid(AS_BufferGeometry(TorusGeometry_new(0.72, 0.23, 16.0, 48.0)),
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
    js_scope_close();
}
