#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static EulerProxy g_knot_rot;
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
    EulerProxy_set_y(g_knot_rot, g_elapsed * 0.55);
    EulerProxy_set_x(g_knot_rot, sin(g_elapsed * 0.4) * 0.25);
    double x = cos(g_elapsed * 0.9) * 2.8;
    double y = 2.0 + sin(g_elapsed * 1.8) * 0.45;
    double z = sin(g_elapsed * 0.9) * 2.8;
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
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.11);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.0, 8.5, vec3(0.0, 1.6, 0.0));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.75);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.75, 0.85, 1.0));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.14, 0.22));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.2), 2.2);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.96, 0.9));
    addToScene(scene, js_value(dir));

    PointLight orb_light = createPointLight(JsTuple_double_double_double_new(2.8, 1.6, 0.0), 8.0);
    PointLight_set_diffuse(orb_light, JsTuple_double_double_double_new(1.0, 0.45, 0.35));
    PointLight_set_range(orb_light, 12.0);
    addToScene(scene, js_value(orb_light));
    g_orb_light_pos = PointLight_get_position(orb_light);
    js_retain(g_orb_light_pos);

    createPbrMaterial_props kprops = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(kprops, JsTuple_double_double_double_double_new(0.18, 0.78, 0.95, 1.0));
    createPbrMaterial_props_set_metallicFactor(kprops, 0.75);
    createPbrMaterial_props_set_roughnessFactor(kprops, 0.22);

    TorusKnotOptions kopts = TorusKnotOptions_create();
    TorusKnotOptions_set_radius(kopts, 1.15);
    TorusKnotOptions_set_tube(kopts, 0.34);
    TorusKnotOptions_set_radialSegments(kopts, 48.0);
    TorusKnotOptions_set_tubularSegments(kopts, 160.0);
    Mesh knot = createTorusKnot(g_engine, kopts);
    ObservableVec3_set(Mesh_get_position(knot), 0.0, 2.0, 0.0);
    Mesh_set_material(knot, createPbrMaterial(kprops));
    addToScene(scene, js_value(knot));
    g_knot_rot = Mesh_get_rotation(knot);
    js_retain(g_knot_rot);

    createPbrMaterial_props oprops = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(oprops, JsTuple_double_double_double_double_new(1.0, 0.42, 0.32, 1.0));
    createPbrMaterial_props_set_metallicFactor(oprops, 0.2);
    createPbrMaterial_props_set_roughnessFactor(oprops, 0.35);

    SphereOptions sopts = SphereOptions_create();
    SphereOptions_set_diameter(sopts, 0.55);
    SphereOptions_set_segments(sopts, 32.0);
    Mesh orb = createSphere(g_engine, sopts);
    Mesh_set_material(orb, createPbrMaterial(oprops));
    addToScene(scene, js_value(orb));
    g_orb_pos = Mesh_get_position(orb);
    js_retain(g_orb_pos);

    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, 16.0);
    GroundOptions_set_height(gopts, 16.0);
    Mesh ground = createGround(g_engine, gopts);
    Mesh_set_material(ground, createGridMaterial());
    addToScene(scene, js_value(ground));

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
