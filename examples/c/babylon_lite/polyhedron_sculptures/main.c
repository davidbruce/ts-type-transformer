#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static Babylon_EngineContext g_engine;
static Babylon_ObservableVec3 g_spot_pos;
static Babylon_ObservableVec3 g_spot_dir;
static Babylon_ObservableVec3 g_poly_pos[5];
static Babylon_EulerProxy g_poly_rot[5];
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
    double sx = sin(g_elapsed * 0.65) * 5.2;
    Babylon_ObservableVec3_set(g_spot_pos, sx, 6.2, 3.0);
    Babylon_ObservableVec3_set(g_spot_dir, -sx * 0.15, -1.0, -0.55);
    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        Babylon_ObservableVec3_set_y(g_poly_pos[i], 2.05 + sin(g_elapsed * 1.3 + fi * 1.1) * 0.18);
        Babylon_EulerProxy_set_y(g_poly_rot[i], g_elapsed * (0.45 + 0.12 * fi));
        Babylon_EulerProxy_set_x(g_poly_rot[i], sin(g_elapsed * 0.7 + fi) * 0.35);
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
    js_scope();
    Promise_then(babylon_startEngine(g_engine), on_started, 0);
}

static Babylon_StandardMaterialProps mk_std(double diff, double spec, double power) {
    Babylon_StandardMaterialProps mat = babylon_createStandardMaterial();
    Babylon_StandardMaterialProps_set_diffuseColor(mat, JsTuple_double_double_double_new(diff, diff, diff));
    Babylon_StandardMaterialProps_set_specularColor(mat, JsTuple_double_double_double_new(spec, spec, spec));
    Babylon_StandardMaterialProps_set_specularPower(mat, power);
    return mat;
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

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 2.0, PI / 2.8, 11.5, babylon_vec3(0.0, 1.6, 0.0));
    babylon_addToScene(scene, camera);
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.0), 0.42);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.70, 0.70, 0.70));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.08, 0.08, 0.08));
    babylon_addToScene(scene, hemi);

    Babylon_SpotLight spot = babylon_createSpotLight(
        JsTuple_double_double_double_new(0.0, 6.2, 3.0),
        JsTuple_double_double_double_new(0.0, -1.0, -0.55),
        PI / 3.2, 3.0, 26.0);
    Babylon_SpotLight_set_diffuse(spot, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    Babylon_SpotLight_set_range(spot, 22.0);
    babylon_addToScene(scene, spot);
    g_spot_pos = Babylon_SpotLight_get_position(spot);
    g_spot_dir = Babylon_SpotLight_get_direction(spot);
    js_retain(g_spot_pos);
    js_retain(g_spot_dir);

    Babylon_DirectionalLight fill = babylon_createDirectionalLight(JsTuple_double_double_double_new(1.2, -1.5, -1.0), 0.65);
    Babylon_DirectionalLight_set_diffuse(fill, JsTuple_double_double_double_new(0.80, 0.80, 0.80));
    babylon_addToScene(scene, fill);

    Babylon_Mesh floor_m = babylon_createGround(g_engine, Babylon_GroundOptions_create(.width = 18.0, .height = 18.0));
    Babylon_Mesh_set_material(floor_m, mk_std(0.10, 0.25, 24.0));
    babylon_addToScene(scene, floor_m);

    Babylon_StandardMaterialProps plinth_mat = mk_std(0.18, 0.30, 32.0);

    for (int i = 0; i < 5; i++) {
        double fi = (double)i;
        double x = (fi - 2.0) * 2.25;
        double z = -fabs(fi - 2.0) * 0.55;

        Babylon_CylinderOptions copts = Babylon_CylinderOptions_create(.height = 1.1, .diameter = 1.15, .tessellation = 28.0);
        Babylon_Mesh plinth = babylon_createCylinder(g_engine, copts);
        Babylon_ObservableVec3_set(Babylon_Mesh_get_position(plinth), x, 0.55, z);
        Babylon_Mesh_set_material(plinth, plinth_mat);
        babylon_addToScene(scene, plinth);

        double v = 0.90 - fi * 0.13;
        Babylon_Mesh mesh = babylon_createPolyhedron(g_engine, Babylon_PolyhedronOptions_create(.type = fi, .size = 0.62, .flat = true));
        g_poly_pos[i] = Babylon_Mesh_get_position(mesh);
        g_poly_rot[i] = Babylon_Mesh_get_rotation(mesh);
        js_retain(g_poly_pos[i]);
        js_retain(g_poly_rot[i]);
        Babylon_ObservableVec3_set(g_poly_pos[i], x, 2.05, z);
        Babylon_Mesh_set_material(mesh, mk_std(v, 0.95, 18.0 + fi * 24.0));
        babylon_addToScene(scene, mesh);
    }

    babylon_onBeforeRender(scene, on_before_render);
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
