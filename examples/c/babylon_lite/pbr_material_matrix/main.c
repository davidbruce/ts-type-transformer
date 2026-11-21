#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static Babylon_EngineContext g_engine;
static Babylon_ObservableVec3 g_orb_pos;
static Babylon_ObservableVec3 g_orb_light_pos;
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
    double x = cos(g_elapsed * 0.85) * 4.5;
    double y = 3.0 + sin(g_elapsed * 1.4) * 0.65;
    double z = sin(g_elapsed * 0.85) * 4.5;
    Babylon_ObservableVec3_set(g_orb_pos, x, y, z);
    Babylon_ObservableVec3_set(g_orb_light_pos, x, y, z);
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
    Promise_then(babylon_startEngine(g_engine), on_started, 0);
    js_scope_close();
}

static Babylon_PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    Babylon_createPbrMaterial_props props = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(props, JsTuple_double_double_double_double_new(v, v, v, 1.0));
    Babylon_createPbrMaterial_props_set_metallicFactor(props, metallic);
    Babylon_createPbrMaterial_props_set_roughnessFactor(props, roughness);
    return babylon_createPbrMaterial(props);
}

static void on_engine(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) {
        set_status_error(value);
        return;
    }
    js_scope_open();
    g_engine = (Babylon_EngineContext){ value.h };
    js_retain(g_engine);

    Babylon_SceneContext scene = babylon_createSceneContext(g_engine);
    GPUColorDict clear = GPUColorDict_create();
    GPUColorDict_set_r(clear, 0.06);
    GPUColorDict_set_g(clear, 0.06);
    GPUColorDict_set_b(clear, 0.06);
    GPUColorDict_set_a(clear, 1.0);
    Babylon_SceneContext_set_clearColor(scene, clear);

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 4.0, PI / 3.1, 12.0, babylon_vec3(0.0, 1.1, 0.0));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.65);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.82, 0.82, 0.82));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.12, 0.12));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.0), 1.8);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(0.95, 0.95, 0.95));
    babylon_addToScene(scene, js_value(dir));

    Babylon_PointLight orb_light = babylon_createPointLight(JsTuple_double_double_double_new(4.5, 3.0, 0.0), 12.0);
    Babylon_PointLight_set_diffuse(orb_light, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    Babylon_PointLight_set_range(orb_light, 18.0);
    babylon_addToScene(scene, js_value(orb_light));
    g_orb_light_pos = Babylon_PointLight_get_position(orb_light);
    js_retain(g_orb_light_pos);

    Babylon_GroundOptions gopts = Babylon_GroundOptions_create();
    Babylon_GroundOptions_set_width(gopts, 14.0);
    Babylon_GroundOptions_set_height(gopts, 14.0);
    Babylon_Mesh floor_m = babylon_createGround(g_engine, gopts);
    Babylon_Mesh_set_material(floor_m, mk_pbr(0.11, 0.08, 0.88));
    babylon_addToScene(scene, js_value(floor_m));

    Babylon_BoxOptions popts = Babylon_BoxOptions_create();
    Babylon_BoxOptions_set_width(popts, 8.4);
    Babylon_BoxOptions_set_height(popts, 0.08);
    Babylon_BoxOptions_set_depth(popts, 8.4);
    Babylon_Mesh plinth = babylon_createBox_with_BoxOptions(g_engine, popts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(plinth), 0.0, 0.04, 0.0);
    Babylon_Mesh_set_material(plinth, mk_pbr(0.17, 0.12, 0.78));
    babylon_addToScene(scene, js_value(plinth));

    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            double roughness = 0.08 + 0.22 * (double)row;
            double metallic = 0.25 * (double)col;
            Babylon_SphereOptions sopts = Babylon_SphereOptions_create();
            Babylon_SphereOptions_set_diameter(sopts, 1.05);
            Babylon_SphereOptions_set_segments(sopts, 24.0);
            Babylon_Mesh sphere = babylon_createSphere(g_engine, sopts);
            Babylon_ObservableVec3_set(Babylon_Mesh_get_position(sphere), (double)(col - 2) * 1.55, 0.75, (double)(row - 2) * 1.55);
            Babylon_Mesh_set_material(sphere, mk_pbr(0.85, metallic, roughness));
            babylon_addToScene(scene, js_value(sphere));
        }
    }

    Babylon_createPbrMaterial_props oprops = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(oprops, JsTuple_double_double_double_double_new(0.95, 0.95, 0.95, 1.0));
    Babylon_createPbrMaterial_props_set_metallicFactor(oprops, 0.0);
    Babylon_createPbrMaterial_props_set_roughnessFactor(oprops, 0.2);
    babylon_setPbrEmissive(oprops, JsTuple_double_double_double_new(0.95, 0.95, 0.95));

    Babylon_SphereOptions mopts = Babylon_SphereOptions_create();
    Babylon_SphereOptions_set_diameter(mopts, 0.36);
    Babylon_SphereOptions_set_segments(mopts, 16.0);
    Babylon_Mesh orb = babylon_createSphere(g_engine, mopts);
    g_orb_pos = Babylon_Mesh_get_position(orb);
    js_retain(g_orb_pos);
    Babylon_ObservableVec3_set(g_orb_pos, 4.5, 3.0, 0.0);
    Babylon_Mesh_set_material(orb, babylon_createPbrMaterial(oprops));
    babylon_addToScene(scene, js_value(orb));

    babylon_onBeforeRender(scene, Babylon_number_fn_from(on_before_render, 0));
    Promise_then(babylon_registerScene(scene), on_registered, 0);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_canvas = Document_querySelector_canvas(get_document());
    if (g_canvas.h != 0) {
        js_retain(g_canvas);
        Promise_then(babylon_createEngine_with_EngineOptions(Babylon_RenderCanvas_of(g_canvas)), on_engine, 0);
    }
    js_scope_close();
}
