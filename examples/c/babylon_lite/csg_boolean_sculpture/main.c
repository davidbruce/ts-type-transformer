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

static HTMLCanvasElement g_canvas;
static EngineContext g_engine;
static EulerProxy g_mono_rot;
static EulerProxy g_core_rot;
static EulerProxy g_outer_rot;
static EulerProxy g_inner_rot;
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
    EulerProxy_set_y(g_mono_rot, g_elapsed * 0.35);
    EulerProxy_set_x(g_mono_rot, approx_sin(g_elapsed * 0.3) * 0.20);
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
    GPUColorDict_set_r(clear, 0.06);
    GPUColorDict_set_g(clear, 0.06);
    GPUColorDict_set_b(clear, 0.06);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);
    js_release(clear);

    Vec3 cam_target = vec3(0.0, 2.1, 0.0);
    ArcRotateCamera camera = createArcRotateCamera(PI / 4.2, PI / 2.9, 8.8, cam_target);
    addToScene(scene, js_value(camera));
    void_fn ctrl = attachControl(camera, g_canvas, scene);
    js_release(ctrl);
    js_release(camera);
    js_release(cam_target);

    JsTuple_double_double_double hemi_dir = JsTuple_double_double_double_new(0.0, 1.0, 0.2);
    HemisphericLight hemi = createHemisphericLight(hemi_dir, js_some_double(0.72));
    JsTuple_double_double_double hdc = JsTuple_double_double_double_new(0.85, 0.85, 0.85);
    JsTuple_double_double_double hgc = JsTuple_double_double_double_new(0.14, 0.14, 0.14);
    HemisphericLight_set_diffuseColor(hemi, hdc);
    HemisphericLight_set_groundColor(hemi, hgc);
    addToScene(scene, js_value(hemi));
    js_release(hgc); js_release(hdc); js_release(hemi); js_release(hemi_dir);

    JsTuple_double_double_double dir_v = JsTuple_double_double_double_new(-1.2, -2.0, -1.1);
    DirectionalLight dir = createDirectionalLight(dir_v, js_some_double(2.1));
    JsTuple_double_double_double dd = JsTuple_double_double_double_new(1.0, 1.0, 1.0);
    DirectionalLight_set_diffuse(dir, dd);
    addToScene(scene, js_value(dir));
    js_release(dd); js_release(dir); js_release(dir_v);

    JsTuple_double_double_double kl_p = JsTuple_double_double_double_new(3.2, 3.6, 2.4);
    PointLight key_light = createPointLight(kl_p, js_some_double(8.0));
    JsTuple_double_double_double kld = JsTuple_double_double_double_new(0.92, 0.92, 0.92);
    PointLight_set_diffuse(key_light, kld);
    PointLight_set_range(key_light, 14.0);
    addToScene(scene, js_value(key_light));
    js_release(kld); js_release(key_light); js_release(kl_p);

    PbrMaterialProps floor_mat = mk_pbr(0.09, 0.10, 0.88);
    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, js_some_double(16.0));
    GroundOptions_set_height(gopts, js_some_double(16.0));
    Mesh floor_m = createGround(g_engine, gopts);
    Mesh_set_material(floor_m, floor_mat);
    addToScene(scene, js_value(floor_m));
    js_release(floor_m); js_release(gopts); js_release(floor_mat);

    PbrMaterialProps disc_mat = mk_pbr(0.22, 0.25, 0.65);
    DiscOptions dopts = DiscOptions_create();
    DiscOptions_set_radius(dopts, js_some_double(2.6));
    DiscOptions_set_tessellation(dopts, js_some_double(48.0));
    Mesh disc = createDisc(g_engine, dopts);
    EulerProxy disc_rot = SceneNode_get_rotation(disc);
    EulerProxy_set_x(disc_rot, PI / 2.0);
    ObservableVec3 disc_pos = SceneNode_get_position(disc);
    ObservableVec3_set(disc_pos, 0.0, 0.03, 0.0);
    Mesh_set_material(disc, disc_mat);
    addToScene(scene, js_value(disc));
    js_release(disc_pos); js_release(disc_rot); js_release(disc); js_release(dopts); js_release(disc_mat);

    BoxOptions sbopts = BoxOptions_create();
    BoxOptions_set_size(sbopts, js_some_double(2.1));
    Mesh src_box = createBox_with_BoxOptions(g_engine, sbopts);

    CylinderOptions copts = CylinderOptions_create();
    CylinderOptions_set_height(copts, js_some_double(3.0));
    CylinderOptions_set_diameter(copts, js_some_double(1.18));
    CylinderOptions_set_tessellation(copts, js_some_double(16.0));
    Mesh cyl_y = createCylinder(g_engine, copts);
    Mesh cyl_x = createCylinder(g_engine, copts);
    EulerProxy rx = SceneNode_get_rotation(cyl_x);
    EulerProxy_set_z(rx, PI / 2.0);
    Mesh cyl_z = createCylinder(g_engine, copts);
    EulerProxy rz = SceneNode_get_rotation(cyl_z);
    EulerProxy_set_x(rz, PI / 2.0);

    CsgSolid cb = createCsgFromMesh(src_box);
    CsgSolid cy = createCsgFromMesh(cyl_y);
    CsgSolid cx = createCsgFromMesh(cyl_x);
    CsgSolid cz = createCsgFromMesh(cyl_z);
    CsgSolid s1 = csgSubtract(cb, cy);
    CsgSolid s2 = csgSubtract(s1, cx);
    CsgSolid s3 = csgSubtract(s2, cz);
    Mesh monolith = createMeshFromCsg(g_engine, s3, "monolith");
    js_release(s3); js_release(s2); js_release(s1); js_release(cz); js_release(cx); js_release(cy); js_release(cb);
    js_release(rz); js_release(cyl_z); js_release(rx); js_release(cyl_x); js_release(cyl_y);
    js_release(copts); js_release(src_box); js_release(sbopts);

    ObservableVec3 mpos = SceneNode_get_position(monolith);
    ObservableVec3_set(mpos, 0.0, 2.15, 0.0);
    g_mono_rot = SceneNode_get_rotation(monolith);
    createPbrMaterial_props mprops = createPbrMaterial_props_create();
    JsTuple_double_double_double_double mbcf = JsTuple_double_double_double_double_new(0.86, 0.86, 0.86, 1.0);
    createPbrMaterial_props_set_baseColorFactor(mprops, mbcf);
    createPbrMaterial_props_set_metallicFactor(mprops, js_some_double(0.65));
    createPbrMaterial_props_set_roughnessFactor(mprops, js_some_double(0.24));
    ClearCoatProps cc = ClearCoatProps_create();
    ClearCoatProps_set_isEnabled(cc, js_some_bool(true));
    ClearCoatProps_set_intensity(cc, js_some_double(0.90));
    ClearCoatProps_set_roughness(cc, js_some_double(0.10));
    setPbrClearCoat(mprops, cc);
    PbrMaterialProps mono_mat = createPbrMaterial(mprops);
    Mesh_set_material(monolith, mono_mat);
    addToScene(scene, js_value(monolith));
    js_release(mono_mat); js_release(cc); js_release(mbcf); js_release(mprops);
    js_release(mpos); js_release(monolith);

    BoxOptions cbopts = BoxOptions_create();
    BoxOptions_set_size(cbopts, js_some_double(0.82));
    Mesh core_box = createBox_with_BoxOptions(g_engine, cbopts);
    SphereOptions csopts = SphereOptions_create();
    SphereOptions_set_diameter(csopts, js_some_double(1.02));
    SphereOptions_set_segments(csopts, js_some_double(14.0));
    Mesh core_sphere = createSphere(g_engine, csopts);
    CsgSolid cbox_s = createCsgFromMesh(core_box);
    CsgSolid csph_s = createCsgFromMesh(core_sphere);
    CsgSolid core_s = csgIntersect(cbox_s, csph_s);
    Mesh core = createMeshFromCsg(g_engine, core_s, "core");
    js_release(core_s); js_release(csph_s); js_release(cbox_s);
    js_release(core_sphere); js_release(csopts); js_release(core_box); js_release(cbopts);

    ObservableVec3 core_pos = SceneNode_get_position(core);
    ObservableVec3_set(core_pos, 0.0, 2.15, 0.0);
    g_core_rot = SceneNode_get_rotation(core);
    PbrMaterialProps core_mat = mk_pbr(0.18, 0.85, 0.16);
    Mesh_set_material(core, core_mat);
    addToScene(scene, js_value(core));
    js_release(core_mat); js_release(core_pos); js_release(core);

    TorusOptions otopts = TorusOptions_create();
    TorusOptions_set_diameter(otopts, js_some_double(4.2));
    TorusOptions_set_thickness(otopts, js_some_double(0.09));
    TorusOptions_set_tessellation(otopts, js_some_double(48.0));
    Mesh outer_ring = createTorus(g_engine, otopts);
    ObservableVec3 opos = SceneNode_get_position(outer_ring);
    ObservableVec3_set(opos, 0.0, 2.15, 0.0);
    g_outer_rot = SceneNode_get_rotation(outer_ring);
    PbrMaterialProps outer_mat = mk_pbr(0.55, 0.80, 0.25);
    Mesh_set_material(outer_ring, outer_mat);
    addToScene(scene, js_value(outer_ring));
    js_release(outer_mat); js_release(opos); js_release(outer_ring); js_release(otopts);

    TorusOptions itopts = TorusOptions_create();
    TorusOptions_set_diameter(itopts, js_some_double(3.55));
    TorusOptions_set_thickness(itopts, js_some_double(0.07));
    TorusOptions_set_tessellation(itopts, js_some_double(48.0));
    Mesh inner_ring = createTorus(g_engine, itopts);
    ObservableVec3 ipos = SceneNode_get_position(inner_ring);
    ObservableVec3_set(ipos, 0.0, 2.15, 0.0);
    g_inner_rot = SceneNode_get_rotation(inner_ring);
    PbrMaterialProps inner_mat = mk_pbr(0.88, 0.40, 0.30);
    Mesh_set_material(inner_ring, inner_mat);
    addToScene(scene, js_value(inner_ring));
    js_release(inner_mat); js_release(ipos); js_release(inner_ring); js_release(itopts);

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
