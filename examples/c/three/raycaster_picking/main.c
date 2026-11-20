#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Object3D_add(AS_Object3D(parent), JsArray_Object3D_from((Object3D[]){ AS_Object3D(child) }, 1))

static const char* NAMES[9] = {
    "Box", "Cone", "Cylinder", "Sphere", "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron"
};

static WebGLRenderer g_renderer;
static HTMLCanvasElement g_canvas;
static Scene g_scene;
static PerspectiveCamera g_camera;
static MeshStandardMaterial g_mats[9];
static Object3D g_meshes[9];
static JsArray_Object3D g_mesh_arr;
static Box3 g_box;
static Box3Helper g_helper;
static Raycaster g_raycaster;
static HTMLElement g_meta;
static double g_yaw = 0.45;
static int g_selected = -1;
static char g_hit_buf[160] = "none";

static void update_camera(void) {
    js_scope_open();
    double r = 8.4;
    Vector3_set(PerspectiveCamera_get_position(g_camera), r * sin(g_yaw), 5.1, r * cos(g_yaw));
    PerspectiveCamera_lookAt(g_camera, Vector3_new(0.0, 0.0, 0.0));
    PerspectiveCamera_updateMatrixWorld(g_camera, true);
    Scene_updateMatrixWorld(g_scene, true);
    js_scope_close();
}

static void render_scene(void) {
    js_scope_open();
    for (int i = 0; i < 9; i++) {
        Color_setHex(MeshStandardMaterial_get_color(g_mats[i]), i == g_selected ? 0x111111 : 0xd4d4d4);
    }
    if (g_selected >= 0 && g_selected < 9) {
        Box3_setFromObject(g_box, g_meshes[g_selected], true);
        Box3Helper_set_visible(g_helper, true);
    } else {
        Box3Helper_set_visible(g_helper, false);
    }
    WebGLRenderer_render(g_renderer, g_scene, g_camera);

    char meta_buf[256];
    snprintf(meta_buf, sizeof meta_buf, "picked: %s \xC2\xB7 9 specimens \xC2\xB7 click canvas or buttons", g_hit_buf);
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
}

static void cast_at_ndc(double nx, double ny) {
    js_scope_open();
    Raycaster_setFromCamera(g_raycaster, Vector2_new(nx, ny), g_camera);

    JsArray_Intersection hits = Raycaster_intersectObjects(g_raycaster, g_mesh_arr, false);
    if (JsArray_Intersection_length(hits) > 0) {
        Intersection hit = JsArray_Intersection_get(hits, 0);
        Object3D hobj = Intersection_get_object(hit);
        int found = -1;
        for (int i = 0; i < 9; i++) {
            if (js_same(g_meshes[i], hobj)) { found = i; break; }
        }
        g_selected = found;
        if (found >= 0) {
            Box3_setFromObject(g_box, g_meshes[found], true);
            Vector3 sz = Box3_getSize(g_box, Vector3_new());
            Vector3 pt = Intersection_get_point(hit);
            snprintf(
                g_hit_buf, sizeof g_hit_buf,
                "%s at d=%.2f (%.2f, %.2f, %.2f) box=%.2fx%.2fx%.2f",
                NAMES[found], Intersection_get_distance(hit),
                Vector3_get_x(pt), Vector3_get_y(pt), Vector3_get_z(pt),
                Vector3_get_x(sz), Vector3_get_y(sz), Vector3_get_z(sz)
            );
        }
    } else {
        g_selected = -1;
        snprintf(g_hit_buf, sizeof g_hit_buf, "miss at ndc (%.2f, %.2f)", nx, ny);
    }
    render_scene();
    js_scope_close();
}

static void pick_specimen(int idx) {
    js_scope_open();
    update_camera();
    Vector3 wp = Object3D_getWorldPosition(g_meshes[idx], Vector3_new());
    Vector3 proj = Vector3_project(wp, g_camera);
    cast_at_ndc(Vector3_get_x(proj), Vector3_get_y(proj));
    js_scope_close();
}

static void on_canvas_click(void* ctx, PointerEvent ev) {
    (void)ctx;
    js_scope_open();
    DOMRect rect = HTMLCanvasElement_getBoundingClientRect(g_canvas);
    double nx = ((PointerEvent_get_clientX(ev) - DOMRect_get_left(rect)) / DOMRect_get_width(rect)) * 2.0 - 1.0;
    double ny = -((PointerEvent_get_clientY(ev) - DOMRect_get_top(rect)) / DOMRect_get_height(rect)) * 2.0 + 1.0;
    update_camera();
    cast_at_ndc(nx, ny);
    js_scope_close();
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) pick_specimen(4);
    else if (id == 1) pick_specimen(0);
    else if (id == 2) pick_specimen(8);
    else if (id == 3) { g_yaw += PI / 4.0; update_camera(); render_scene(); }
    else {
        g_selected = -1;
        snprintf(g_hit_buf, sizeof g_hit_buf, "none");
        update_camera();
        render_scene();
    }
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
    g_canvas = WebGLRenderer_get_domElement(g_renderer);
    js_retain(g_canvas);
    HTMLElement_appendChild(Document_getElementById(doc, "stage"), g_canvas);

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

    GridHelper grid = GridHelper_new(9.0, 9.0, ColorRepresentation_of(0x999999), ColorRepresentation_of(0xd4d4d4));
    Vector3_set_y(GridHelper_get_position(grid), -0.85);
    add_obj(g_scene, grid);

    BufferGeometry geos[9] = {
        AS_BufferGeometry(BoxGeometry_new(1.05, 1.05, 1.05)),
        AS_BufferGeometry(ConeGeometry_new(0.62, 1.25, 20.0)),
        AS_BufferGeometry(CylinderGeometry_new(0.48, 0.48, 1.2, 24.0)),
        AS_BufferGeometry(SphereGeometry_new(0.62, 24.0, 18.0)),
        AS_BufferGeometry(OctahedronGeometry_new(0.68, 0.0)),
        AS_BufferGeometry(DodecahedronGeometry_new(0.62, 0.0)),
        AS_BufferGeometry(TorusGeometry_new(0.5, 0.18, 14.0, 36.0)),
        AS_BufferGeometry(IcosahedronGeometry_new(0.64, 0.0)),
        AS_BufferGeometry(TetrahedronGeometry_new(0.72, 0.0)),
    };

    for (int i = 0; i < 9; i++) {
        int gx = (i % 3) - 1;
        int gz = (i / 3) - 1;
        MeshStandardMaterialParameters mp = MeshStandardMaterialParameters_create();
        MeshStandardMaterialParameters_set_color(mp, ColorRepresentation_of(0xd4d4d4));
        MeshStandardMaterialParameters_set_roughness(mp, 0.55);
        MeshStandardMaterialParameters_set_metalness(mp, 0.1);
        MeshStandardMaterialParameters_set_flatShading(mp, i >= 4);
        g_mats[i] = MeshStandardMaterial_new(mp);
        js_retain(g_mats[i]);

        Mesh m = Mesh_new(geos[i], g_mats[i]);
        Object3D mo = AS_Object3D(m);
        Object3D_set_name(mo, NAMES[i]);
        Vector3_set(Object3D_get_position(mo), (double)gx * 2.1, 0.0, (double)gz * 2.1);
        Euler_set_x(Object3D_get_rotation(mo), 0.2);
        Euler_set_y(Object3D_get_rotation(mo), 0.35 * (double)i);
        add_obj(g_scene, mo);
        g_meshes[i] = mo;
        js_retain(g_meshes[i]);
    }
    g_mesh_arr = JsArray_Object3D_from(g_meshes, 9);
    js_retain(g_mesh_arr);

    g_box = Box3_new(Vector3_new(-0.5, -0.5, -0.5), Vector3_new(0.5, 0.5, 0.5));
    js_retain(g_box);
    g_helper = Box3Helper_new(g_box, ColorRepresentation_of(0x111111));
    js_retain(g_helper);
    Box3Helper_set_visible(g_helper, false);
    add_obj(g_scene, g_helper);

    g_raycaster = Raycaster_new(Vector3_new(0.0, 0.0, 0.0), Vector3_new(0.0, 0.0, -1.0));
    js_retain(g_raycaster);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);

    HTMLCanvasElement_addEventListener_click(g_canvas, HTMLElement_PointerEvent_fn_from(on_canvas_click, NULL));

    static const char* ids[5] = { "btn-p0", "btn-p1", "btn-p2", "btn-orbit", "btn-clear" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    pick_specimen(4);
    js_scope_close();
}
