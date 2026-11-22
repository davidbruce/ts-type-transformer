#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
#include "dimforge_rapier3d_compat/dimforge_rapier3d_compat.h"
#include <math.h>
#include <stdio.h>

#define RACK_COUNT 10
#define BALL_COUNT 11
#define BALL_RADIUS 0.36

static HTMLCanvasElement g_canvas;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static Babylon_EngineContext g_engine;
static Rapier_World g_world;
static Rapier_RigidBody g_balls[BALL_COUNT];
static bool g_has_bodies = false;
static Babylon_ObservableVec3 g_ball_pos[BALL_COUNT];
static Babylon_ObservableQuat g_ball_rot[BALL_COUNT];
static bool g_has_meshes = false;
static bool g_paused = false;
static int g_step_count = 0;

static void set_status_text(const char* text) {
    js_scope();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (el.h != 0) HTMLElement_set_textContent(el, text);
}

static void set_status_error(JsValue err) {
    js_scope();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (el.h != 0) {
        char msg[272], buf[320];
        JsString_copy(js_error_message(err), msg, sizeof msg);
        snprintf(buf, sizeof buf, "WebGPU / Babylon Lite error: %s", msg);
        HTMLElement_set_textContent(el, buf);
    }
}

static void build_world(bool shoot_cue) {
    js_scope();
    if (g_world.h != 0) {
        if (g_has_bodies) {
            for (int i = 0; i < BALL_COUNT; i++) js_release(g_balls[i]);
            g_has_bodies = false;
        }
        Rapier_World_free(g_world);
        js_release(g_world);
        g_world = (Rapier_World){ 0 };
    }

    g_world = Rapier_World_new(Rapier_Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    Rapier_RigidBody bed = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), 0.0, -0.25, 0.0));
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(4.6, 0.25, 7.6), 0.14), 0.35),
        bed
    );

    static const double rails[4][6] = {
        { -4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 0.0, 0.45, -7.35, 4.6, 0.45, 0.25 },
        { 0.0, 0.45, 7.35, 4.6, 0.45, 0.25 },
    };
    for (int i = 0; i < 4; i++) {
        Rapier_RigidBody rb = Rapier_World_createRigidBody(
            g_world,
            Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_fixed(), rails[i][0], rails[i][1], rails[i][2])
        );
        Rapier_World_createCollider(
            g_world,
            Rapier_ColliderDesc_setRestitution(
                Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(rails[i][3], rails[i][4], rails[i][5]), 0.12),
                0.88
            ),
            rb
        );
    }

    int idx = 0;
    for (int row = 0; row < 4; row++) {
        for (int col = 0; col <= row; col++) {
            double x = ((double)col - (double)row * 0.5) * 0.74;
            double z = -1.6 - (double)row * 0.645;
            Rapier_RigidBodyDesc desc = Rapier_RigidBodyDesc_setAngularDamping(
                Rapier_RigidBodyDesc_setLinearDamping(
                    Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), x, BALL_RADIUS, z),
                    0.30
                ),
                0.30
            );
            Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, desc);
            Rapier_World_createCollider(
                g_world,
                Rapier_ColliderDesc_setDensity(
                    Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_ball(BALL_RADIUS), 0.92), 0.10),
                    1.0
                ),
                rb
            );
            g_balls[idx] = rb;
            js_retain(g_balls[idx]);
            idx++;
        }
    }

    Rapier_RigidBodyDesc cue_desc = Rapier_RigidBodyDesc_setAngularDamping(
        Rapier_RigidBodyDesc_setLinearDamping(
            Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), 0.04, BALL_RADIUS, 4.4),
            0.22
        ),
        0.22
    );
    if (shoot_cue) {
        cue_desc = Rapier_RigidBodyDesc_setLinvel(cue_desc, -0.18, 0.0, -16.0);
    }
    Rapier_RigidBody cue = Rapier_World_createRigidBody(g_world, cue_desc);
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setDensity(
            Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_ball(BALL_RADIUS), 0.94), 0.10),
            1.25
        ),
        cue
    );
    g_balls[RACK_COUNT] = cue;
    js_retain(g_balls[RACK_COUNT]);
    g_has_bodies = true;
    g_step_count = 0;
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope();
    for (int i = 0; i < BALL_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_balls[i]);
        Rapier_Rotation q = Rapier_RigidBody_rotation(g_balls[i]);
        Babylon_ObservableVec3_set(g_ball_pos[i], Rapier_Vector_get_x(p), Rapier_Vector_get_y(p), Rapier_Vector_get_z(p));
        Babylon_ObservableQuat_set(g_ball_rot[i], Rapier_Rotation_get_x(q), Rapier_Rotation_get_y(q), Rapier_Rotation_get_z(q), Rapier_Rotation_get_w(q));
    }
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope();
    int moving = 0;
    double ke = 0.0;
    for (int i = 0; i < BALL_COUNT; i++) {
        Rapier_Vector v = Rapier_RigidBody_linvel(g_balls[i]);
        double vx = Rapier_Vector_get_x(v), vy = Rapier_Vector_get_y(v), vz = Rapier_Vector_get_z(v);
        double s2 = vx * vx + vy * vy + vz * vz;
        if (s2 > 0.05) moving++;
        ke += 0.5 * s2;
    }

    char buf[160];
    snprintf(
        buf,
        sizeof buf,
        "11 spheres · moving=%d · ke=%.1f · step=%d (%s)",
        moving,
        ke,
        g_step_count,
        g_paused ? "paused" : "running"
    );
    HTMLElement_set_textContent(g_meta, buf);
}

static void step_sim(void) {
    js_scope();
    if (g_world.h != 0) Rapier_World_step(g_world);
    g_step_count++;
    sync_meshes();
    update_meta();
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    (void)delta_ms;
    if (!g_paused) step_sim();
}

static const int BTN_TAGS[4] = { 0, 1, 2, 3 };

static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int id = *(const int*)ctx;
    if (id == 0) {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_world(true);
        sync_meshes();
        update_meta();
    } else if (id == 1) {
        if (g_has_bodies) {
            for (int i = 0; i < BALL_COUNT; i++) {
                double ang = (double)i * 1.15;
                Rapier_RigidBody_applyImpulse(g_balls[i], Rapier_Vector3_new(cos(ang) * 1.4, 0.25, sin(ang) * 1.4), true);
            }
        }
        update_meta();
    } else if (id == 2) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        update_meta();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_world(false);
        sync_meshes();
        update_meta();
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

static Babylon_PbrMaterialProps mk_pbr(double r, double g, double bl, double metallic, double roughness) {
    return babylon_createPbrMaterial(Babylon_createPbrMaterial_props_create(.baseColorFactor = JsTuple_double_double_double_double_new(r, g, bl, 1.0), .metallicFactor = metallic, .roughnessFactor = roughness));
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
    Babylon_SceneContext_set_clearColor(scene, GPUColorDict_create(.r = 0.05, .g = 0.07, .b = 0.09, .a = 1.0));

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(1.18, 0.92, 13.5, babylon_vec3(0.0, 0.3, -0.4));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.9, 0.95));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.14, 0.16));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.0), 2.1);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.97, 0.92));
    babylon_addToScene(scene, js_value(dir));

    Babylon_BoxOptions topts = Babylon_BoxOptions_create(.width = 9.2, .height = 0.5, .depth = 15.2);
    Babylon_Mesh table = babylon_createBox_with_BoxOptions(g_engine, topts);
    Babylon_ObservableVec3_set(Babylon_Mesh_get_position(table), 0.0, -0.25, 0.0);
    Babylon_Mesh_set_material(table, mk_pbr(0.08, 0.24, 0.18, 0.05, 0.78));
    babylon_addToScene(scene, js_value(table));

    Babylon_PbrMaterialProps rail_mat = mk_pbr(0.18, 0.18, 0.20, 0.35, 0.42);
    static const double rail_meshes[4][6] = {
        { -4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 0.0, 0.45, -7.35, 9.2, 0.9, 0.5 },
        { 0.0, 0.45, 7.35, 9.2, 0.9, 0.5 },
    };
    for (int i = 0; i < 4; i++) {
        Babylon_BoxOptions ropts = Babylon_BoxOptions_create(.width = rail_meshes[i][3], .height = rail_meshes[i][4], .depth = rail_meshes[i][5]);
        Babylon_Mesh r = babylon_createBox_with_BoxOptions(g_engine, ropts);
        Babylon_ObservableVec3_set(Babylon_Mesh_get_position(r), rail_meshes[i][0], rail_meshes[i][1], rail_meshes[i][2]);
        Babylon_Mesh_set_material(r, rail_mat);
        babylon_addToScene(scene, js_value(r));
    }

    Babylon_PbrMaterialProps cue_mat = mk_pbr(0.96, 0.96, 0.95, 0.15, 0.18);
    Babylon_PbrMaterialProps rack_mats[4] = {
        mk_pbr(0.85, 0.24, 0.22, 0.2, 0.22),
        mk_pbr(0.22, 0.48, 0.85, 0.2, 0.22),
        mk_pbr(0.92, 0.68, 0.18, 0.2, 0.22),
        mk_pbr(0.20, 0.20, 0.22, 0.25, 0.2),
    };

    Babylon_SphereOptions sopts = Babylon_SphereOptions_create(.diameter = BALL_RADIUS * 2.0, .segments = 24.0);
    for (int i = 0; i < BALL_COUNT; i++) {
        Babylon_Mesh m = babylon_createSphere(g_engine, sopts);
        Babylon_Mesh_set_material(m, (i == RACK_COUNT) ? cue_mat : rack_mats[i % 4]);
        babylon_addToScene(scene, js_value(m));
        g_ball_pos[i] = Babylon_Mesh_get_position(m);
        g_ball_rot[i] = Babylon_Mesh_get_rotationQuaternion(m);
        js_retain(g_ball_pos[i]);
        js_retain(g_ball_rot[i]);
    }
    g_has_meshes = true;

    sync_meshes();

    babylon_onBeforeRender(scene, Babylon_number_fn_from(on_before_render, 0));
    Promise_then(babylon_registerScene(scene), on_registered, 0);
}

static void on_rapier_ready(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    (void)value;
    if (!ok) {
        set_status_text("Rapier3D init failed");
        return;
    }
    js_scope();
    Document doc = get_document();
    g_meta = Document_getElementById(doc, "meta");
    js_retain(g_meta);
    g_pause_btn = Document_getElementById(doc, "btn-pause");
    js_retain(g_pause_btn);

    build_world(true);
    for (int i = 0; i < 26; i++) {
        Rapier_World_step(g_world);
        g_step_count++;
    }
    update_meta();

    static const char* ids[4] = { "btn-break", "btn-scatter", "btn-pause", "btn-reset" };
    for (int i = 0; i < 4; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }

    g_canvas = Document_querySelector_canvas(doc);
    if (g_canvas.h == 0) {
        set_status_text("WebGPU / Babylon Lite error: missing <canvas> element");
        return;
    }
    js_retain(g_canvas);
    Promise_then(babylon_createEngine_with_EngineOptions(g_canvas), on_engine, 0);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Promise_then(rapier_init(), on_rapier_ready, 0);
}
