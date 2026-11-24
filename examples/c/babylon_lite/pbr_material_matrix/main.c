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
    Babylon_SceneContext_set_clearColor(scene, GPUColorDict_create(.r = 0.06, .g = 0.06, .b = 0.06, .a = 1.0));

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 4.0, PI / 3.1, 12.0, babylon_vec3(0.0, 1.1, 0.0));
    babylon_addToScene(scene, AS_Babylon_Camera(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.65);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.82, 0.82, 0.82));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.12, 0.12));
    babylon_addToScene(scene, AS_Babylon_LightBase(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.0), 1.8);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(0.95, 0.95, 0.95));
    babylon_addToScene(scene, AS_Babylon_LightBase(dir));

    Babylon_PointLight orb_light = babylon_createPointLight(JsTuple_double_double_double_new(4.5, 3.0, 0.0), 12.0);
    Babylon_PointLight_set_diffuse(orb_light, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    Babylon_PointLight_set_range(orb_light, 18.0);
    babylon_addToScene(scene, AS_Babylon_LightBase(orb_light));
    g_orb_light_pos = Babylon_PointLight_get_position(orb_light);
    js_retain(g_orb_light_pos);

    Babylon_Mesh floor_m = babylon_createGround(g_engine, Babylon_GroundOptions_create(.width = 14.0, .height = 14.0));
    Babylon_Mesh_set_material(floor_m, mk_pbr(0.11, 0.08, 0.88));
    babylon_addToScene(scene, floor_m);

    Babylon_BoxOptions popts = Babylon_BoxOptions_create(.width = 8.4, .height = 0.08, .depth = 8.4);
    Babylon_Mesh plinth = babylon_createBox(g_engine, popts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(plinth), 0.0, 0.04, 0.0);
    Babylon_Mesh_set_material(plinth, mk_pbr(0.17, 0.12, 0.78));
    babylon_addToScene(scene, plinth);

    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            double roughness = 0.08 + 0.22 * (double)row;
            double metallic = 0.25 * (double)col;
            Babylon_Mesh sphere = babylon_createSphere(g_engine, Babylon_SphereOptions_create(.diameter = 1.05, .segments = 24.0));
            Babylon_ObservableVec3_set(Babylon_Mesh_get_position(sphere), (double)(col - 2) * 1.55, 0.75, (double)(row - 2) * 1.55);
            Babylon_Mesh_set_material(sphere, mk_pbr(0.85, metallic, roughness));
            babylon_addToScene(scene, sphere);
        }
    }

    Babylon_createPbrMaterial_props oprops = Babylon_createPbrMaterial_props_create(
        .baseColorFactor = JsTuple_double_double_double_double_new(0.95, 0.95, 0.95, 1.0), .metallicFactor = 0.0,
        .roughnessFactor = 0.2
    );
    babylon_setPbrEmissive(oprops, JsTuple_double_double_double_new(0.95, 0.95, 0.95));

    Babylon_Mesh orb = babylon_createSphere(g_engine, Babylon_SphereOptions_create(.diameter = 0.36, .segments = 16.0));
    g_orb_pos = Babylon_Mesh_get_position(orb);
    js_retain(g_orb_pos);
    Babylon_ObservableVec3_set(g_orb_pos, 4.5, 3.0, 0.0);
    Babylon_Mesh_set_material(orb, babylon_createPbrMaterial(oprops));
    babylon_addToScene(scene, orb);

    babylon_onBeforeRender(scene, Babylon_number_fn_from(on_before_render, 0));
    Promise_then(babylon_registerScene(scene), on_registered, 0);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_canvas = Document_querySelector_canvas(get_document());
    if (g_canvas.h != 0) {
        js_retain(g_canvas);
        Promise_then(babylon_createEngine(g_canvas), on_engine, 0);
    }
}
