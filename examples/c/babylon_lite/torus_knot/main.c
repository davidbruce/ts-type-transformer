#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static Babylon_EngineContext g_engine;
static Babylon_EulerProxy g_knot_rot;
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
    Babylon_EulerProxy_set_y(g_knot_rot, g_elapsed * 0.55);
    Babylon_EulerProxy_set_x(g_knot_rot, sin(g_elapsed * 0.4) * 0.25);
    double x = cos(g_elapsed * 0.9) * 2.8;
    double y = 2.0 + sin(g_elapsed * 1.8) * 0.45;
    double z = sin(g_elapsed * 0.9) * 2.8;
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
    GPUColorDict_set_r(clear, 0.05);
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.11);
    GPUColorDict_set_a(clear, 1.0);
    Babylon_SceneContext_set_clearColor(scene, clear);

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 4.0, PI / 3.0, 8.5, babylon_vec3(0.0, 1.6, 0.0));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.75);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.75, 0.85, 1.0));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.14, 0.22));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.2), 2.2);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.96, 0.9));
    babylon_addToScene(scene, js_value(dir));

    Babylon_PointLight orb_light = babylon_createPointLight(JsTuple_double_double_double_new(2.8, 1.6, 0.0), 8.0);
    Babylon_PointLight_set_diffuse(orb_light, JsTuple_double_double_double_new(1.0, 0.45, 0.35));
    Babylon_PointLight_set_range(orb_light, 12.0);
    babylon_addToScene(scene, js_value(orb_light));
    g_orb_light_pos = Babylon_PointLight_get_position(orb_light);
    js_retain(g_orb_light_pos);

    Babylon_createPbrMaterial_props kprops = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(kprops, JsTuple_double_double_double_double_new(0.18, 0.78, 0.95, 1.0));
    Babylon_createPbrMaterial_props_set_metallicFactor(kprops, 0.75);
    Babylon_createPbrMaterial_props_set_roughnessFactor(kprops, 0.22);

    Babylon_TorusKnotOptions kopts = Babylon_TorusKnotOptions_create();
    Babylon_TorusKnotOptions_set_radius(kopts, 1.15);
    Babylon_TorusKnotOptions_set_tube(kopts, 0.34);
    Babylon_TorusKnotOptions_set_radialSegments(kopts, 48.0);
    Babylon_TorusKnotOptions_set_tubularSegments(kopts, 160.0);
    Babylon_Mesh knot = babylon_createTorusKnot(g_engine, kopts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(knot), 0.0, 2.0, 0.0);
    Babylon_Mesh_set_material(knot, babylon_createPbrMaterial(kprops));
    babylon_addToScene(scene, js_value(knot));
    g_knot_rot = Babylon_Mesh_get_rotation(knot);
    js_retain(g_knot_rot);

    Babylon_createPbrMaterial_props oprops = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(oprops, JsTuple_double_double_double_double_new(1.0, 0.42, 0.32, 1.0));
    Babylon_createPbrMaterial_props_set_metallicFactor(oprops, 0.2);
    Babylon_createPbrMaterial_props_set_roughnessFactor(oprops, 0.35);

    Babylon_SphereOptions sopts = Babylon_SphereOptions_create();
    Babylon_SphereOptions_set_diameter(sopts, 0.55);
    Babylon_SphereOptions_set_segments(sopts, 32.0);
    Babylon_Mesh orb = babylon_createSphere(g_engine, sopts);
    Babylon_Mesh_set_material(orb, babylon_createPbrMaterial(oprops));
    babylon_addToScene(scene, js_value(orb));
    g_orb_pos = Babylon_Mesh_get_position(orb);
    js_retain(g_orb_pos);

    Babylon_GroundOptions gopts = Babylon_GroundOptions_create();
    Babylon_GroundOptions_set_width(gopts, 16.0);
    Babylon_GroundOptions_set_height(gopts, 16.0);
    Babylon_Mesh ground = babylon_createGround(g_engine, gopts);
    Babylon_Mesh_set_material(ground, babylon_createGridMaterial());
    babylon_addToScene(scene, js_value(ground));

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
