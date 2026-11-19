#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"

#define PI 3.14159265358979323846
#define TWO_PI (2.0 * PI)

static double approx_sin(double x) {
    while (x > PI) x -= TWO_PI;
    while (x < -PI) x += TWO_PI;
    double x2 = x * x;
    return x * (1.0 - x2 / 6.0 * (1.0 - x2 / 20.0 * (1.0 - x2 / 42.0)));
}

static double approx_cos(double x) {
    return approx_sin(x + PI * 0.5);
}

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static EulerProxy g_knot_rot;
static ObservableVec3 g_orb_pos;
static ObservableVec3 g_orb_light_pos;
static double g_elapsed = 0.0;

static void set_status_error(JsValue err) {
    Document doc = get_document();
    HTMLElement status = Document_getElementById(doc, "status");
    if (status.h != 0) {
        const char prefix[] = "WebGPU / Babylon Lite error: ";
        char buf[320];
        uint32_t plen = (uint32_t)(sizeof(prefix) - 1);
        for (uint32_t i = 0; i < plen; i++) buf[i] = prefix[i];
        JsString msg = js_error_message(err);
        JsString_copy(msg, buf + plen, (uint32_t)(sizeof(buf) - plen));
        Node_set_textContent(status, buf);
        js_release(msg);
        js_release(status);
    }
    js_release(doc);
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    g_elapsed += delta_ms * 0.001;
    EulerProxy_set_y(g_knot_rot, g_elapsed * 0.55);
    EulerProxy_set_x(g_knot_rot, approx_sin(g_elapsed * 0.4) * 0.25);
    double x = approx_cos(g_elapsed * 0.9) * 2.8;
    double y = 2.0 + approx_sin(g_elapsed * 1.8) * 0.45;
    double z = approx_sin(g_elapsed * 0.9) * 2.8;
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
    Promise p = startEngine(g_engine);
    Promise_then(p, on_started, 0);
    js_release(p);
}

static void on_engine(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    if (!ok) {
        set_status_error(value);
        return;
    }
    g_engine = (EngineContext){ value.h };
    js_retain(g_engine);

    SceneContext scene = createSceneContext(g_engine);
    GPUColorDict clear = GPUColorDict_create();
    GPUColorDict_set_r(clear, 0.05);
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.11);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);
    js_release(clear);

    Vec3 cam_target = vec3(0.0, 1.6, 0.0);
    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.0, 8.5, cam_target);
    addToScene(scene, js_value(camera));
    void_fn ctrl = attachControl(camera, g_canvas, scene);
    js_release(ctrl);
    js_release(camera);
    js_release(cam_target);

    JsTuple_double_double_double hemi_dir = JsTuple_double_double_double_new(0.0, 1.0, 0.2);
    HemisphericLight hemi = createHemisphericLight(hemi_dir, js_some_double(0.75));
    JsTuple_double_double_double hdc = JsTuple_double_double_double_new(0.75, 0.85, 1.0);
    JsTuple_double_double_double hgc = JsTuple_double_double_double_new(0.12, 0.14, 0.22);
    HemisphericLight_set_diffuseColor(hemi, hdc);
    HemisphericLight_set_groundColor(hemi, hgc);
    addToScene(scene, js_value(hemi));
    js_release(hgc); js_release(hdc); js_release(hemi); js_release(hemi_dir);

    JsTuple_double_double_double dir_v = JsTuple_double_double_double_new(-1.0, -2.0, -1.2);
    DirectionalLight dir = createDirectionalLight(dir_v, js_some_double(2.2));
    JsTuple_double_double_double dd = JsTuple_double_double_double_new(1.0, 0.96, 0.9);
    DirectionalLight_set_diffuse(dir, dd);
    addToScene(scene, js_value(dir));
    js_release(dd); js_release(dir); js_release(dir_v);

    JsTuple_double_double_double orb_lpos = JsTuple_double_double_double_new(2.8, 1.6, 0.0);
    PointLight orb_light = createPointLight(orb_lpos, js_some_double(8.0));
    JsTuple_double_double_double old = JsTuple_double_double_double_new(1.0, 0.45, 0.35);
    PointLight_set_diffuse(orb_light, old);
    PointLight_set_range(orb_light, 12.0);
    addToScene(scene, js_value(orb_light));
    g_orb_light_pos = PointLight_get_position(orb_light);
    js_release(old); js_release(orb_light); js_release(orb_lpos);

    createPbrMaterial_props kprops = createPbrMaterial_props_create();
    JsTuple_double_double_double_double kbcf = JsTuple_double_double_double_double_new(0.18, 0.78, 0.95, 1.0);
    createPbrMaterial_props_set_baseColorFactor(kprops, kbcf);
    createPbrMaterial_props_set_metallicFactor(kprops, js_some_double(0.75));
    createPbrMaterial_props_set_roughnessFactor(kprops, js_some_double(0.22));
    PbrMaterialProps knot_mat = createPbrMaterial(kprops);
    js_release(kbcf); js_release(kprops);

    TorusKnotOptions kopts = TorusKnotOptions_create();
    TorusKnotOptions_set_radius(kopts, js_some_double(1.15));
    TorusKnotOptions_set_tube(kopts, js_some_double(0.34));
    TorusKnotOptions_set_radialSegments(kopts, js_some_double(48.0));
    TorusKnotOptions_set_tubularSegments(kopts, js_some_double(160.0));
    Mesh knot = createTorusKnot(g_engine, kopts);
    ObservableVec3 knot_pos = SceneNode_get_position(knot);
    ObservableVec3_set(knot_pos, 0.0, 2.0, 0.0);
    Mesh_set_material(knot, knot_mat);
    addToScene(scene, js_value(knot));
    g_knot_rot = SceneNode_get_rotation(knot);
    js_release(knot_pos); js_release(knot); js_release(kopts); js_release(knot_mat);

    createPbrMaterial_props oprops = createPbrMaterial_props_create();
    JsTuple_double_double_double_double obcf = JsTuple_double_double_double_double_new(1.0, 0.42, 0.32, 1.0);
    createPbrMaterial_props_set_baseColorFactor(oprops, obcf);
    createPbrMaterial_props_set_metallicFactor(oprops, js_some_double(0.2));
    createPbrMaterial_props_set_roughnessFactor(oprops, js_some_double(0.35));
    PbrMaterialProps orb_mat = createPbrMaterial(oprops);
    js_release(obcf); js_release(oprops);

    SphereOptions sopts = SphereOptions_create();
    SphereOptions_set_diameter(sopts, js_some_double(0.55));
    SphereOptions_set_segments(sopts, js_some_double(32.0));
    Mesh orb = createSphere(g_engine, sopts);
    Mesh_set_material(orb, orb_mat);
    addToScene(scene, js_value(orb));
    g_orb_pos = SceneNode_get_position(orb);
    js_release(orb); js_release(sopts); js_release(orb_mat);

    ShaderMaterial ground_mat = createGridMaterial((GridMaterialOptions){ 0 });
    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, js_some_double(16.0));
    GroundOptions_set_height(gopts, js_some_double(16.0));
    Mesh ground = createGround(g_engine, gopts);
    Mesh_set_material(ground, ground_mat);
    addToScene(scene, js_value(ground));
    js_release(ground); js_release(gopts); js_release(ground_mat);

    number_fn render_cb = number_fn_from(on_before_render, 0);
    onBeforeRender(scene, render_cb);
    js_release(render_cb);

    Promise reg = registerScene(scene);
    Promise_then(reg, on_registered, 0);
    js_release(reg);
    js_release(scene);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    g_canvas = ParentNode_querySelector_canvas(doc);
    js_release(doc);
    if (g_canvas.h == 0) return;

    Promise p = createEngine_with_EngineOptions(RenderCanvas_of(g_canvas));
    Promise_then(p, on_engine, 0);
    js_release(p);
}
