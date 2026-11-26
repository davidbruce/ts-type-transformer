#include "dom/dom.h"
#include "babylonjs_lite/babylonjs_lite.h"
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
static Babylon_EngineContext g_engine;
static Rapier_World g_world;
static Rapier_RigidBody g_planks[PLANK_COUNT];
static Rapier_RigidBody g_striker;
static bool g_has_bodies = false;
static Babylon_ObservableVec3 g_plank_pos[PLANK_COUNT];
static Babylon_ObservableQuat g_plank_rot[PLANK_COUNT];
static Babylon_ObservableVec3 g_striker_pos;
static Babylon_ObservableQuat g_striker_rot;
static bool g_has_meshes = false;
static bool g_paused = false;
static int g_step_count = 0;
static int g_shot_idx = 0;

static void set_status_text(const char* text) {
    js_scope();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (js_not_null(el)) HTMLElement_set_textContent(el, text);
}

static void set_status_error(JsValue err) {
    js_scope();
    HTMLElement el = Document_getElementById(get_document(), "status");
    if (js_not_null(el)) {
        char msg[272], buf[320];
        JsString_copy(js_error_message(err), msg, sizeof msg);
        snprintf(buf, sizeof buf, "WebGPU / Babylon Lite error: %s", msg);
        HTMLElement_set_textContent(el, buf);
    }
}

static void build_world(void) {
    js_scope();
    if (js_not_null(g_world)) {
        if (g_has_bodies) {
            for (int i = 0; i < PLANK_COUNT; i++) js_release(g_planks[i]);
            js_release(g_striker);
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

    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        int slot = idx % PER_LAYER;
        double offset = ((double)slot - 1.0) * 0.57;
        double y = HALF_HEIGHT + (double)layer * 0.445;
        double x = (layer % 2 == 0) ? 0.0 : offset;
        double z = (layer % 2 == 0) ? offset : 0.0;
        double hx = (layer % 2 == 0) ? HALF_LONG : HALF_THIN;
        double hz = (layer % 2 == 0) ? HALF_THIN : HALF_LONG;

        Rapier_RigidBody rb = Rapier_World_createRigidBody(g_world, Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), x, y, z));
        Rapier_World_createCollider(
            g_world,
            Rapier_ColliderDesc_setDensity(
                Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setFriction(Rapier_ColliderDesc_cuboid(hx, HALF_HEIGHT, hz), 0.65), 0.12),
                1.0
            ),
            rb
        );
        g_planks[idx] = rb;
        js_retain(g_planks[idx]);
    }

    g_striker = Rapier_World_createRigidBody(
        g_world,
        Rapier_RigidBodyDesc_setLinvel(
            Rapier_RigidBodyDesc_setTranslation(Rapier_RigidBodyDesc_dynamic(), -6.2, 1.55, 0.12),
            13.5, 1.1, -0.25
        )
    );
    Rapier_World_createCollider(
        g_world,
        Rapier_ColliderDesc_setFriction(
            Rapier_ColliderDesc_setRestitution(Rapier_ColliderDesc_setDensity(Rapier_ColliderDesc_ball(SPHERE_RADIUS), 4.5), 0.35),
            0.3
        ),
        g_striker
    );
    js_retain(g_striker);
    g_has_bodies = true;
    g_step_count = 0;
}

static void sync_meshes(void) {
    if (!g_has_meshes || !g_has_bodies) return;
    js_scope();
    for (int i = 0; i < PLANK_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_planks[i]);
        Rapier_Rotation q = Rapier_RigidBody_rotation(g_planks[i]);
        Babylon_ObservableVec3_set(g_plank_pos[i], Rapier_Vector_get_x(p), Rapier_Vector_get_y(p), Rapier_Vector_get_z(p));
        Babylon_ObservableQuat_set(g_plank_rot[i], Rapier_Rotation_get_x(q), Rapier_Rotation_get_y(q), Rapier_Rotation_get_z(q), Rapier_Rotation_get_w(q));
    }
    Rapier_Vector sp = Rapier_RigidBody_translation(g_striker);
    Rapier_Rotation sq = Rapier_RigidBody_rotation(g_striker);
    Babylon_ObservableVec3_set(g_striker_pos, Rapier_Vector_get_x(sp), Rapier_Vector_get_y(sp), Rapier_Vector_get_z(sp));
    Babylon_ObservableQuat_set(g_striker_rot, Rapier_Rotation_get_x(sq), Rapier_Rotation_get_y(sq), Rapier_Rotation_get_z(sq), Rapier_Rotation_get_w(sq));
}

static void update_meta(void) {
    if (!g_has_bodies) return;
    js_scope();
    double max_y = 0.0;
    double ke = 0.0;
    for (int i = 0; i < PLANK_COUNT; i++) {
        Rapier_Vector p = Rapier_RigidBody_translation(g_planks[i]);
        Rapier_Vector v = Rapier_RigidBody_linvel(g_planks[i]);
        double py = Rapier_Vector_get_y(p);
        if (py > max_y) max_y = py;
        double vx = Rapier_Vector_get_x(v), vy = Rapier_Vector_get_y(v), vz = Rapier_Vector_get_z(v);
        ke += 0.5 * (vx * vx + vy * vy + vz * vz);
    }
    Rapier_Vector sv = Rapier_RigidBody_linvel(g_striker);
    double svx = Rapier_Vector_get_x(sv), svy = Rapier_Vector_get_y(sv), svz = Rapier_Vector_get_z(sv);
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
}

static void step_sim(void) {
    js_scope();
    if (js_not_null(g_world)) Rapier_World_step(g_world);
    g_step_count++;
    sync_meshes();
    update_meta();
}

static void on_before_render(void* ctx, double delta_ms) {
    (void)ctx;
    (void)delta_ms;
    if (!g_paused) step_sim();
}

static const int BTN_TAGS[3] = { 0, 1, 2 };

static void on_btn(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int id = *(const int*)ctx;
    if (id == 0) {
        g_shot_idx++;
        double angle = (double)g_shot_idx * 0.9;
        double dist = 6.2;
        double sx = -dist * cos(angle);
        double sz = dist * sin(angle);
        double sy = 1.2 + (double)(g_shot_idx % 3) * 0.45;
        if (g_has_bodies) {
            Rapier_RigidBody_setTranslation(g_striker, Rapier_Vector3_new(sx, sy, sz), true);
            Rapier_RigidBody_setLinvel(g_striker, Rapier_Vector3_new(14.4 * cos(angle), 0.9, -14.4 * sin(angle)), true);
            Rapier_RigidBody_setAngvel(g_striker, Rapier_Vector3_new(0.0, 0.0, 0.0), true);
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
            Rapier_World_step(g_world);
            g_step_count++;
        }
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
    g_engine = js_cast(Babylon_EngineContext, value);
    js_retain(g_engine);

    Babylon_SceneContext scene = babylon_createSceneContext(g_engine);
    Babylon_SceneContext_set_clearColor(scene, GPUColorDict_create(.r = 0.06, .g = 0.07, .b = 0.09, .a = 1.0));

    Babylon_ArcRotateCamera camera = babylon_createArcRotateCamera(0.85, 1.12, 11.5, babylon_vec3(0.0, 1.5, 0.0));
    babylon_addToScene(scene, js_value(camera));
    babylon_attachControl(camera, g_canvas, scene);

    Babylon_HemisphericLight hemi = babylon_createHemisphericLight(JsTuple_double_double_double_new(0.0, 1.0, 0.2), 0.72);
    Babylon_HemisphericLight_set_diffuseColor(hemi, JsTuple_double_double_double_new(0.85, 0.88, 0.95));
    Babylon_HemisphericLight_set_groundColor(hemi, JsTuple_double_double_double_new(0.14, 0.14, 0.16));
    babylon_addToScene(scene, js_value(hemi));

    Babylon_DirectionalLight dir = babylon_createDirectionalLight(JsTuple_double_double_double_new(-1.0, -2.0, -1.2), 2.1);
    Babylon_DirectionalLight_set_diffuse(dir, JsTuple_double_double_double_new(1.0, 0.97, 0.92));
    babylon_addToScene(scene, js_value(dir));

    Babylon_GroundOptions gopts = Babylon_GroundOptions_create(.width = 20.0, .height = 20.0);
    Babylon_Mesh ground = babylon_createGround(g_engine, gopts);
    Babylon_Mesh_set_material(ground, babylon_createGridMaterial());
    babylon_addToScene(scene, js_value(ground));

    Babylon_PbrMaterialProps palette[3] = {
        mk_pbr(0.86, 0.86, 0.86, 0.15, 0.45),
        mk_pbr(0.58, 0.58, 0.60, 0.25, 0.38),
        mk_pbr(0.30, 0.31, 0.33, 0.35, 0.35),
    };

    for (int idx = 0; idx < PLANK_COUNT; idx++) {
        int layer = idx / PER_LAYER;
        double w_box = (layer % 2 == 0) ? (HALF_LONG * 2.0) : (HALF_THIN * 2.0);
        double d_box = (layer % 2 == 0) ? (HALF_THIN * 2.0) : (HALF_LONG * 2.0);
        Babylon_BoxOptions bopts = Babylon_BoxOptions_create(.width = w_box, .height = HALF_HEIGHT * 2.0, .depth = d_box);
        Babylon_Mesh m = babylon_createBox(g_engine, bopts);
        Babylon_Mesh_set_material(m, palette[layer % 3]);
        babylon_addToScene(scene, js_value(m));
        g_plank_pos[idx] = Babylon_Mesh_get_position(m);
        g_plank_rot[idx] = Babylon_Mesh_get_rotationQuaternion(m);
        js_retain(g_plank_pos[idx]);
        js_retain(g_plank_rot[idx]);
    }

    Babylon_SphereOptions sopts = Babylon_SphereOptions_create(.diameter = SPHERE_RADIUS * 2.0, .segments = 24.0);
    Babylon_Mesh sm = babylon_createSphere(g_engine, sopts);
    Babylon_Mesh_set_material(sm, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.22));
    babylon_addToScene(scene, js_value(sm));
    g_striker_pos = Babylon_Mesh_get_position(sm);
    g_striker_rot = Babylon_Mesh_get_rotationQuaternion(sm);
    js_retain(g_striker_pos);
    js_retain(g_striker_rot);
    g_has_meshes = true;

    sync_meshes();

    babylon_onBeforeRender(scene, on_before_render);
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

    build_world();
    for (int i = 0; i < 18; i++) {
        Rapier_World_step(g_world);
        g_step_count++;
    }
    update_meta();

    static const char* ids[3] = { "btn-launch", "btn-pause", "btn-reset" };
    for (int i = 0; i < 3; i++) {
        HTMLElement b = Document_getElementById(doc, ids[i]);
        HTMLElement_addEventListener_click(b, HTMLElement_PointerEvent_fn_from(on_btn, (void*)&BTN_TAGS[i]));
    }

    g_canvas = Document_querySelector_canvas(doc);
    if (js_is_null(g_canvas)) {
        set_status_text("WebGPU / Babylon Lite error: missing <canvas> element");
        return;
    }
    js_retain(g_canvas);
    Promise_then(babylon_createEngine(g_canvas), on_engine, 0);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Promise_then(rapier_init(), on_rapier_ready, 0);
}
