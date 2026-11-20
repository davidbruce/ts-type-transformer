#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static EulerProxy g_mono_rot;
static EulerProxy g_core_rot;
static EulerProxy g_outer_rot;
static EulerProxy g_inner_rot;
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
    EulerProxy_set_y(g_mono_rot, g_elapsed * 0.35);
    EulerProxy_set_x(g_mono_rot, sin(g_elapsed * 0.3) * 0.20);
    EulerProxy_set_y(g_core_rot, -g_elapsed * 0.90);
    EulerProxy_set_z(g_core_rot, g_elapsed * 0.50);
    EulerProxy_set_x(g_outer_rot, g_elapsed * 0.42);
    EulerProxy_set_y(g_outer_rot, g_elapsed * 0.25);
    EulerProxy_set_z(g_inner_rot, -g_elapsed * 0.58);
    EulerProxy_set_x(g_inner_rot, g_elapsed * 0.31);
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

    ArcRotateCamera camera = createArcRotateCamera(PI / 4.2, PI / 2.9, 8.8, vec3(0.0, 2.1, 0.0));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.85, 0.85));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.14, 0.14, 0.14));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.2, -2.0, -1.1), 2.1);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    addToScene(scene, js_value(dir));

    PointLight key_light = createPointLight(JsTuple_double_double_double_new(3.2, 3.6, 2.4), 8.0);
    PointLight_set_diffuse(key_light, JsTuple_double_double_double_new(0.92, 0.92, 0.92));
    PointLight_set_range(key_light, 14.0);
    addToScene(scene, js_value(key_light));

    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, 16.0);
    GroundOptions_set_height(gopts, 16.0);
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, mk_pbr(0.09, 0.10, 0.88));
    addToScene(scene, js_value(floor_m));

    DiscOptions dopts = DiscOptions_create();
    DiscOptions_set_radius(dopts, 2.6);
    DiscOptions_set_tessellation(dopts, 48.0);
    Mesh disc = createDisc(g_engine, dopts);
    EulerProxy_set_x(Mesh_get_rotation(disc), PI / 2.0);
    ObservableVec3_set(Mesh_get_position(disc), 0.0, 0.03, 0.0);
    Mesh_set_material(disc, mk_pbr(0.22, 0.25, 0.65));
    addToScene(scene, js_value(disc));

    BoxOptions sbopts = BoxOptions_create();
    BoxOptions_set_size(sbopts, 2.1);
    Mesh src_box = createBox_with_BoxOptions(g_engine, sbopts);

    CylinderOptions copts = CylinderOptions_create();
    CylinderOptions_set_height(copts, 3.0);
    CylinderOptions_set_diameter(copts, 1.18);
    CylinderOptions_set_tessellation(copts, 16.0);
    Mesh cyl_y = createCylinder(g_engine, copts);
    Mesh cyl_x = createCylinder(g_engine, copts);
    EulerProxy_set_z(Mesh_get_rotation(cyl_x), PI / 2.0);
    Mesh cyl_z = createCylinder(g_engine, copts);
    EulerProxy_set_x(Mesh_get_rotation(cyl_z), PI / 2.0);

    CsgSolid s1 = csgSubtract(createCsgFromMesh(src_box), createCsgFromMesh(cyl_y));
    CsgSolid s2 = csgSubtract(s1, createCsgFromMesh(cyl_x));
    CsgSolid s3 = csgSubtract(s2, createCsgFromMesh(cyl_z));
    Mesh monolith = createMeshFromCsg(g_engine, s3, "monolith");
    ObservableVec3_set(Mesh_get_position(monolith), 0.0, 2.15, 0.0);
    g_mono_rot = Mesh_get_rotation(monolith);
    js_retain(g_mono_rot);

    createPbrMaterial_props mprops = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(mprops, JsTuple_double_double_double_double_new(0.86, 0.86, 0.86, 1.0));
    createPbrMaterial_props_set_metallicFactor(mprops, 0.65);
    createPbrMaterial_props_set_roughnessFactor(mprops, 0.24);
    ClearCoatProps cc = ClearCoatProps_create();
    ClearCoatProps_set_isEnabled(cc, true);
    ClearCoatProps_set_intensity(cc, 0.90);
    ClearCoatProps_set_roughness(cc, 0.10);
    setPbrClearCoat(mprops, cc);
    Mesh_set_material(monolith, createPbrMaterial(mprops));
    addToScene(scene, js_value(monolith));

    BoxOptions cbopts = BoxOptions_create();
    BoxOptions_set_size(cbopts, 0.82);
    SphereOptions csopts = SphereOptions_create();
    SphereOptions_set_diameter(csopts, 1.02);
    SphereOptions_set_segments(csopts, 14.0);
    CsgSolid core_s = csgIntersect(
        createCsgFromMesh(createBox_with_BoxOptions(g_engine, cbopts)),
        createCsgFromMesh(createSphere(g_engine, csopts)));
    Mesh core = createMeshFromCsg(g_engine, core_s, "core");
    ObservableVec3_set(Mesh_get_position(core), 0.0, 2.15, 0.0);
    g_core_rot = Mesh_get_rotation(core);
    js_retain(g_core_rot);
    Mesh_set_material(core, mk_pbr(0.18, 0.85, 0.16));
    addToScene(scene, js_value(core));

    TorusOptions otopts = TorusOptions_create();
    TorusOptions_set_diameter(otopts, 4.2);
    TorusOptions_set_thickness(otopts, 0.09);
    TorusOptions_set_tessellation(otopts, 48.0);
    Mesh outer_ring = createTorus(g_engine, otopts);
    ObservableVec3_set(Mesh_get_position(outer_ring), 0.0, 2.15, 0.0);
    g_outer_rot = Mesh_get_rotation(outer_ring);
    js_retain(g_outer_rot);
    Mesh_set_material(outer_ring, mk_pbr(0.55, 0.80, 0.25));
    addToScene(scene, js_value(outer_ring));

    TorusOptions itopts = TorusOptions_create();
    TorusOptions_set_diameter(itopts, 3.55);
    TorusOptions_set_thickness(itopts, 0.07);
    TorusOptions_set_tessellation(itopts, 48.0);
    Mesh inner_ring = createTorus(g_engine, itopts);
    ObservableVec3_set(Mesh_get_position(inner_ring), 0.0, 2.15, 0.0);
    g_inner_rot = Mesh_get_rotation(inner_ring);
    js_retain(g_inner_rot);
    Mesh_set_material(inner_ring, mk_pbr(0.88, 0.40, 0.30));
    addToScene(scene, js_value(inner_ring));

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
