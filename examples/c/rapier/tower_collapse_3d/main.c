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

#define LAYERS 6
#define PER_LAYER 3
#define PLANK_COUNT (LAYERS * PER_LAYER)
#define HALF_LONG 0.85
#define HALF_HEIGHT 0.22
#define HALF_THIN 0.26
#define SPHERE_RADIUS 0.65

static HTMLCanvasElement g_canvas;
static HTMLElement g_meta;
static HTMLElement g_pause_btn;
static EngineContext g_engine;
static World g_world;
static RigidBody g_planks[PLANK_COUNT];
static RigidBody g_striker;
static bool g_has_bodies = false;
static ObservableVec3 g_plank_pos[PLANK_COUNT];
static ObservableQuat g_plank_rot[PLANK_COUNT];
static ObservableVec3 g_striker_pos;
static ObservableQuat g_striker_rot;
static bool g_has_meshes = false;
static bool g_paused = false;
static int g_step_count = 0;
static int g_shot_idx = 0;

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
            for (int i = 0; i < PLANK_COUNT; i++) js_release(g_planks[i]);
            js_release(g_striker);
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

    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        int slot = idx % PER_LAYER;
        double offset = ((double)slot - 1.0) * 0.57;
        double y = HALF_HEIGHT + (double)layer * 0.445;
        double x = (layer % 2 == 0) ? 0.0 : offset;
        double z = (layer % 2 == 0) ? offset : 0.0;
        double hx = (layer % 2 == 0) ? HALF_LONG : HALF_THIN;
        double hz = (layer % 2 == 0) ? HALF_THIN : HALF_LONG;

        RigidBody rb = World_createRigidBody(g_world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), x, y, z));
        World_createCollider(
            g_world,
            ColliderDesc_setDensity(
                ColliderDesc_setRestitution(ColliderDesc_setFriction(ColliderDesc_cuboid(hx, HALF_HEIGHT, hz), 0.65), 0.12),
                1.0
            ),
            rb
        );
        g_planks[idx] = rb;
        js_retain(g_planks[idx]);
    }

    g_striker = World_createRigidBody(
        g_world,
        RigidBodyDesc_setLinvel(
            RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), -6.2, 1.55, 0.12),
            13.5, 1.1, -0.25
        )
    );
    World_createCollider(
        g_world,
        ColliderDesc_setFriction(
            ColliderDesc_setRestitution(ColliderDesc_setDensity(ColliderDesc_ball(SPHERE_RADIUS), 4.5), 0.35),
            0.3
        ),
        g_striker
    );
    js_retain(g_striker);
    g_has_bodies = true;
    g_step_count = 0;
    js_scope_close();
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope_open();
    for (int i = 0; i < PLANK_COUNT; i++) {
        Vector p = RigidBody_translation(g_planks[i]);
        Rotation q = RigidBody_rotation(g_planks[i]);
        ObservableVec3_set(g_plank_pos[i], Vector_get_x(p), Vector_get_y(p), Vector_get_z(p));
        ObservableQuat_set(g_plank_rot[i], Rotation_get_x(q), Rotation_get_y(q), Rotation_get_z(q), Rotation_get_w(q));
    }
    Vector sp = RigidBody_translation(g_striker);
    Rotation sq = RigidBody_rotation(g_striker);
    ObservableVec3_set(g_striker_pos, Vector_get_x(sp), Vector_get_y(sp), Vector_get_z(sp));
    ObservableQuat_set(g_striker_rot, Rotation_get_x(sq), Rotation_get_y(sq), Rotation_get_z(sq), Rotation_get_w(sq));
    js_scope_close();
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope_open();
    double max_y = 0.0;
    double ke = 0.0;
    for (int i = 0; i < PLANK_COUNT; i++) {
        Vector p = RigidBody_translation(g_planks[i]);
        Vector v = RigidBody_linvel(g_planks[i]);
        double py = Vector_get_y(p);
        if (py > max_y) max_y = py;
        double vx = Vector_get_x(v), vy = Vector_get_y(v), vz = Vector_get_z(v);
        ke += 0.5 * (vx * vx + vy * vy + vz * vz);
    }
    Vector sv = RigidBody_linvel(g_striker);
    double svx = Vector_get_x(sv), svy = Vector_get_y(sv), svz = Vector_get_z(sv);
    ke += 2.25 * (svx * svx + svy * svy + svz * svz);

    char buf[192];
    snprintf(
        buf,
        sizeof buf,
        "19 bodies (18 planks + sphere) · max_y=%.2f m · ke=%.1f · step=%d (%s)",
        max_y,
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

static const int BTN_TAGS[3] = { 0, 1, 2 };

static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int id = *(const int*)ctx;
    if (id == 0) {
        g_shot_idx++;
        double angle = (double)g_shot_idx * 0.9;
        double dist = 6.2;
        double sx = -dist * cos(angle);
        double sz = dist * sin(angle);
        double sy = 1.2 + (double)(g_shot_idx % 3) * 0.45;
        if (g_has_bodies) {
            RigidBody_setTranslation(g_striker, Vector3_new(sx, sy, sz), true);
            RigidBody_setLinvel(g_striker, Vector3_new(14.4 * cos(angle), 0.9, -14.4 * sin(angle)), true);
            RigidBody_setAngvel(g_striker, Vector3_new(0.0, 0.0, 0.0), true);
        }
        sync_meshes();
        update_meta();
    } else if (id == 1) {
        g_paused = !g_paused;
        HTMLElement_set_textContent(g_pause_btn, g_paused ? "Resume" : "Pause");
        update_meta();
    } else {
        g_paused = false;
        HTMLElement_set_textContent(g_pause_btn, "Pause");
        build_world();
        for (int i = 0; i < 12; i++) {
            World_step(g_world);
            g_step_count++;
        }
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

    ArcRotateCamera camera = createArcRotateCamera(0.85, 1.12, 11.5, vec3(0.0, 1.5, 0.0));
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

    PbrMaterialProps palette[3] = {
        mk_pbr(0.86, 0.86, 0.86, 0.15, 0.45),
        mk_pbr(0.58, 0.58, 0.60, 0.25, 0.38),
        mk_pbr(0.30, 0.31, 0.33, 0.35, 0.35),
    };

    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        double w_box = (layer % 2 == 0) ? (HALF_LONG * 2.0) : (HALF_THIN * 2.0);
        double d_box = (layer % 2 == 0) ? (HALF_THIN * 2.0) : (HALF_LONG * 2.0);
        BoxOptions bopts = BoxOptions_create();
        BoxOptions_set_width(bopts, w_box);
        BoxOptions_set_height(bopts, HALF_HEIGHT * 2.0);
        BoxOptions_set_depth(bopts, d_box);
        Mesh m = createBox_with_BoxOptions(g_engine, bopts);
        Mesh_set_material(m, palette[layer % 3]);
        addToScene(scene, js_value(m));
        g_plank_pos[idx] = Mesh_get_position(m);
        g_plank_rot[idx] = Mesh_get_rotationQuaternion(m);
        js_retain(g_plank_pos[idx]);
        js_retain(g_plank_rot[idx]);
    }

    SphereOptions sopts = SphereOptions_create();
    SphereOptions_set_diameter(sopts, SPHERE_RADIUS * 2.0);
    SphereOptions_set_segments(sopts, 24.0);
    Mesh sm = createSphere(g_engine, sopts);
    Mesh_set_material(sm, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.22));
    addToScene(scene, js_value(sm));
    g_striker_pos = Mesh_get_position(sm);
    g_striker_rot = Mesh_get_rotationQuaternion(sm);
    js_retain(g_striker_pos);
    js_retain(g_striker_rot);
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
    for (int i = 0; i < 18; i++) {
        World_step(g_world);
        g_step_count++;
    }
    update_meta();

    static const char* ids[3] = { "btn-launch", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
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
