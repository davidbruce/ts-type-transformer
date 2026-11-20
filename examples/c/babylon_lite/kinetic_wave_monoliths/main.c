#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static ObservableVec3 g_col_pos[36];
static ObservableVec3 g_col_scale[36];
static double g_col_x[36];
static double g_col_z[36];
static double g_col_dist[36];
static double g_col_diag[36];
static ObservableVec3 g_cap_pos[4];
static EulerProxy g_cap_rot[4];
static ObservableVec3 g_light1_pos;
static ObservableVec3 g_light2_pos;
static double g_elapsed = 0.0;

static void set_status_error(JsValue err) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement status = Document_getElementById(doc, "status");
    if (status.h != 0) {
        char msg[272], buf[320];
        JsString_copy(js_error_message(err), msg, sizeof msg);
        snprintf(buf, sizeof buf, "WebGPU / Babylon Lite error: %s", msg);
        HTMLElement_set_textContent(status, buf);
    }
    js_scope_close();
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    g_elapsed += delta_ms * 0.001;
    for (int idx = 0; idx < 36; idx++) {
        double h = 0.55
            + 1.15 * (0.5 + 0.5 * sin(g_elapsed * 2.2 - g_col_dist[idx] * 1.15))
            + 0.30 * cos(g_elapsed * 1.3 + g_col_diag[idx] * 0.7);
        ObservableVec3_set(g_col_scale[idx], 1.0, h, 1.0);
        ObservableVec3_set(g_col_pos[idx], g_col_x[idx], h * 0.5, g_col_z[idx]);
    }
    for (int k = 0; k < 4; k++) {
        double fk = (double)k;
        double theta = g_elapsed * 0.7 + fk * (PI * 0.5);
        ObservableVec3_set(
            g_cap_pos[k],
            cos(theta) * 4.9,
            2.6 + sin(g_elapsed * 1.6 + fk * 1.57) * 0.55,
            sin(theta) * 4.9
        );
        EulerProxy_set_z(g_cap_rot[k], sin(g_elapsed + fk) * 0.35);
        EulerProxy_set_y(g_cap_rot[k], -theta);
    }
    ObservableVec3_set(g_light1_pos, cos(g_elapsed * 0.65) * 5.2, 4.2, sin(g_elapsed * 0.65) * 5.2);
    ObservableVec3_set(g_light2_pos, cos(-g_elapsed * 0.52) * 5.2, 3.4, sin(-g_elapsed * 0.52) * 5.2);
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
    js_scope_open();
    Promise_then(startEngine(g_engine), on_started, 0);
    js_scope_close();
}

static PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    createPbrMaterial_props props = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(props, JsTuple_double_double_double_double_new(v, v, v, 1.0));
    createPbrMaterial_props_set_metallicFactor(props, metallic);
    createPbrMaterial_props_set_roughnessFactor(props, roughness);
    return createPbrMaterial(props);
}

static void on_engine(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) {
        set_status_error(value);
        return;
    }
    js_scope_open();
    g_engine = (EngineContext){ value.h };
    js_retain(g_engine);

    SceneContext scene = createSceneContext(g_engine);
    GPUColorDict clear = GPUColorDict_create();
    GPUColorDict_set_r(clear, 0.05);
    GPUColorDict_set_g(clear, 0.05);
    GPUColorDict_set_b(clear, 0.05);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.1, 12.8, vec3(0.0, 1.4, 0.0));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.15), 0.55);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.76, 0.76, 0.76));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.10, 0.10, 0.10));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.2, -0.8), 1.35);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(0.90, 0.90, 0.90));
    addToScene(scene, js_value(dir));

    PointLight light1 = createPointLight(JsTuple_double_double_double_new(5.2, 4.2, 0.0), 10.0);
    PointLight_set_diffuse(light1, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    PointLight_set_range(light1, 16.0);
    addToScene(scene, js_value(light1));
    g_light1_pos = PointLight_get_position(light1);
    js_retain(g_light1_pos);

    PointLight light2 = createPointLight(JsTuple_double_double_double_new(-5.2, 3.4, 0.0), 7.5);
    PointLight_set_diffuse(light2, JsTuple_double_double_double_new(0.78, 0.78, 0.78));
    PointLight_set_range(light2, 16.0);
    addToScene(scene, js_value(light2));
    g_light2_pos = PointLight_get_position(light2);
    js_retain(g_light2_pos);

    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, 18.0);
    GroundOptions_set_height(gopts, 18.0);
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, mk_pbr(0.09, 0.10, 0.88));
    addToScene(scene, js_value(floor_m));

    const double shades[4] = { 0.22, 0.44, 0.68, 0.88 };
    PbrMaterialProps col_mats[4];
    for (int i = 0; i < 4; i++) col_mats[i] = mk_pbr(shades[i], 0.35, 0.36);

    BoxOptions bopts = BoxOptions_create();
    BoxOptions_set_width(bopts, 0.72);
    BoxOptions_set_height(bopts, 1.0);
    BoxOptions_set_depth(bopts, 0.72);

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

            Mesh m = createBox_with_BoxOptions(g_engine, bopts);
            g_col_pos[idx] = Mesh_get_position(m);
            g_col_scale[idx] = Mesh_get_scaling(m);
            js_retain(g_col_pos[idx]);
            js_retain(g_col_scale[idx]);
            ObservableVec3_set(g_col_pos[idx], x, 0.5, z);
            Mesh_set_material(m, col_mats[(row + col) % 4]);
            addToScene(scene, js_value(m));
        }
    }

    createPbrMaterial_props cprops = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(cprops, JsTuple_double_double_double_double_new(0.92, 0.92, 0.92, 1.0));
    createPbrMaterial_props_set_metallicFactor(cprops, 0.50);
    createPbrMaterial_props_set_roughnessFactor(cprops, 0.15);
    ClearCoatProps cc = ClearCoatProps_create();
    ClearCoatProps_set_isEnabled(cc, true);
    ClearCoatProps_set_intensity(cc, 1.0);
    ClearCoatProps_set_roughness(cc, 0.08);
    setPbrClearCoat(cprops, cc);
    setPbrEmissive(cprops, JsTuple_double_double_double_new(0.16, 0.16, 0.16));
    PbrMaterialProps cap_mat = createPbrMaterial(cprops);

    CapsuleOptions copts = CapsuleOptions_create();
    CapsuleOptions_set_height(copts, 1.15);
    CapsuleOptions_set_radius(copts, 0.22);
    CapsuleOptions_set_tessellation(copts, 20.0);
    for (int k = 0; k < 4; k++) {
        Mesh cap = createCapsule(g_engine, copts);
        g_cap_pos[k] = Mesh_get_position(cap);
        g_cap_rot[k] = Mesh_get_rotation(cap);
        js_retain(g_cap_pos[k]);
        js_retain(g_cap_rot[k]);
        Mesh_set_material(cap, cap_mat);
        addToScene(scene, js_value(cap));
    }

    onBeforeRender(scene, number_fn_from(on_before_render, 0));
    Promise_then(registerScene(scene), on_registered, 0);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_canvas = Document_querySelector_canvas(get_document());
    if (g_canvas.h != 0) {
        js_retain(g_canvas);
        Promise_then(createEngine_with_EngineOptions(RenderCanvas_of(g_canvas)), on_engine, 0);
    }
    js_scope_close();
}
