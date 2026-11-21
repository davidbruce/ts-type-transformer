#include <math.h>
#include <stdio.h>
#include "dom/dom.h"
#include "three/three.h"

static const double PI = 3.14159265358979323846;

#define add_obj(parent, child) \
    Three_Object3D_add(AS_Three_Object3D(parent), JsArray_Three_Object3D_from((Three_Object3D[]){ AS_Three_Object3D(child) }, 1))

static double tower_height(int mode, int gx, int gz) {
    double fx = (double)gx - 4.5;
    double fz = (double)gz - 4.5;
    if (mode == 1) {
        double r = sqrt(fx * fx + fz * fz);
        return 0.35 + 0.42 * r + 0.25 * cos(fx * 0.8);
    } else if (mode == 2) {
        double diag = fabs(fx - fz);
        return 0.3 + 2.8 * exp(-diag * diag / 5.5) + 0.25 * sin(fz * 0.9);
    }
    double r2 = fx * fx + fz * fz;
    return 0.35 + 3.1 * exp(-r2 / 9.5) + 0.2 * cos(fx * 1.4) * cos(fz * 1.4);
}

static const char* layout_name(int mode) {
    if (mode == 1) return "Amphitheater";
    if (mode == 2) return "Diagonal";
    return "Plaza";
}

static Three_WebGLRenderer g_renderer;
static Three_Scene g_scene;
static Three_OrthographicCamera g_ortho;
static Three_PerspectiveCamera g_persp;
static Three_InstancedMesh g_inst;
static HTMLElement g_cam_btn;
static HTMLElement g_meta;
static bool g_use_ortho = true;
static int g_layout_mode = 0;
static double g_cam_yaw = 0.785398;

static void render_city(void) {
    js_scope_open();
    Three_Vector3 pos = Three_Vector3_new(0.0, 0.0, 0.0);
    Three_Quaternion rot_q = Three_Quaternion_new();
    Three_Vector3 scale = Three_Vector3_new(1.0, 1.0, 1.0);
    Three_Matrix4 mtx = Three_Matrix4_new();
    Three_Vector3 axis_y = Three_Vector3_new(0.0, 1.0, 0.0);
    Three_Color col = Three_Color_new(Three_ColorRepresentation_of(0xffffff));

    double max_h = 0.0;
    int idx = 0;
    for (int gz = 0; gz < 10; gz++) {
        for (int gx = 0; gx < 10; gx++) {
            double x = ((double)gx - 4.5) * 0.78;
            double z = ((double)gz - 4.5) * 0.78;
            double h = tower_height(g_layout_mode, gx, gz);
            if (h > max_h) max_h = h;
            Three_Vector3_set(pos, x, h * 0.5, z);
            Three_Quaternion_setFromAxisAngle(rot_q, axis_y, 0.08 * (double)((gx + gz) % 4));
            Three_Vector3_set(scale, 1.0, h, 1.0);
            Three_Matrix4_compose(mtx, pos, rot_q, scale);
            Three_InstancedMesh_setMatrixAt(g_inst, (double)idx, mtx);

            double t = (h - 0.3) / 3.2;
            if (t < 0.0) t = 0.0;
            if (t > 1.0) t = 1.0;
            double shade = 0.82 - 0.70 * t;
            Three_Color_setRGB(col, shade, shade, shade);
            Three_InstancedMesh_setColorAt(g_inst, (double)idx, col);
            idx++;
        }
    }

    Three_InstancedBufferAttribute_set_needsUpdate(Three_InstancedMesh_get_instanceMatrix(g_inst), true);
    Three_InstancedBufferAttribute icol = Three_InstancedMesh_get_instanceColor(g_inst);
    if (icol.h) Three_InstancedBufferAttribute_set_needsUpdate(icol, true);

    double radius = 11.5;
    double cx = radius * cos(g_cam_yaw);
    double cz = radius * sin(g_cam_yaw);
    Three_Vector3 target = Three_Vector3_new(0.0, 0.9, 0.0);

    Three_Vector3_set(Three_OrthographicCamera_get_position(g_ortho), cx, 8.2, cz);
    Three_OrthographicCamera_lookAt(g_ortho, target);
    Three_Vector3_set(Three_PerspectiveCamera_get_position(g_persp), cx, 7.6, cz);
    Three_PerspectiveCamera_lookAt(g_persp, target);

    if (g_use_ortho) {
        Three_WebGLRenderer_render(g_renderer, g_scene, g_ortho);
    } else {
        Three_WebGLRenderer_render(g_renderer, g_scene, g_persp);
    }

    const char* cname = g_use_ortho ? "Orthographic" : "Perspective";
    char cam_buf[64];
    snprintf(cam_buf, sizeof cam_buf, "Camera: %s", cname);
    HTMLElement_set_textContent(g_cam_btn, cam_buf);

    char meta_buf[256];
    snprintf(
        meta_buf, sizeof meta_buf,
        "100 InstancedMesh towers \xC2\xB7 layout: %s \xC2\xB7 peak height: %.2f \xC2\xB7 camera: %s",
        layout_name(g_layout_mode), max_h, cname
    );
    HTMLElement_set_textContent(g_meta, meta_buf);
    js_scope_close();
}

static const int BTN_TAGS[5] = { 0, 1, 2, 3, 4 };
static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    int id = *(const int*)ctx;
    if (id == 0) g_use_ortho = !g_use_ortho;
    else if (id >= 1 && id <= 3) g_layout_mode = id - 1;
    else g_cam_yaw += PI / 4.0;
    render_city();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    double width = 680.0, height = 400.0, aspect = width / height;

    Three_WebGLRendererParameters rp = Three_WebGLRendererParameters_create();
    Three_WebGLRendererParameters_set_antialias(rp, true);
    Three_WebGLRendererParameters_set_preserveDrawingBuffer(rp, true);
    g_renderer = Three_WebGLRenderer_new(rp);
    js_retain(g_renderer);
    Three_WebGLRenderer_setPixelRatio(g_renderer, 1.0);
    Three_WebGLRenderer_setSize(g_renderer, width, height);
    HTMLElement_appendChild(Document_getElementById(doc, "stage"), Three_WebGLRenderer_get_domElement(g_renderer));

    g_scene = Three_Scene_new();
    js_retain(g_scene);
    Three_Scene_set_background(g_scene, Three_Color_or_Texture_of(Three_Color_new(Three_ColorRepresentation_of(0xf7f7f7))));
    g_ortho = Three_OrthographicCamera_new(-5.4 * aspect, 5.4 * aspect, 5.4, -5.4, 0.1, 100.0);
    js_retain(g_ortho);
    g_persp = Three_PerspectiveCamera_new(40.0, aspect, 0.1, 100.0);
    js_retain(g_persp);

    add_obj(g_scene, Three_AmbientLight_new(Three_ColorRepresentation_of(0xffffff), 1.25));
    Three_DirectionalLight key = Three_DirectionalLight_new(Three_ColorRepresentation_of(0xffffff), 2.1);
    Three_Vector3_set(Three_DirectionalLight_get_position(key), 8.0, 12.0, 6.0);
    add_obj(g_scene, key);
    Three_DirectionalLight rim = Three_DirectionalLight_new(Three_ColorRepresentation_of(0xffffff), 0.9);
    Three_Vector3_set(Three_DirectionalLight_get_position(rim), -7.0, 6.0, -8.0);
    add_obj(g_scene, rim);

    Three_MeshStandardMaterialParameters pmp = Three_MeshStandardMaterialParameters_create();
    Three_MeshStandardMaterialParameters_set_color(pmp, Three_ColorRepresentation_of(0xe5e5e5));
    Three_MeshStandardMaterialParameters_set_roughness(pmp, 0.85);
    Three_Mesh plinth = Three_Mesh_new(Three_PlaneGeometry_new(8.6, 8.6), Three_MeshStandardMaterial_new(pmp));
    Three_Euler_set_x(Three_Mesh_get_rotation(plinth), -PI / 2.0);
    Three_Vector3_set_y(Three_Mesh_get_position(plinth), -0.01);
    add_obj(g_scene, plinth);

    add_obj(g_scene, Three_GridHelper_new(9.0, 10.0, Three_ColorRepresentation_of(0x999999), Three_ColorRepresentation_of(0xd4d4d4)));

    Three_BoxGeometry tgeo = Three_BoxGeometry_new(0.54, 1.0, 0.54);
    Three_MeshStandardMaterialParameters tmp = Three_MeshStandardMaterialParameters_create();
    Three_MeshStandardMaterialParameters_set_color(tmp, Three_ColorRepresentation_of(0xffffff));
    Three_MeshStandardMaterialParameters_set_roughness(tmp, 0.65);
    Three_MeshStandardMaterialParameters_set_metalness(tmp, 0.08);
    Three_MeshStandardMaterial tmat = Three_MeshStandardMaterial_new(tmp);
    g_inst = Three_InstancedMesh_new(tgeo, tmat, 100.0);
    js_retain(g_inst);
    add_obj(g_scene, g_inst);

    g_cam_btn = Document_getElementById(doc, "btn-cam");
    js_retain(g_cam_btn);
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);

    static const char* ids[5] = { "btn-cam", "btn-lay0", "btn-lay1", "btn-lay2", "btn-rot" };
    for (int i = 0; i < 5; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }
    render_city();
    js_scope_close();
}
