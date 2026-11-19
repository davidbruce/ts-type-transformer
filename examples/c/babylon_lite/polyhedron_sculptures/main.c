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

static double approx_fabs(double x) {
    return x < 0.0 ? -x : x;
}

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static ObservableVec3 g_spot_pos;
static ObservableVec3 g_spot_dir;
static ObservableVec3 g_poly_pos[5];
static EulerProxy g_poly_rot[5];
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
    double sx = approx_sin(g_elapsed * 0.65) * 5.2;
    ObservableVec3_set(g_spot_pos, sx, 6.2, 3.0);
    ObservableVec3_set(g_spot_dir, -sx * 0.15, -1.0, -0.55);
    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        ObservableVec3_set_y(g_poly_pos[i], 2.05 + approx_sin(g_elapsed * 1.3 + fi * 1.1) * 0.18);
        EulerProxy_set_y(g_poly_rot[i], g_elapsed * (0.45 + 0.12 * fi));
        EulerProxy_set_x(g_poly_rot[i], approx_sin(g_elapsed * 0.7 + fi) * 0.35);
    }
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

static StandardMaterialProps mk_std(double diff, double spec, double power) {
    StandardMaterialProps mat = createStandardMaterial();
    JsTuple_double_double_double dc = JsTuple_double_double_double_new(diff, diff, diff);
    JsTuple_double_double_double sc = JsTuple_double_double_double_new(spec, spec, spec);
    StandardMaterialProps_set_diffuseColor(mat, dc);
    StandardMaterialProps_set_specularColor(mat, sc);
    StandardMaterialProps_set_specularPower(mat, power);
    js_release(sc);
    js_release(dc);
    return mat;
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
    GPUColorDict_set_g(clear, 0.05);
    GPUColorDict_set_b(clear, 0.05);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);
    js_release(clear);

    Vec3 cam_target = vec3(0.0, 1.6, 0.0);
    ArcRotateCamera camera = createArcRotateCamera(PI / 2.0, PI / 2.8, 11.5, cam_target);
    addToScene(scene, js_value(camera));
    void_fn ctrl = attachControl(camera, g_canvas, scene);
    js_release(ctrl);
    js_release(camera);
    js_release(cam_target);

    JsTuple_double_double_double hemi_dir = JsTuple_double_double_double_new(0.0, 1.0, 0.0);
    HemisphericLight hemi = createHemisphericLight(hemi_dir, js_some_double(0.42));
    JsTuple_double_double_double hdc = JsTuple_double_double_double_new(0.70, 0.70, 0.70);
    JsTuple_double_double_double hgc = JsTuple_double_double_double_new(0.08, 0.08, 0.08);
    HemisphericLight_set_diffuseColor(hemi, hdc);
    HemisphericLight_set_groundColor(hemi, hgc);
    addToScene(scene, js_value(hemi));
    js_release(hgc); js_release(hdc); js_release(hemi); js_release(hemi_dir);

    JsTuple_double_double_double spot_p = JsTuple_double_double_double_new(0.0, 6.2, 3.0);
    JsTuple_double_double_double spot_d = JsTuple_double_double_double_new(0.0, -1.0, -0.55);
    SpotLight spot = createSpotLight(spot_p, spot_d, PI / 3.2, 3.0, js_some_double(26.0));
    JsTuple_double_double_double sd = JsTuple_double_double_double_new(1.0, 1.0, 1.0);
    SpotLight_set_diffuse(spot, sd);
    SpotLight_set_range(spot, 22.0);
    addToScene(scene, js_value(spot));
    g_spot_pos = SpotLight_get_position(spot);
    g_spot_dir = SpotLight_get_direction(spot);
    js_release(sd); js_release(spot); js_release(spot_d); js_release(spot_p);

    JsTuple_double_double_double fill_v = JsTuple_double_double_double_new(1.2, -1.5, -1.0);
    DirectionalLight fill = createDirectionalLight(fill_v, js_some_double(0.65));
    JsTuple_double_double_double fd = JsTuple_double_double_double_new(0.80, 0.80, 0.80);
    DirectionalLight_set_diffuse(fill, fd);
    addToScene(scene, js_value(fill));
    js_release(fd); js_release(fill); js_release(fill_v);

    StandardMaterialProps floor_mat = mk_std(0.10, 0.25, 24.0);
    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, js_some_double(18.0));
    GroundOptions_set_height(gopts, js_some_double(18.0));
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, floor_mat);
    addToScene(scene, js_value(floor_m));
    js_release(floor_m); js_release(gopts); js_release(floor_mat);

    StandardMaterialProps plinth_mat = mk_std(0.18, 0.30, 32.0);

    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        double x = (fi - 2.0) * 2.25;
        double z = -approx_fabs(fi - 2.0) * 0.55;

        CylinderOptions copts = CylinderOptions_create();
        CylinderOptions_set_height(copts, js_some_double(1.1));
        CylinderOptions_set_diameter(copts, js_some_double(1.15));
        CylinderOptions_set_tessellation(copts, js_some_double(28.0));
        Mesh plinth = createCylinder(g_engine, copts);
        ObservableVec3 ppos = SceneNode_get_position(plinth);
        ObservableVec3_set(ppos, x, 0.55, z);
        Mesh_set_material(plinth, plinth_mat);
        addToScene(scene, js_value(plinth));
        js_release(ppos); js_release(plinth); js_release(copts);

        double v = 0.90 - fi * 0.13;
        StandardMaterialProps poly_mat = mk_std(v, 0.95, 18.0 + fi * 24.0);
        PolyhedronOptions popts = PolyhedronOptions_create();
        PolyhedronOptions_set_type(popts, js_some_double(fi));
        PolyhedronOptions_set_size(popts, js_some_double(0.62));
        PolyhedronOptions_set_flat(popts, js_some_bool(true));
        Mesh mesh = createPolyhedron(g_engine, popts);
        g_poly_pos[i] = SceneNode_get_position(mesh);
        ObservableVec3_set(g_poly_pos[i], x, 2.05, z);
        g_poly_rot[i] = SceneNode_get_rotation(mesh);
        Mesh_set_material(mesh, poly_mat);
        addToScene(scene, js_value(mesh));
        js_release(mesh); js_release(popts); js_release(poly_mat);
    }
    js_release(plinth_mat);

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
