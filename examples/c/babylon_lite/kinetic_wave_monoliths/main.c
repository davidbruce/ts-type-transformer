#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static Babylon_EngineContext g_engine;
static Babylon_ObservableVec3 g_col_pos[36];
static Babylon_ObservableVec3 g_col_scale[36];
static double g_col_x[36];
static double g_col_z[36];
static double g_col_dist[36];
static double g_col_diag[36];
static Babylon_ObservableVec3 g_cap_pos[4];
static Babylon_EulerProxy g_cap_rot[4];
static Babylon_ObservableVec3 g_light1_pos;
static Babylon_ObservableVec3 g_light2_pos;
static double g_elapsed = 0.0;

static void set_status_error(JsValue err) {
    js_scope();
    Document doc = get_document();
    HTMLElement status = Document_getElementById(doc, "status");
    if (status.h != 0) {
        char msg[272], buf[320];
        JsString_copy(js_error_message(err), msg, sizeof msg);
        snprintf(buf, sizeof buf, "WebGPU / Babylon Lite error: %s", msg);
        HTMLElement_set_textContent(status, buf);
    }
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    g_elapsed += delta_ms * 0.001;
    for (int idx = 0; idx < 36; idx++) {
        double h = 0.55
            + 1.15 * (0.5 + 0.5 * sin(g_elapsed * 2.2 - g_col_dist[idx] * 1.15))
            + 0.30 * cos(g_elapsed * 1.3 + g_col_diag[idx] * 0.7);
        Babylon_ObservableVec3_set(g_col_scale[idx], 1.0, h, 1.0);
        Babylon_ObservableVec3_set(g_col_pos[idx], g_col_x[idx], h * 0.5, g_col_z[idx]);
    }
    for (int k = 0; k < 4; k++) {
        double fk = (double)k;
        double theta = g_elapsed * 0.7 + fk * (PI * 0.5);
        Babylon_ObservableVec3_set(
            g_cap_pos[k],
            cos(theta) * 4.9,
            2.6 + sin(g_elapsed * 1.6 + fk * 1.57) * 0.55,
            sin(theta) * 4.9
        );
        Babylon_EulerProxy_set_z(g_cap_rot[k], sin(g_elapsed + fk) * 0.35);
        Babylon_EulerProxy_set_y(g_cap_rot[k], -theta);
    }
    Babylon_ObservableVec3_set(g_light1_pos, cos(g_elapsed * 0.65) * 5.2, 4.2, sin(g_elapsed * 0.65) * 5.2);
    Babylon_ObservableVec3_set(g_light2_pos, cos(-g_elapsed * 0.52) * 5.2, 3.4, sin(-g_elapsed * 0.52) * 5.2);
}

static void on_started(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) set_status_error(value);
}

static void on_registered(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) {
        set_status_error(value);
        return;
    }
    js_scope();
    Promise_then(babylon_startEngine(g_engine), on_started, 0);
}

static Babylon_PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    Babylon_createPbrMaterial_props props = Babylon_createPbrMaterial_props_create(
        .baseColorFactor = JsTuple_double_double_double_double_new(v, v, v, 1.0), .metallicFactor = metallic,
        .roughnessFactor = roughness
    );
    return babylon_createPbrMaterial(props);
}

static void on_engine(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) {
        set_status_error(value);
        return;
    }
    js_scope();
    g_engine = (Babylon_EngineContext){ value.h };
    js_retain(g_engine);

    Babylon_SceneContext scene = babylon_createSceneContext(g_engine);
    Babylon_SceneContext_set_clearColor(scene, GPUColorDict_create(.r = 0.05, .g = 0.05, .b = 0.05, .a = 1.0));

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 4.0, PI / 3.1, 12.8, babylon_vec3(0.0, 1.4, 0.0));
    babylon_addToScene(scene, AS_Babylon_Camera(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.15), 0.55);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.76, 0.76, 0.76));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.10, 0.10, 0.10));
    babylon_addToScene(scene, AS_Babylon_LightBase(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.2, -0.8), 1.35);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(0.90, 0.90, 0.90));
    babylon_addToScene(scene, AS_Babylon_LightBase(dir));

    Babylon_PointLight light1 = babylon_createPointLight(JsTuple_double_double_double_new(5.2, 4.2, 0.0), 10.0);
    Babylon_PointLight_set_diffuse(light1, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    Babylon_PointLight_set_range(light1, 16.0);
    babylon_addToScene(scene, AS_Babylon_LightBase(light1));
    g_light1_pos = Babylon_PointLight_get_position(light1);
    js_retain(g_light1_pos);

    Babylon_PointLight light2 = babylon_createPointLight(JsTuple_double_double_double_new(-5.2, 3.4, 0.0), 7.5);
    Babylon_PointLight_set_diffuse(light2, JsTuple_double_double_double_new(0.78, 0.78, 0.78));
    Babylon_PointLight_set_range(light2, 16.0);
    babylon_addToScene(scene, AS_Babylon_LightBase(light2));
    g_light2_pos = Babylon_PointLight_get_position(light2);
    js_retain(g_light2_pos);

    Babylon_Mesh floor_m = babylon_createGround(g_engine, Babylon_GroundOptions_create(.width = 18.0, .height = 18.0));
    Babylon_Mesh_set_material(floor_m, mk_pbr(0.09, 0.10, 0.88));
    babylon_addToScene(scene, floor_m);

    const double shades[4] = { 0.22, 0.44, 0.68, 0.88 };
    Babylon_PbrMaterialProps col_mats[4];
    for (int i = 0; i < 4; i++) col_mats[i] = mk_pbr(shades[i], 0.35, 0.36);

    Babylon_BoxOptions bopts = Babylon_BoxOptions_create(.width = 0.72, .height = 1.0, .depth = 0.72);

    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 6; col++) {
            int idx = row * 6 + col;
            double fx = (double)col - 2.5;
            double fz = (double)row - 2.5;
            double x = fx * 1.15;
            double z = fz * 1.15;
            g_col_x[idx] = x;
            g_col_z[idx] = z;
            g_col_dist[idx] = sqrt(fx * fx + fz * fz);
            g_col_diag[idx] = fx - fz;

            Babylon_Mesh m = babylon_createBox_with_BoxOptions(g_engine, bopts);
            g_col_pos[idx] = Babylon_Mesh_get_position(m);
            g_col_scale[idx] = Babylon_Mesh_get_scaling(m);
            js_retain(g_col_pos[idx]);
            js_retain(g_col_scale[idx]);
            Babylon_ObservableVec3_set(g_col_pos[idx], x, 0.5, z);
            Babylon_Mesh_set_material(m, col_mats[(row + col) % 4]);
            babylon_addToScene(scene, m);
        }
    }

    Babylon_createPbrMaterial_props cprops = Babylon_createPbrMaterial_props_create(
        .baseColorFactor = JsTuple_double_double_double_double_new(0.92, 0.92, 0.92, 1.0), .metallicFactor = 0.50,
        .roughnessFactor = 0.15
    );
    babylon_setPbrClearCoat(cprops, Babylon_ClearCoatProps_create(.isEnabled = true, .intensity = 1.0, .roughness = 0.08));
    babylon_setPbrEmissive(cprops, JsTuple_double_double_double_new(0.16, 0.16, 0.16));
    Babylon_PbrMaterialProps cap_mat = babylon_createPbrMaterial(cprops);

    Babylon_CapsuleOptions copts = Babylon_CapsuleOptions_create(.height = 1.15, .radius = 0.22, .tessellation = 20.0);
    for (int k = 0; k < 4; k++) {
        Babylon_Mesh cap = babylon_createCapsule(g_engine, copts);
        g_cap_pos[k] = Babylon_Mesh_get_position(cap);
        g_cap_rot[k] = Babylon_Mesh_get_rotation(cap);
        js_retain(g_cap_pos[k]);
        js_retain(g_cap_rot[k]);
        Babylon_Mesh_set_material(cap, cap_mat);
        babylon_addToScene(scene, cap);
    }

    babylon_onBeforeRender(scene, Babylon_number_fn_from(on_before_render, 0));
    Promise_then(babylon_registerScene(scene), on_registered, 0);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_canvas = Document_querySelector_canvas(get_document());
    if (g_canvas.h != 0) {
        js_retain(g_canvas);
        Promise_then(babylon_createEngine_with_EngineOptions(g_canvas), on_engine, 0);
    }
}
