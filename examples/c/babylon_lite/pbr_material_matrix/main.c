#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static ObservableVec3 g_orb_pos;
static ObservableVec3 g_orb_light_pos;
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
    ObservableVec3_set(g_orb_pos, x, y, z);
    ObservableVec3_set(g_orb_light_pos, x, y, z);
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
    GPUColorDict_set_r(clear, 0.06);
    GPUColorDict_set_g(clear, 0.06);
    GPUColorDict_set_b(clear, 0.06);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.1, 12.0, vec3(0.0, 1.1, 0.0));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.65);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.82, 0.82, 0.82));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.12, 0.12));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.0), 1.8);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(0.95, 0.95, 0.95));
    addToScene(scene, js_value(dir));

    PointLight orb_light = createPointLight(JsTuple_double_double_double_new(4.5, 3.0, 0.0), 12.0);
    PointLight_set_diffuse(orb_light, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    PointLight_set_range(orb_light, 18.0);
    addToScene(scene, js_value(orb_light));
    g_orb_light_pos = PointLight_get_position(orb_light);
    js_retain(g_orb_light_pos);

    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, 14.0);
    GroundOptions_set_height(gopts, 14.0);
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, mk_pbr(0.11, 0.08, 0.88));
    addToScene(scene, js_value(floor_m));

    BoxOptions popts = BoxOptions_create();
    BoxOptions_set_width(popts, 8.4);
    BoxOptions_set_height(popts, 0.08);
    BoxOptions_set_depth(popts, 8.4);
    Mesh plinth = createBox_with_BoxOptions(g_engine, popts);
    ObservableVec3_set(Mesh_get_position(plinth), 0.0, 0.04, 0.0);
    Mesh_set_material(plinth, mk_pbr(0.17, 0.12, 0.78));
    addToScene(scene, js_value(plinth));

    for (int row = 0; row < 5; row++) {
        for (int col = 0; col < 5; col++) {
            double roughness = 0.08 + 0.22 * (double)row;
            double metallic = 0.25 * (double)col;
            SphereOptions sopts = SphereOptions_create();
            SphereOptions_set_diameter(sopts, 1.05);
            SphereOptions_set_segments(sopts, 24.0);
            Mesh sphere = createSphere(g_engine, sopts);
            ObservableVec3_set(Mesh_get_position(sphere), (double)(col - 2) * 1.55, 0.75, (double)(row - 2) * 1.55);
            Mesh_set_material(sphere, mk_pbr(0.85, metallic, roughness));
            addToScene(scene, js_value(sphere));
        }
    }

    createPbrMaterial_props oprops = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(oprops, JsTuple_double_double_double_double_new(0.95, 0.95, 0.95, 1.0));
    createPbrMaterial_props_set_metallicFactor(oprops, 0.0);
    createPbrMaterial_props_set_roughnessFactor(oprops, 0.2);
    setPbrEmissive(oprops, JsTuple_double_double_double_new(0.95, 0.95, 0.95));

    SphereOptions mopts = SphereOptions_create();
    SphereOptions_set_diameter(mopts, 0.36);
    SphereOptions_set_segments(mopts, 16.0);
    Mesh orb = createSphere(g_engine, mopts);
    g_orb_pos = Mesh_get_position(orb);
    js_retain(g_orb_pos);
    ObservableVec3_set(g_orb_pos, 4.5, 3.0, 0.0);
    Mesh_set_material(orb, createPbrMaterial(oprops));
    addToScene(scene, js_value(orb));

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
