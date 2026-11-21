#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include <math.h>
#include <stdio.h>

static const double PI = 3.14159265358979323846;

static HTMLCanvasElement g_canvas;
static Babylon_EngineContext g_engine;
static Babylon_EulerProxy g_mono_rot;
static Babylon_EulerProxy g_core_rot;
static Babylon_EulerProxy g_outer_rot;
static Babylon_EulerProxy g_inner_rot;
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
    Babylon_EulerProxy_set_y(g_mono_rot, g_elapsed * 0.35);
    Babylon_EulerProxy_set_x(g_mono_rot, sin(g_elapsed * 0.3) * 0.20);
    Babylon_EulerProxy_set_y(g_core_rot, -g_elapsed * 0.90);
    Babylon_EulerProxy_set_z(g_core_rot, g_elapsed * 0.50);
    Babylon_EulerProxy_set_x(g_outer_rot, g_elapsed * 0.42);
    Babylon_EulerProxy_set_y(g_outer_rot, g_elapsed * 0.25);
    Babylon_EulerProxy_set_z(g_inner_rot, -g_elapsed * 0.58);
    Babylon_EulerProxy_set_x(g_inner_rot, g_elapsed * 0.31);
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

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(PI / 4.2, PI / 2.9, 8.8, babylon_vec3(0.0, 2.1, 0.0));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.85, 0.85));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.14, 0.14, 0.14));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.2, -2.0, -1.1), 2.1);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 1.0, 1.0));
    babylon_addToScene(scene, js_value(dir));

    Babylon_PointLight key_light = babylon_createPointLight(JsTuple_double_double_double_new(3.2, 3.6, 2.4), 8.0);
    Babylon_PointLight_set_diffuse(key_light, JsTuple_double_double_double_new(0.92, 0.92, 0.92));
    Babylon_PointLight_set_range(key_light, 14.0);
    babylon_addToScene(scene, js_value(key_light));

    Babylon_GroundOptions gopts = Babylon_GroundOptions_create();
    Babylon_GroundOptions_set_width(gopts, 16.0);
    Babylon_GroundOptions_set_height(gopts, 16.0);
    Babylon_Mesh floor_m = babylon_createGround(g_engine, gopts);
    Babylon_Mesh_set_material(floor_m, mk_pbr(0.09, 0.10, 0.88));
    babylon_addToScene(scene, js_value(floor_m));

    Babylon_DiscOptions dopts = Babylon_DiscOptions_create();
    Babylon_DiscOptions_set_radius(dopts, 2.6);
    Babylon_DiscOptions_set_tessellation(dopts, 48.0);
    Babylon_Mesh disc = babylon_createDisc(g_engine, dopts);
    Babylon_EulerProxy_set_x(Babylon_Mesh_get_rotation(disc), PI / 2.0);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(disc), 0.0, 0.03, 0.0);
    Babylon_Mesh_set_material(disc, mk_pbr(0.22, 0.25, 0.65));
    babylon_addToScene(scene, js_value(disc));

    Babylon_BoxOptions sbopts = Babylon_BoxOptions_create();
    Babylon_BoxOptions_set_size(sbopts, 2.1);
    Babylon_Mesh src_box = babylon_createBox_with_BoxOptions(g_engine, sbopts);

    Babylon_CylinderOptions copts = Babylon_CylinderOptions_create();
    Babylon_CylinderOptions_set_height(copts, 3.0);
    Babylon_CylinderOptions_set_diameter(copts, 1.18);
    Babylon_CylinderOptions_set_tessellation(copts, 16.0);
    Babylon_Mesh cyl_y = babylon_createCylinder(g_engine, copts);
    Babylon_Mesh cyl_x = babylon_createCylinder(g_engine, copts);
    Babylon_EulerProxy_set_z(Babylon_Mesh_get_rotation(cyl_x), PI / 2.0);
    Babylon_Mesh cyl_z = babylon_createCylinder(g_engine, copts);
    Babylon_EulerProxy_set_x(Babylon_Mesh_get_rotation(cyl_z), PI / 2.0);

    Babylon_CsgSolid s1 = babylon_csgSubtract(babylon_createCsgFromMesh(src_box), babylon_createCsgFromMesh(cyl_y));
    Babylon_CsgSolid s2 = babylon_csgSubtract(s1, babylon_createCsgFromMesh(cyl_x));
    Babylon_CsgSolid s3 = babylon_csgSubtract(s2, babylon_createCsgFromMesh(cyl_z));
    Babylon_Mesh monolith = babylon_createMeshFromCsg(g_engine, s3, "monolith");
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(monolith), 0.0, 2.15, 0.0);
    g_mono_rot = Babylon_Mesh_get_rotation(monolith);
    js_retain(g_mono_rot);

    Babylon_createPbrMaterial_props mprops = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(mprops, JsTuple_double_double_double_double_new(0.86, 0.86, 0.86, 1.0));
    Babylon_createPbrMaterial_props_set_metallicFactor(mprops, 0.65);
    Babylon_createPbrMaterial_props_set_roughnessFactor(mprops, 0.24);
    Babylon_ClearCoatProps cc = Babylon_ClearCoatProps_create();
    Babylon_ClearCoatProps_set_isEnabled(cc, true);
    Babylon_ClearCoatProps_set_intensity(cc, 0.90);
    Babylon_ClearCoatProps_set_roughness(cc, 0.10);
    babylon_setPbrClearCoat(mprops, cc);
    Babylon_Mesh_set_material(monolith, babylon_createPbrMaterial(mprops));
    babylon_addToScene(scene, js_value(monolith));

    Babylon_BoxOptions cbopts = Babylon_BoxOptions_create();
    Babylon_BoxOptions_set_size(cbopts, 0.82);
    Babylon_SphereOptions csopts = Babylon_SphereOptions_create();
    Babylon_SphereOptions_set_diameter(csopts, 1.02);
    Babylon_SphereOptions_set_segments(csopts, 14.0);
    Babylon_CsgSolid core_s = babylon_csgIntersect(
        babylon_createCsgFromMesh(babylon_createBox_with_BoxOptions(g_engine, cbopts)),
        babylon_createCsgFromMesh(babylon_createSphere(g_engine, csopts)));
    Babylon_Mesh core = babylon_createMeshFromCsg(g_engine, core_s, "core");
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(core), 0.0, 2.15, 0.0);
    g_core_rot = Babylon_Mesh_get_rotation(core);
    js_retain(g_core_rot);
    Babylon_Mesh_set_material(core, mk_pbr(0.18, 0.85, 0.16));
    babylon_addToScene(scene, js_value(core));

    Babylon_TorusOptions otopts = Babylon_TorusOptions_create();
    Babylon_TorusOptions_set_diameter(otopts, 4.2);
    Babylon_TorusOptions_set_thickness(otopts, 0.09);
    Babylon_TorusOptions_set_tessellation(otopts, 48.0);
    Babylon_Mesh outer_ring = babylon_createTorus(g_engine, otopts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(outer_ring), 0.0, 2.15, 0.0);
    g_outer_rot = Babylon_Mesh_get_rotation(outer_ring);
    js_retain(g_outer_rot);
    Babylon_Mesh_set_material(outer_ring, mk_pbr(0.55, 0.80, 0.25));
    babylon_addToScene(scene, js_value(outer_ring));

    Babylon_TorusOptions itopts = Babylon_TorusOptions_create();
    Babylon_TorusOptions_set_diameter(itopts, 3.55);
    Babylon_TorusOptions_set_thickness(itopts, 0.07);
    Babylon_TorusOptions_set_tessellation(itopts, 48.0);
    Babylon_Mesh inner_ring = babylon_createTorus(g_engine, itopts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(inner_ring), 0.0, 2.15, 0.0);
    g_inner_rot = Babylon_Mesh_get_rotation(inner_ring);
    js_retain(g_inner_rot);
    Babylon_Mesh_set_material(inner_ring, mk_pbr(0.88, 0.40, 0.30));
    babylon_addToScene(scene, js_value(inner_ring));

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
