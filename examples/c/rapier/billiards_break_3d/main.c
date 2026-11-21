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

#define RACK_COUNT 10
#define BALL_COUNT 11
#define BALL_RADIUS 0.36

static HTMLCanvasElement g_canvas;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static EngineContext g_engine;
static World g_world;
static RigidBody g_balls[BALL_COUNT];
static bool g_has_bodies = false;
static ObservableVec3 g_ball_pos[BALL_COUNT];
static ObservableQuat g_ball_rot[BALL_COUNT];
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

static void build_world(bool shoot_cue) {
    js_scope_open();
    if (g_world.h != 0) {
        if (g_has_bodies) {
            for (int i = 0; i < BALL_COUNT; i++) js_release(g_balls[i]);
            g_has_bodies = false;
        }
        World_free(g_world);
        js_release(g_world);
        g_world = (World){ 0 };
    }

    g_world = World_new(Vector3_new(0.0, -9.81, 0.0));
    js_retain(g_world);

    RigidBody bed = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0.0, -0.25, 0.0));
    World_createCollider(
        g_world,
        ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(4.6, 0.25, 7.6), 0.14), 0.35),
        bed
    );

    static const double rails[4][6] = {
        { -4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 4.35, 0.45, 0.0, 0.25, 0.45, 7.6 },
        { 0.0, 0.45, -7.35, 4.6, 0.45, 0.25 },
        { 0.0, 0.45, 7.35, 4.6, 0.45, 0.25 },
    };
    for (int i = 0; i < 4; i++) {
        RigidBody rb = World_createRigidBody(
            g_world,
            RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), rails[i][0], rails[i][1], rails[i][2])
        );
        World_createCollider(
            g_world,
            ColliderDesc_setRestitution(
                ColliderDesc_setFriction(ColliderDesc_cuboid(rails[i][3], rails[i][4], rails[i][5]), 0.12),
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
            RigidBodyDesc desc = RigidBodyDesc_setAngularDamping(
                RigidBodyDesc_setLinearDamping(
                    RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), x, BALL_RADIUS, z),
                    0.30
                ),
                0.30
            );
            RigidBody rb = World_createRigidBody(g_world, desc);
            World_createCollider(
                g_world,
                ColliderDesc_setDensity(
                    ColliderDesc_setFriction(ColliderDesc_setRestitution(ColliderDesc_ball(BALL_RADIUS), 0.92), 0.10),
                    1.0
                ),
                rb
            );
            g_balls[idx] = rb;
            js_retain(g_balls[idx]);
            idx++;
        }
    }

    RigidBodyDesc cue_desc = RigidBodyDesc_setAngularDamping(
        RigidBodyDesc_setLinearDamping(
            RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), 0.04, BALL_RADIUS, 4.4),
            0.22
        ),
        0.22
    );
    if (shoot_cue) {
        cue_desc = RigidBodyDesc_setLinvel(cue_desc, -0.18, 0.0, -16.0);
    }
    RigidBody cue = World_createRigidBody(g_world, cue_desc);
    World_createCollider(
        g_world,
        ColliderDesc_setDensity(
            ColliderDesc_setFriction(ColliderDesc_setRestitution(ColliderDesc_ball(BALL_RADIUS), 0.94), 0.10),
            1.25
        ),
        cue
    );
    g_balls[RACK_COUNT] = cue;
    js_retain(g_balls[RACK_COUNT]);
    g_has_bodies = true;
    g_step_count = 0;
    js_scope_close();
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope_open();
    for (int i = 0; i < BALL_COUNT; i++) {
        Vector p = RigidBody_translation(g_balls[i]);
        Rotation q = RigidBody_rotation(g_balls[i]);
        ObservableVec3_set(g_ball_pos[i], Vector_get_x(p), Vector_get_y(p), Vector_get_z(p));
        ObservableQuat_set(g_ball_rot[i], Rotation_get_x(q), Rotation_get_y(q), Rotation_get_z(q), Rotation_get_w(q));
    }
    js_scope_close();
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope_open();
    int moving = 0;
    double ke = 0.0;
    for (int i = 0; i < BALL_COUNT; i++) {
        Vector v = RigidBody_linvel(g_balls[i]);
        double vx = Vector_get_x(v), vy = Vector_get_y(v), vz = Vector_get_z(v);
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
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_world(true);
        sync_meshes();
        update_meta();
    } else if (id == 1) {
        if (g_has_bodies) {
            for (int i = 0; i < BALL_COUNT; i++) {
                double ang = (double)i * 1.15;
                RigidBody_applyImpulse(g_balls[i], Vector3_new(cos(ang) * 1.4, 0.25, sin(ang) * 1.4), true);
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
    GPUColorDict_set_r(clear, 0.05);
    GPUColorDict_set_g(clear, 0.07);
    GPUColorDict_set_b(clear, 0.09);
    GPUColorDict_set_a(clear, 1.0);
    SceneContext_set_clearColor(scene, clear);

    ArcRotateCamera camera = createArcRotateCamera(1.18, 0.92, 13.5, vec3(0.0, 0.3, -0.4));
    addToScene(scene, js_value(camera));
    attachControl(camera, g_canvas, scene);

    HemisphericLight hemi = createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.9, 0.95));
    HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.12, 0.14, 0.16));
    addToScene(scene, js_value(hemi));

    DirectionalLight dir = createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.0), 2.1);
    DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.97, 0.92));
    addToScene(scene, js_value(dir));

    BoxOptions topts = BoxOptions_create();
    BoxOptions_set_width(topts, 9.2);
    BoxOptions_set_height(topts, 0.5);
    BoxOptions_set_depth(topts, 15.2);
    Mesh table = createBox_with_BoxOptions(g_engine, topts);
    ObservableVec3_set(Mesh_get_position(table), 0.0, -0.25, 0.0);
    Mesh_set_material(table, mk_pbr(0.08, 0.24, 0.18, 0.05, 0.78));
    addToScene(scene, js_value(table));

    PbrMaterialProps rail_mat = mk_pbr(0.18, 0.18, 0.20, 0.35, 0.42);
    static const double rail_meshes[4][6] = {
        { -4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 4.35, 0.45, 0.0, 0.5, 0.9, 15.2 },
        { 0.0, 0.45, -7.35, 9.2, 0.9, 0.5 },
        { 0.0, 0.45, 7.35, 9.2, 0.9, 0.5 },
    };
    for (int i = 0; i < 4; i++) {
        BoxOptions ropts = BoxOptions_create();
        BoxOptions_set_width(ropts, rail_meshes[i][3]);
        BoxOptions_set_height(ropts, rail_meshes[i][4]);
        BoxOptions_set_depth(ropts, rail_meshes[i][5]);
        Mesh r = createBox_with_BoxOptions(g_engine, ropts);
        ObservableVec3_set(Mesh_get_position(r), rail_meshes[i][0], rail_meshes[i][1], rail_meshes[i][2]);
        Mesh_set_material(r, rail_mat);
        addToScene(scene, js_value(r));
    }

    PbrMaterialProps cue_mat = mk_pbr(0.96, 0.96, 0.95, 0.15, 0.18);
    PbrMaterialProps rack_mats[4] = {
        mk_pbr(0.85, 0.24, 0.22, 0.2, 0.22),
        mk_pbr(0.22, 0.48, 0.85, 0.2, 0.22),
        mk_pbr(0.92, 0.68, 0.18, 0.2, 0.22),
        mk_pbr(0.20, 0.20, 0.22, 0.25, 0.2),
    };

    SphereOptions sopts = SphereOptions_create();
    SphereOptions_set_diameter(sopts, BALL_RADIUS * 2.0);
    SphereOptions_set_segments(sopts, 24.0);
    for (int i = 0; i < BALL_COUNT; i++) {
        Mesh m = createSphere(g_engine, sopts);
        Mesh_set_material(m, (i == RACK_COUNT) ? cue_mat : rack_mats[i % 4]);
        addToScene(scene, js_value(m));
        g_ball_pos[i] = Mesh_get_position(m);
        g_ball_rot[i] = Mesh_get_rotationQuaternion(m);
        js_retain(g_ball_pos[i]);
        js_retain(g_ball_rot[i]);
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

    build_world(true);
    for (int i = 0; i < 26; i++) {
        World_step(g_world);
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
