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

static double approx_sqrt(double v) {
    if (v <= 0.0) return 0.0;
    double r = v > 1.0 ? v * 0.5 : 1.0;
    for (int i = 0; i < 8; i++) r = 0.5 * (r + v / r);
    return r;
}

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
    for (int idx = 0; idx < 36; idx++) {
        double h = 0.55
            + 1.15 * (0.5 + 0.5 * approx_sin(g_elapsed * 2.2 - g_col_dist[idx] * 1.15))
            + 0.30 * approx_cos(g_elapsed * 1.3 + g_col_diag[idx] * 0.7);
        ObservableVec3_set(g_col_scale[idx], 1.0, h, 1.0);
        ObservableVec3_set(g_col_pos[idx], g_col_x[idx], h * 0.5, g_col_z[idx]);
    }
    for (int k = 0; k < 4; k++) {
        double fk = (double)k;
        double theta = g_elapsed * 0.7 + fk * (PI * 0.5);
        ObservableVec3_set(
            g_cap_pos[k],
            approx_cos(theta) * 4.9,
            2.6 + approx_sin(g_elapsed * 1.6 + fk * 1.57) * 0.55,
            approx_sin(theta) * 4.9
        );
        EulerProxy_set_z(g_cap_rot[k], approx_sin(g_elapsed + fk) * 0.35);
        EulerProxy_set_y(g_cap_rot[k], -theta);
    }
    ObservableVec3_set(g_light1_pos, approx_cos(g_elapsed * 0.65) * 5.2, 4.2, approx_sin(g_elapsed * 0.65) * 5.2);
    ObservableVec3_set(g_light2_pos, approx_cos(-g_elapsed * 0.52) * 5.2, 3.4, approx_sin(-g_elapsed * 0.52) * 5.2);
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

static PbrMaterialProps mk_pbr(double v, double metallic, double roughness) {
    createPbrMaterial_props props = createPbrMaterial_props_create();
    JsTuple_double_double_double_double col = JsTuple_double_double_double_double_new(v, v, v, 1.0);
    createPbrMaterial_props_set_baseColorFactor(props, col);
    createPbrMaterial_props_set_metallicFactor(props, js_some_double(metallic));
    createPbrMaterial_props_set_roughnessFactor(props, js_some_double(roughness));
    PbrMaterialProps mat = createPbrMaterial(props);
    js_release(col);
    js_release(props);
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

    Vec3 cam_target = vec3(0.0, 1.4, 0.0);
    ArcRotateCamera camera = createArcRotateCamera(PI / 4.0, PI / 3.1, 12.8, cam_target);
    addToScene(scene, js_value(camera));
    void_fn ctrl = attachControl(camera, g_canvas, scene);
    js_release(ctrl);
    js_release(camera);
    js_release(cam_target);

    JsTuple_double_double_double hemi_dir = JsTuple_double_double_double_new(0.0, 1.0, 0.15);
    HemisphericLight hemi = createHemisphericLight(hemi_dir, js_some_double(0.55));
    JsTuple_double_double_double hdc = JsTuple_double_double_double_new(0.76, 0.76, 0.76);
    JsTuple_double_double_double hgc = JsTuple_double_double_double_new(0.10, 0.10, 0.10);
    HemisphericLight_set_diffuseColor(hemi, hdc);
    HemisphericLight_set_groundColor(hemi, hgc);
    addToScene(scene, js_value(hemi));
    js_release(hgc); js_release(hdc); js_release(hemi); js_release(hemi_dir);

    JsTuple_double_double_double dir_v = JsTuple_double_double_double_new(-1.0, -2.2, -0.8);
    DirectionalLight dir = createDirectionalLight(dir_v, js_some_double(1.35));
    JsTuple_double_double_double dd = JsTuple_double_double_double_new(0.90, 0.90, 0.90);
    DirectionalLight_set_diffuse(dir, dd);
    addToScene(scene, js_value(dir));
    js_release(dd); js_release(dir); js_release(dir_v);

    JsTuple_double_double_double p1 = JsTuple_double_double_double_new(5.2, 4.2, 0.0);
    PointLight light1 = createPointLight(p1, js_some_double(10.0));
    JsTuple_double_double_double d1 = JsTuple_double_double_double_new(1.0, 1.0, 1.0);
    PointLight_set_diffuse(light1, d1);
    PointLight_set_range(light1, 16.0);
    addToScene(scene, js_value(light1));
    g_light1_pos = PointLight_get_position(light1);
    js_release(d1); js_release(light1); js_release(p1);

    JsTuple_double_double_double p2 = JsTuple_double_double_double_new(-5.2, 3.4, 0.0);
    PointLight light2 = createPointLight(p2, js_some_double(7.5));
    JsTuple_double_double_double d2 = JsTuple_double_double_double_new(0.78, 0.78, 0.78);
    PointLight_set_diffuse(light2, d2);
    PointLight_set_range(light2, 16.0);
    addToScene(scene, js_value(light2));
    g_light2_pos = PointLight_get_position(light2);
    js_release(d2); js_release(light2); js_release(p2);

    PbrMaterialProps floor_mat = mk_pbr(0.09, 0.10, 0.88);
    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, js_some_double(18.0));
    GroundOptions_set_height(gopts, js_some_double(18.0));
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, floor_mat);
    addToScene(scene, js_value(floor_m));
    js_release(floor_m); js_release(gopts); js_release(floor_mat);

    const double shades[4] = { 0.22, 0.44, 0.68, 0.88 };
    PbrMaterialProps col_mats[4];
    for (int i = 0; i < 4; i++) col_mats[i] = mk_pbr(shades[i], 0.35, 0.36);

    BoxOptions bopts = BoxOptions_create();
    BoxOptions_set_width(bopts, js_some_double(0.72));
    BoxOptions_set_height(bopts, js_some_double(1.0));
    BoxOptions_set_depth(bopts, js_some_double(0.72));

    for (int row = 0; row < 6; row++) {
        for (int col = 0; col < 6; col++) {
            int idx = row * 6 + col;
            double fx = (double)col - 2.5;
            double fz = (double)row - 2.5;
            double x = fx * 1.15;
            double z = fz * 1.15;
            g_col_x[idx] = x;
            g_col_z[idx] = z;
            g_col_dist[idx] = approx_sqrt(fx * fx + fz * fz);
            g_col_diag[idx] = fx - fz;

            Mesh m = createBox_with_BoxOptions(g_engine, bopts);
            g_col_pos[idx] = SceneNode_get_position(m);
            g_col_scale[idx] = SceneNode_get_scaling(m);
            ObservableVec3_set(g_col_pos[idx], x, 0.5, z);
            Mesh_set_material(m, col_mats[(row + col) % 4]);
            addToScene(scene, js_value(m));
            js_release(m);
        }
    }
    js_release(bopts);
    for (int i = 0; i < 4; i++) js_release(col_mats[i]);

    createPbrMaterial_props cprops = createPbrMaterial_props_create();
    JsTuple_double_double_double_double cbcf = JsTuple_double_double_double_double_new(0.92, 0.92, 0.92, 1.0);
    JsTuple_double_double_double cem = JsTuple_double_double_double_new(0.16, 0.16, 0.16);
    createPbrMaterial_props_set_baseColorFactor(cprops, cbcf);
    createPbrMaterial_props_set_metallicFactor(cprops, js_some_double(0.50));
    createPbrMaterial_props_set_roughnessFactor(cprops, js_some_double(0.15));
    ClearCoatProps cc = ClearCoatProps_create();
    ClearCoatProps_set_isEnabled(cc, js_some_bool(true));
    ClearCoatProps_set_intensity(cc, js_some_double(1.0));
    ClearCoatProps_set_roughness(cc, js_some_double(0.08));
    setPbrClearCoat(cprops, cc);
    setPbrEmissive(cprops, cem);
    PbrMaterialProps cap_mat = createPbrMaterial(cprops);
    js_release(cc); js_release(cem); js_release(cbcf); js_release(cprops);

    CapsuleOptions copts = CapsuleOptions_create();
    CapsuleOptions_set_height(copts, js_some_double(1.15));
    CapsuleOptions_set_radius(copts, js_some_double(0.22));
    CapsuleOptions_set_tessellation(copts, js_some_double(20.0));
    for (int k = 0; k < 4; k++) {
        Mesh cap = createCapsule(g_engine, copts);
        g_cap_pos[k] = SceneNode_get_position(cap);
        g_cap_rot[k] = SceneNode_get_rotation(cap);
        Mesh_set_material(cap, cap_mat);
        addToScene(scene, js_value(cap));
        js_release(cap);
    }
    js_release(copts);
    js_release(cap_mat);

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
