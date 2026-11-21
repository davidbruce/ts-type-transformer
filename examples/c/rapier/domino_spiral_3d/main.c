#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
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
static Babylon_EngineContext g_engine;
static Rapier_World g_world;
static Rapier_RigidBody g_marble;
static Rapier_RigidBody g_doms[DOM_COUNT];
static Rapier_RigidBody g_cyls[CYL_COUNT];
static bool g_has_bodies = false;
static Babylon_ObservableVec3 g_marble_pos;
static Babylon_ObservableQuat g_marble_rot;
static Babylon_ObservableVec3 g_dom_pos[DOM_COUNT];
static Babylon_ObservableQuat g_dom_rot[DOM_COUNT];
static Babylon_ObservableVec3 g_cyl_pos[CYL_COUNT];
static Babylon_ObservableQuat g_cyl_rot[CYL_COUNT];
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
        Rapier_World_free(g_world);
        js_release(g_world);
        g_world = (Rapier_World){ 0 };
    }

    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    Rapier_RigidBody floor_body = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, -0.3, 0.0));
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(10.0, 0.3, 10.0), 0.75), 0.1),
        floor_body
    );

    Rapier_Quaternion ramp_rot = Rapier_Quaternion_new(0.0, 0.0, sin(RAMP_ANGLE * 0.5), cos(RAMP_ANGLE * 0.5));
    Rapier_RigidBody ramp_body = Rapier_World_createRigidBody(
        g_world,
        Rapier_RigidBodyDesc_setRotation(Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), RAMP_X, RAMP_Y, RAMP_Z), ramp_rot)
    );
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(1.75, 0.10, 0.65), 0.15), 0.2),
        ramp_body
    );

    g_marble = Rapier_World_createRigidBody(
        g_world,
        Rapier_RigidBodyDesc_setLinvel(Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), -4.75, 2.02, -2.5), 3.0, 0.0, 0.0)
    );
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setFriction(
            Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_ball(MARBLE_RADIUS), 3.2), 0.32),
            0.18
        ),
        g_marble
    );
    js_retain(g_marble);

    for (int i = 0; i < DOM_COUNT; i++) {
        Pose pose = spiral_pose(i);
        Rapier_Quaternion q = Rapier_Quaternion_new(0.0, sin(pose.yaw * 0.5), 0.0, cos(pose.yaw * 0.5));
        Rapier_RigidBody rb = Rapier_World_createRigidBody(
            g_world,
            Rapier_RigidBodyDesc_setRotation(
                Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), pose.x, pose.y, pose.z),
                q
            )
        );
        Rapier_World_createCollider(
            g_world,
            Rapier_ColliderDesc_setDensity(
                Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(DOM_HX, DOM_HY, DOM_HZ), 0.55), 0.18),
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
        Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), cx, 0.42, cz));
        Rapier_World_createCollider(
            g_world,
            Rapier_ColliderDesc_setDensity(
                Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cylinder(0.42, 0.25), 0.55), 0.25),
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

static void sync_pair(Rapier_RigidBody rb, Babylon_ObservableVec3 pos, Babylon_ObservableQuat rot) {
    Rapier_Vector p = Rapier_RigidBody_translation(rb);
    Rapier_Rotation q = Rapier_RigidBody_rotation(rb);
    Babylon_ObservableVec3_set(pos, Rapier_Vector_get_x(p), Rapier_Vector_get_y(p), Rapier_Vector_get_z(p));
    Babylon_ObservableQuat_set(rot, Rapier_Rotation_get_x(q), Rapier_Rotation_get_y(q), Rapier_Rotation_get_z(q), Rapier_Rotation_get_w(q));
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope_open();
    sync_pair(g_marble, g_marble_pos, g_marble_rot);
    for (int i = 0; i < DOM_COUNT; i++) sync_pair(g_doms[i], g_dom_pos[i], g_dom_rot[i]);
    for (int j = 0; j < CYL_COUNT; j++) sync_pair(g_cyls[j], g_cyl_pos[j], g_cyl_rot[j]);
    js_scope_close();
}

static double body_ke(Rapier_RigidBody rb) {
    Rapier_Vector v = Rapier_RigidBody_linvel(rb);
    double vx = Rapier_Vector_get_x(v), vy = Rapier_Vector_get_y(v), vz = Rapier_Vector_get_z(v);
    return 0.5 * (vx * vx + vy * vy + vz * vz);
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope_open();
    int fallen = 0;
    double ke = body_ke(g_marble);
    for (int i = 0; i < DOM_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_doms[i]);
        if (Rapier_Vector_get_y(p) < 0.48) fallen++;
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
    if (g_world.h != 0) Rapier_World_step(g_world);
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
            Rapier_RigidBody_setTranslation(g_marble, Rapier_Vector3_new(-4.75, 2.02, -2.5), true);
            Rapier_RigidBody_setLinvel(g_marble, Rapier_Vector3_new(3.0, 0.0, 0.0), true);
            Rapier_RigidBody_setAngvel(g_marble, Rapier_Vector3_new(0.0, 0.0, 0.0), true);
        }
        sync_meshes();
        update_meta();
    } else if (id == 1) {
        if (g_has_bodies) {
            Rapier_RigidBody_applyImpulse(g_doms[0], Rapier_Vector3_new(1.6, 0.0, 0.0), true);
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
    Promise_then(babylon_startEngine(g_engine), on_started, 0);
    js_scope_close();
}

static Babylon_PbrMaterialProps mk_pbr(double r, double g, double bl, double metallic, double roughness) {
    Babylon_createPbrMaterial_props props = Babylon_createPbrMaterial_props_create();
    Babylon_createPbrMaterial_props_set_baseColorFactor(props, JsTuple_double_double_double_double_new(r, g, bl, 1.0));
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
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.09);
    GPUColorDict_set_a(clear, 1.0);
    Babylon_SceneContext_set_clearColor(scene, clear);

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(0.65, 1.05, 12.0, babylon_vec3(-0.4, 0.8, 0.0));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.88, 0.95));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.14, 0.14, 0.16));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.2), 2.1);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.97, 0.92));
    babylon_addToScene(scene, js_value(dir));

    Babylon_GroundOptions gopts = Babylon_GroundOptions_create();
    Babylon_GroundOptions_set_width(gopts, 20.0);
    Babylon_GroundOptions_set_height(gopts, 20.0);
    Babylon_Mesh ground = babylon_createGround(g_engine, gopts);
    Babylon_Mesh_set_material(ground, babylon_createGridMaterial());
    babylon_addToScene(scene, js_value(ground));

    Babylon_BoxOptions ropts = Babylon_BoxOptions_create();
    Babylon_BoxOptions_set_width(ropts, 3.5);
    Babylon_BoxOptions_set_height(ropts, 0.2);
    Babylon_BoxOptions_set_depth(ropts, 1.3);
    Babylon_Mesh ramp_mesh = babylon_createBox_with_BoxOptions(g_engine, ropts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(ramp_mesh), RAMP_X, RAMP_Y, RAMP_Z);
    Babylon_ObservableQuat_set(Babylon_Mesh_get_rotationQuaternion(ramp_mesh), 0.0, 0.0, sin(RAMP_ANGLE * 0.5), cos(RAMP_ANGLE * 0.5));
    Babylon_Mesh_set_material(ramp_mesh, mk_pbr(0.28, 0.30, 0.34, 0.35, 0.48));
    babylon_addToScene(scene, js_value(ramp_mesh));

    Babylon_SphereOptions sopts = Babylon_SphereOptions_create();
    Babylon_SphereOptions_set_diameter(sopts, MARBLE_RADIUS * 2.0);
    Babylon_SphereOptions_set_segments(sopts, 24.0);
    Babylon_Mesh mb = babylon_createSphere(g_engine, sopts);
    Babylon_Mesh_set_material(mb, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.2));
    babylon_addToScene(scene, js_value(mb));
    g_marble_pos = Babylon_Mesh_get_position(mb);
    g_marble_rot = Babylon_Mesh_get_rotationQuaternion(mb);
    js_retain(g_marble_pos);
    js_retain(g_marble_rot);

    Babylon_PbrMaterialProps mat_a = mk_pbr(0.88, 0.88, 0.88, 0.15, 0.4);
    Babylon_PbrMaterialProps mat_b = mk_pbr(0.45, 0.46, 0.48, 0.25, 0.38);
    Babylon_BoxOptions dopts = Babylon_BoxOptions_create();
    Babylon_BoxOptions_set_width(dopts, DOM_HX * 2.0);
    Babylon_BoxOptions_set_height(dopts, DOM_HY * 2.0);
    Babylon_BoxOptions_set_depth(dopts, DOM_HZ * 2.0);
    for (int i = 0; i < DOM_COUNT; i++) {
        Babylon_Mesh m = babylon_createBox_with_BoxOptions(g_engine, dopts);
        Babylon_Mesh_set_material(m, (i % 2 == 0) ? mat_a : mat_b);
        babylon_addToScene(scene, js_value(m));
        g_dom_pos[i] = Babylon_Mesh_get_position(m);
        g_dom_rot[i] = Babylon_Mesh_get_rotationQuaternion(m);
        js_retain(g_dom_pos[i]);
        js_retain(g_dom_rot[i]);
    }

    Babylon_PbrMaterialProps cyl_mat = mk_pbr(0.26, 0.62, 0.88, 0.45, 0.25);
    Babylon_CylinderOptions copts = Babylon_CylinderOptions_create();
    Babylon_CylinderOptions_set_height(copts, 0.84);
    Babylon_CylinderOptions_set_diameter(copts, 0.5);
    Babylon_CylinderOptions_set_tessellation(copts, 24.0);
    for (int j = 0; j < CYL_COUNT; j++) {
        Babylon_Mesh m = babylon_createCylinder(g_engine, copts);
        Babylon_Mesh_set_material(m, cyl_mat);
        babylon_addToScene(scene, js_value(m));
        g_cyl_pos[j] = Babylon_Mesh_get_position(m);
        g_cyl_rot[j] = Babylon_Mesh_get_rotationQuaternion(m);
        js_retain(g_cyl_pos[j]);
        js_retain(g_cyl_rot[j]);
    }
    g_has_meshes = true;

    sync_meshes();

    babylon_onBeforeRender(scene, Babylon_number_fn_from(on_before_render, 0));
    Promise_then(babylon_registerScene(scene), on_registered, 0);
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
        Rapier_World_step(g_world);
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
    Promise_then(babylon_createEngine_with_EngineOptions(Babylon_RenderCanvas_of(g_canvas)), on_engine, 0);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Promise_then(rapier_init(), on_rapier_ready, 0);
    js_scope_close();
}
