#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Three_Object3D_add(AS_Three_Object3D(parent), JsArray_Three_Object3D_from((Three_Object3D[]){ AS_Three_Object3D(child) }, 1))

static const char* NAMES[9] = {
    "Box", "Cone", "Cylinder", "Sphere", "Octahedron",
    "Dodecahedron", "Torus", "Icosahedron", "Tetrahedron"
};

static Three_WebGLRenderer g_renderer;
static HTMLCanvasElement g_canvas;
static Three_Scene g_scene;
static Three_PerspectiveCamera g_camera;
static Three_MeshStandardMaterial g_mats[9];
static Three_Object3D g_meshes[9];
static JsArray_Three_Object3D g_mesh_arr;
static Three_Box3 g_box;
static Three_Box3Helper g_helper;
static Three_Raycaster g_raycaster;
static HTMLElement g_meta;
static double g_yaw = 0.45;
static int g_selected = -1;
static char g_hit_buf[160] = "none";

static void update_camera(void) {
    js_scope();
    double r = 8.4;
    Three_Vector3_set(Three_PerspectiveCamera_get_position(g_camera), r * sin(g_yaw), 5.1, r * cos(g_yaw));
    Three_PerspectiveCamera_lookAt(g_camera, Three_Vector3_new(0.0, 0.0, 0.0));
    Three_PerspectiveCamera_updateMatrixWorld(g_camera, true);
    Three_Scene_updateMatrixWorld(g_scene, true);
}

static void render_scene(void) {
    js_scope();
    for (int i = 0; i < 9; i++) {
        Three_Color_setHex(Three_MeshStandardMaterial_get_color(g_mats[i]), i == g_selected ? 0x111111 : 0xd4d4d4);
    }
    if (g_selected >= 0 && g_selected < 9) {
        Three_Box3_setFromObject(g_box, g_meshes[g_selected], true);
        Three_Box3Helper_set_visible(g_helper, true);
    } else {
        Three_Box3Helper_set_visible(g_helper, false);
    }
    Three_WebGLRenderer_render(g_renderer, g_scene, g_camera);

    char meta_buf[256];
    snprintf(meta_buf, sizeof meta_buf, "picked: %s \xC2\xB7 9 specimens \xC2\xB7 click canvas or buttons", g_hit_buf);
    HTMLElement_set_textContent(g_meta, meta_buf);
}

static void cast_at_ndc(double nx, double ny) {
    js_scope();
    Three_Raycaster_setFromCamera(g_raycaster, Three_Vector2_new(nx, ny), g_camera);

    JsArray_Three_Intersection hits = Three_Raycaster_intersectObjects(g_raycaster, g_mesh_arr, false);
    if (JsArray_Three_Intersection_length(hits) > 0) {
        Three_Intersection hit = JsArray_Three_Intersection_get(hits, 0);
        Three_Object3D hobj = Three_Intersection_get_object(hit);
        int found = -1;
        for (int i = 0; i < 9; i++) {
            if (js_same(g_meshes[i], hobj)) { found = i; break; }
        }
        g_selected = found;
        if (found >= 0) {
            Three_Box3_setFromObject(g_box, g_meshes[found], true);
            Three_Vector3 sz = Three_Box3_getSize(g_box, Three_Vector3_new());
            Three_Vector3 pt = Three_Intersection_get_point(hit);
            snprintf(
                g_hit_buf, sizeof g_hit_buf,
                "%s at d=%.2f (%.2f, %.2f, %.2f) box=%.2fx%.2fx%.2f",
                NAMES[found], Three_Intersection_get_distance(hit),
                Three_Vector3_get_x(pt), Three_Vector3_get_y(pt), Three_Vector3_get_z(pt),
                Three_Vector3_get_x(sz), Three_Vector3_get_y(sz), Three_Vector3_get_z(sz)
            );
        }
    } else {
        g_selected = -1;
        snprintf(g_hit_buf, sizeof g_hit_buf, "miss at ndc (%.2f, %.2f)", nx, ny);
    }
    render_scene();
}

static void pick_specimen(int idx) {
    js_scope();
    update_camera();
    Three_Vector3 wp = Three_Object3D_getWorldPosition(g_meshes[idx], Three_Vector3_new());
    Three_Vector3 proj = Three_Vector3_project(wp, g_camera);
    cast_at_ndc(Three_Vector3_get_x(proj), Three_Vector3_get_y(proj));
}

static void on_canvas_click(void* ctx, PointerEvent ev) {
    (void)ctx;
    js_scope();
    DOMRect rect = HTMLCanvasElement_getBoundingClientRect(g_canvas);
    double nx = ((PointerEvent_get_clientX(ev) - DOMRect_get_left(rect)) / DOMRect_get_width(rect)) * 2.0 - 1.0;
    double ny = -((PointerEvent_get_clientY(ev) - DOMRect_get_top(rect)) / DOMRect_get_height(rect)) * 2.0 + 1.0;
    update_camera();
    cast_at_ndc(nx, ny);
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
    js_scope();
    Document doc = get_document();
    double width = 680.0, height = 400.0;

    g_renderer = Three_WebGLRenderer_new(Three_WebGLRendererParameters_create(.antialias = true, .preserveDrawingBuffer = true));
    js_retain(g_renderer);
    Three_WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    Three_WebGLRenderer_setSize(g_renderer, width, height);
    g_canvas = Three_WebGLRenderer_get_domElement(g_renderer);
    js_retain(g_canvas);
    HTMLElement_appendChild(Document_getElementById(doc, "stage"), g_canvas);

    g_scene = Three_Scene_new();
    js_retain(g_scene);
    Three_Scene_set_background(g_scene, Three_Color_new(0xf7f7f7));
    g_camera = Three_PerspectiveCamera_new(42.0, width / height, 0.1, 100.0);
    js_retain(g_camera);

    add_obj(g_scene, Three_AmbientLight_new(0xffffff, 1.2));
    Three_DirectionalLight key = Three_DirectionalLight_new(0xffffff, 2.2);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 6.0, 10.0, 8.0);
    add_obj(g_scene, key);
    Three_DirectionalLight rim = Three_DirectionalLight_new(0xffffff, 0.9);
    Three_Vector3_set(Three_DirectionalLight_get_position(rim), -7.0, 5.0, -6.0);
    add_obj(g_scene, rim);

    Three_GridHelper grid = Three_GridHelper_new(9.0, 9.0, 0x999999, 0xd4d4d4);
    Three_Vector3_set_y(Three_GridHelper_get_position(grid), -0.85);
    add_obj(g_scene, grid);

    Three_BufferGeometry geos[9] = {
        AS_Three_BufferGeometry(Three_BoxGeometry_new(1.05, 1.05, 1.05)),
        AS_Three_BufferGeometry(Three_ConeGeometry_new(0.62, 1.25, 20.0)),
        AS_Three_BufferGeometry(Three_CylinderGeometry_new(0.48, 0.48, 1.2, 24.0)),
        AS_Three_BufferGeometry(Three_SphereGeometry_new(0.62, 24.0, 18.0)),
        AS_Three_BufferGeometry(Three_OctahedronGeometry_new(0.68, 0.0)),
        AS_Three_BufferGeometry(Three_DodecahedronGeometry_new(0.62, 0.0)),
        AS_Three_BufferGeometry(Three_TorusGeometry_new(0.5, 0.18, 14.0, 36.0)),
        AS_Three_BufferGeometry(Three_IcosahedronGeometry_new(0.64, 0.0)),
        AS_Three_BufferGeometry(Three_TetrahedronGeometry_new(0.72, 0.0)),
    };

    for (int i = 0; i < 9; i++) {
        int gx = (i % 3) - 1;
        int gz = (i / 3) - 1;
        g_mats[i] = Three_MeshStandardMaterial_new(Three_MeshStandardMaterialParameters_create(
            .color = Three_ColorRepresentation_of(0xd4d4d4), .roughness = 0.55, .metalness = 0.1, .flatShading = i >= 4));
        js_retain(g_mats[i]);

        Three_Mesh m = Three_Mesh_new(geos[i], g_mats[i]);
        Three_Object3D mo = AS_Three_Object3D(m);
        Three_Object3D_set_name(mo, NAMES[i]);
        Three_Vector3_set(Three_Object3D_get_position(mo), (double)gx * 2.1, 0.0, (double)gz * 2.1);
        Three_Euler_set_x(Three_Object3D_get_rotation(mo), 0.2);
        Three_Euler_set_y(Three_Object3D_get_rotation(mo), 0.35 * (double)i);
        add_obj(g_scene, mo);
        g_meshes[i] = mo;
        js_retain(g_meshes[i]);
    }
    g_mesh_arr = JsArray_Three_Object3D_from(g_meshes, 9);
    js_retain(g_mesh_arr);

    g_box = Three_Box3_new(Three_Vector3_new(-0.5, -0.5, -0.5), Three_Vector3_new(0.5, 0.5, 0.5));
    js_retain(g_box);
    g_helper = Three_Box3Helper_new(g_box, 0x111111);
    js_retain(g_helper);
    Three_Box3Helper_set_visible(g_helper, false);
    add_obj(g_scene, g_helper);

    g_raycaster = Three_Raycaster_new(Three_Vector3_new(0.0, 0.0, 0.0), Three_Vector3_new(0.0, 0.0, -1.0));
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
}
