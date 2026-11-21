#include "dom/dom.h"
#define Ray BabylonRay
#define Ray_get_origin BabylonRay_get_origin
#define Ray_set_origin BabylonRay_set_origin
#include "babylonjs_lite/babylonjs_lite.h"
#undef Ray
#undef Ray_get_origin
#undef Ray_set_origin
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"
#include <math.h>
#include <stdio.h>

#define DOM_COUNT 14
#define CYL_COUNT 3
#define DOM_HX 0.09
#define DOM_HY 0.65
#define DOM_HZ 0.34
#define MARBLE_RADIUS 0.36
#define RAMP_ANGLE (-0.38)
#define RAMP_X (-3.55)
#define RAMP_Y 1.02
#define RAMP_Z (-2.5)

typedef struct {
    double x, y, z, yaw;
} Pose;

static Pose spiral_pose(int i) {
    double t = (double)i * 0.255;
    double r = 2.6 - (double)i * 0.045;
    return (Pose){
        -1.35 + r * sin(t),
        DOM_HY,
        -2.5 + 2.6 - r * cos(t),
        -t,
    };
}

static HTMLCanvasElement g_canvas;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static EngineContext g_engine;
static World g_world;
static RigidBody g_marble;
static RigidBody g_doms[DOM_COUNT];
static RigidBody g_cyls[CYL_COUNT];
static bool g_has_bodies = false;
static ObservableVec3 g_marble_pos;
static ObservableQuat g_marble_rot;
static ObservableVec3 g_dom_pos[DOM_COUNT];
static ObservableQuat g_dom_rot[DOM_COUNT];
static ObservableVec3 g_cyl_pos[CYL_COUNT];
static ObservableQuat g_cyl_rot[CYL_COUNT];
static bool g_has_meshes = false;
static bool g_paused = false;
static int g_step_count = 0;

static void set_status_text(const char* text) {
    js_scope_open();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (el.h != 0) HTMLElement_set_textContent(el, text);
    js_scope_close();
}

static void set_status_error(JsValue err) {
    js_scope_open();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (el.h != 0) {
        char msg[272], buf[320];
        JsString_copy(js_error_message(err), msg, sizeof msg);
        snprintf(buf, sizeof buf, "WebGPU / Babylon Lite error: %s", msg);
        HTMLElement_set_textContent(el, buf);
    }
    js_scope_close();
}

static void build_world(void) {
    js_scope_open();
    if (g_world.h != 0) {
        if (g_has_bodies) {
            js_release(g_marble);
            for (int i = 0; i < DOM_COUNT; i++) js_release(g_doms[i]);
            for (int j = 0; j < CYL_COUNT; j++) js_release(g_cyls[j]);
            g_has_bodies = false;
        }
        World_free(g_world);
        js_release(g_world);
        g_world = (World){ 0 };
    }

    g_world = World_new(Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    RigidBody floor_body = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, -0.3, 0.0));
    World_createCollider(
        g_world,
        ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(10.0, 0.3, 10.0), 0.75), 0.1),
        floor_body
    );

    Quaternion ramp_rot = Quaternion_new(0.0, 0.0, sin(RAMP_ANGLE * 0.5), cos(RAMP_ANGLE * 0.5));
    RigidBody ramp_body = World_createRigidBody(
        g_world,
        RigidBodyDesc_setRotation(RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), RAMP_X, RAMP_Y, RAMP_Z), ramp_rot)
    );
    World_createCollider(
        g_world,
        ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(1.75, 0.10, 0.65), 0.15), 0.2),
        ramp_body
    );

    g_marble = World_createRigidBody(
        g_world,
        RigidBodyDesc_setLinvel(RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), -4.75, 2.02, -2.5), 3.0, 0.0, 0.0)
    );
    World_createCollider(
        g_world,
        ColliderDesc_setFriction(
            ColliderDesc_setRestitution(ColliderDesc_setDensity(ColliderDesc_ball(MARBLE_RADIUS), 3.2), 0.32),
            0.18
        ),
        g_marble
    );
    js_retain(g_marble);

    for (int i = 0; i < DOM_COUNT; i++) {
        Pose pose = spiral_pose(i);
        Quaternion q = Quaternion_new(0.0, sin(pose.yaw * 0.5), 0.0, cos(pose.yaw * 0.5));
        RigidBody rb = World_createRigidBody(
            g_world,
            RigidBodyDesc_setRotation(
                RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), pose.x, pose.y, pose.z),
                q
            )
        );
        World_createCollider(
            g_world,
            ColliderDesc_setDensity(
                ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(DOM_HX, DOM_HY, DOM_HZ), 0.55), 0.18),
                1.0
            ),
            rb
        );
        g_doms[i] = rb;
        js_retain(g_doms[i]);
    }

    Pose end_pose = spiral_pose(DOM_COUNT - 1);
    double fwd_x = cos(-end_pose.yaw);
    double fwd_z = sin(-end_pose.yaw);
    double perp_x = -fwd_z;
    double perp_z = fwd_x;
    for (int j = 0; j < CYL_COUNT; j++) {
        double spread = ((double)j - 1.0) * 0.52;
        double cx = end_pose.x + fwd_x * 0.72 + perp_x * spread;
        double cz = end_pose.z + fwd_z * 0.72 + perp_z * spread;
        RigidBody rb = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), cx, 0.42, cz));
        World_createCollider(
            g_world,
            ColliderDesc_setDensity(
                ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cylinder(0.42, 0.25), 0.55), 0.25),
                1.0
            ),
            rb
        );
        g_cyls[j] = rb;
        js_retain(g_cyls[j]);
    }

    g_has_bodies = true;
    g_step_count = 0;
    js_scope_close();
}

static void sync_pair(RigidBody rb, ObservableVec3 pos, ObservableQuat rot) {
    Vector p = RigidBody_translation(rb);
    Rotation q = RigidBody_rotation(rb);
    ObservableVec3_set(pos, Vector_get_x(p), Vector_get_y(p), Vector_get_z(p));
    ObservableQuat_set(rot, Rotation_get_x(q), Rotation_get_y(q), Rotation_get_z(q), Rotation_get_w(q));
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope_open();
    sync_pair(g_marble, g_marble_pos, g_marble_rot);
    for (int i = 0; i < DOM_COUNT; i++) sync_pair(g_doms[i], g_dom_pos[i], g_dom_rot[i]);
    for (int j = 0; j < CYL_COUNT; j++) sync_pair(g_cyls[j], g_cyl_pos[j], g_cyl_rot[j]);
    js_scope_close();
}

static double body_ke(RigidBody rb) {
    Vector v = RigidBody_linvel(rb);
    double vx = Vector_get_x(v), vy = Vector_get_y(v), vz = Vector_get_z(v);
    return 0.5 * (vx * vx + vy * vy + vz * vz);
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope_open();
    int fallen = 0;
    double ke = body_ke(g_marble);
    for (int i = 0; i < DOM_COUNT; i++) {
        Vector p = RigidBody_translation(g_doms[i]);
        if (Vector_get_y(p) < 0.48) fallen++;
        ke += body_ke(g_doms[i]);
    }
    for (int j = 0; j < CYL_COUNT; j++) {
        ke += body_ke(g_cyls[j]);
    }

    char buf[192];
    snprintf(
        buf,
        sizeof buf,
        "18 bodies (marble + 14 dominoes + 3 cylinders) · fallen=%d · ke=%.1f · step=%d (%s)",
        fallen,
        ke,
        g_step_count,
        g_paused ? "paused" : "running"
    );
    HTMLElement_set_textContent(g_meta, buf);
    js_scope_close();
}

static void step_sim(void) {
    js_scope_open();
    if (g_world.h != 0) World_step(g_world);
    g_step_count++;
    sync_meshes();
    update_meta();
    js_scope_close();
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    (void)delta_ms;
    if (!g_paused) step_sim();
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };

static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int id = *(const int*)ctx;
    if (id == 0) {
        if (g_has_bodies) {
            RigidBody_setTranslation(g_marble, Vector3_new(-4.75, 2.02, -2.5), true);
            RigidBody_setLinvel(g_marble, Vector3_new(3.0, 0.0, 0.0), true);
            RigidBody_setAngvel(g_marble, Vector3_new(0.0, 0.0, 0.0), true);
        }
        sync_meshes();
        update_meta();
    } else if (id == 1) {
        if (g_has_bodies) {
            RigidBody_applyImpulse(g_doms[0], Vector3_new(1.6, 0.0, 0.0), true);
        }
        update_meta();
    } else if (id == 2) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        update_meta();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_world();
        sync_meshes();
        update_meta();
    }
    js_scope_close();
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

static PbrMaterialProps mk_pbr(double r, double g, double bl, double metallic, double roughness) {
    createPbrMaterial_props props = createPbrMaterial_props_create();
    createPbrMaterial_props_set_baseColorFactor(props, JsTuple_double_double_double_double_new(r, g, bl, 1.0));
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
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.09);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);

    ArcRotateCamera camera = createArcRotateCamera(0.65, 1.05, 12.0, vec3(-0.4, 0.8, 0.0));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.88, 0.95));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.14, 0.14, 0.16));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.2), 2.1);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.97, 0.92));
    addToScene(scene, js_value(dir));

    GroundOptions gopts = GroundOptions_create();
    GroundOptions_set_width(gopts, 20.0);
    GroundOptions_set_height(gopts, 20.0);
    Mesh ground = createGround(g_engine, gopts);
    Mesh_set_material(ground, createGridMaterial());
    addToScene(scene, js_value(ground));

    BoxOptions ropts = BoxOptions_create();
    BoxOptions_set_width(ropts, 3.5);
    BoxOptions_set_height(ropts, 0.2);
    BoxOptions_set_depth(ropts, 1.3);
    Mesh ramp_mesh = createBox_with_BoxOptions(g_engine, ropts);
    ObservableVec3_set(Mesh_get_position(ramp_mesh), RAMP_X, RAMP_Y, RAMP_Z);
    ObservableQuat_set(Mesh_get_rotationQuaternion(ramp_mesh), 0.0, 0.0, sin(RAMP_ANGLE * 0.5), cos(RAMP_ANGLE * 0.5));
    Mesh_set_material(ramp_mesh, mk_pbr(0.28, 0.30, 0.34, 0.35, 0.48));
    addToScene(scene, js_value(ramp_mesh));

    SphereOptions sopts = SphereOptions_create();
    SphereOptions_set_diameter(sopts, MARBLE_RADIUS * 2.0);
    SphereOptions_set_segments(sopts, 24.0);
    Mesh mb = createSphere(g_engine, sopts);
    Mesh_set_material(mb, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.2));
    addToScene(scene, js_value(mb));
    g_marble_pos = Mesh_get_position(mb);
    g_marble_rot = Mesh_get_rotationQuaternion(mb);
    js_retain(g_marble_pos);
    js_retain(g_marble_rot);

    PbrMaterialProps mat_a = mk_pbr(0.88, 0.88, 0.88, 0.15, 0.4);
    PbrMaterialProps mat_b = mk_pbr(0.45, 0.46, 0.48, 0.25, 0.38);
    BoxOptions dopts = BoxOptions_create();
    BoxOptions_set_width(dopts, DOM_HX * 2.0);
    BoxOptions_set_height(dopts, DOM_HY * 2.0);
    BoxOptions_set_depth(dopts, DOM_HZ * 2.0);
    for (int i = 0; i < DOM_COUNT; i++) {
        Mesh m = createBox_with_BoxOptions(g_engine, dopts);
        Mesh_set_material(m, (i % 2 == 0) ? mat_a : mat_b);
        addToScene(scene, js_value(m));
        g_dom_pos[i] = Mesh_get_position(m);
        g_dom_rot[i] = Mesh_get_rotationQuaternion(m);
        js_retain(g_dom_pos[i]);
        js_retain(g_dom_rot[i]);
    }

    PbrMaterialProps cyl_mat = mk_pbr(0.26, 0.62, 0.88, 0.45, 0.25);
    CylinderOptions copts = CylinderOptions_create();
    CylinderOptions_set_height(copts, 0.84);
    CylinderOptions_set_diameter(copts, 0.5);
    CylinderOptions_set_tessellation(copts, 24.0);
    for (int j = 0; j < CYL_COUNT; j++) {
        Mesh m = createCylinder(g_engine, copts);
        Mesh_set_material(m, cyl_mat);
        addToScene(scene, js_value(m));
        g_cyl_pos[j] = Mesh_get_position(m);
        g_cyl_rot[j] = Mesh_get_rotationQuaternion(m);
        js_retain(g_cyl_pos[j]);
        js_retain(g_cyl_rot[j]);
    }
    g_has_meshes = true;

    sync_meshes();

    onBeforeRender(scene, number_fn_from(on_before_render, 0));
    Promise_then(registerScene(scene), on_registered, 0);
    js_scope_close();
}

static void on_rapier_ready(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    (void)value;
    if (!ok) {
        set_status_text("Rapier3D init failed");
        return;
    }
    js_scope_open();
    Document doc = get_document();
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);
    g_pause_btn = Document_getElementById(doc, "btn-pause");
    js_retain(g_pause_btn);

    build_world();
    for (int i = 0; i < 20; i++) {
        World_step(g_world);
        g_step_count++;
    }
    update_meta();

    static const char* ids[4] = { "btn-launch", "btn-nudge", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }

    g_canvas = Document_querySelector_canvas(doc);
    if (g_canvas.h == 0) {
        set_status_text("WebGPU / Babylon Lite error: missing <canvas> element");
        js_scope_close();
        return;
    }
    js_retain(g_canvas);
    Promise_then(createEngine_with_EngineOptions(RenderCanvas_of(g_canvas)), on_engine, 0);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Promise_then(init(), on_rapier_ready, 0);
    js_scope_close();
}
