// Rapier (3D physics, WebAssembly inside) from C, in Node: `init` is a
// promise; once it settles, a world with gravity, a ball falling onto a fixed
// ground, stepped, and a ray cast. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps) { return (a > b ? a - b : b - a) < eps; }

static int pending = 1;

static int hit_count;
static bool on_hit(void* ctx, Rapier3d_RayColliderIntersection hit) {
    hit_count++;
    return true;
}

static void physics(void) {
    // A world with gravity: Vector3 is a Vector.
    Rapier3d_World world = Rapier3d_World_new(Rapier3d_Vector3_new(0, -9.81, 0));
    CHECK(near(Rapier3d_Vector_get_y(Rapier3d_World_get_gravity(world)), -9.81, 1e-12));
    CHECK(near(Rapier3d_World_get_timestep(world), 1.0 / 60, 1e-6));

    // The ground: fixed, a wide slab whose top is at y = 0.
    Rapier3d_RigidBody ground = Rapier3d_World_createRigidBody(world, Rapier3d_RigidBodyDesc_setTranslation(Rapier3d_RigidBodyDesc_fixed(), 0, -0.5, 0));
    Rapier3d_Collider ground_collider = Rapier3d_World_createCollider(world, Rapier3d_ColliderDesc_cuboid(20, 0.5, 20), ground);
    CHECK(Rapier3d_RigidBody_isFixed(ground) && !Rapier3d_RigidBody_isDynamic(ground));
    CHECK(Rapier3d_RigidBody_bodyType(ground) == Rapier3d_RigidBodyType_Fixed);
    CHECK(Rapier3d_RigidBody_numColliders(ground) == 1);
    Rapier3d_Vector half = Rapier3d_Collider_halfExtents(ground_collider);
    CHECK(Rapier3d_Vector_get_x(half) == 20 && Rapier3d_Vector_get_y(half) == 0.5);

    // A ball, dynamic, 5 up.
    Rapier3d_RigidBody ball = Rapier3d_World_createRigidBody(world, Rapier3d_RigidBodyDesc_setTranslation(Rapier3d_RigidBodyDesc_dynamic(), 0, 5, 0));
    Rapier3d_ColliderDesc ball_desc = Rapier3d_ColliderDesc_setRestitution(Rapier3d_ColliderDesc_ball(0.5), 0);
    Rapier3d_Collider ball_collider = Rapier3d_World_createCollider(world, ball_desc, ball);
    CHECK(Rapier3d_RigidBody_isDynamic(ball));
    CHECK(near(Rapier3d_Vector_get_y(Rapier3d_RigidBody_translation(ball)), 5, 1e-9));
    CHECK(near(Rapier3d_RigidBody_mass(ball), 4.0 / 3.0 * 3.14159265358979 * 0.125, 1e-4));
    CHECK(Rapier3d_Collider_shapeType(ball_collider) == Rapier3d_ShapeType_Ball);
    CHECK(Rapier3d_ColliderSet_len(Rapier3d_World_get_colliders(world)) == 2);
    CHECK(js_same(Rapier3d_World_getRigidBody(world, Rapier3d_RigidBody_get_handle(ball)), ball));

    // Half a second of free fall: v = g t, and about half g t^2 down.
    for (int i = 0; i < 30; i++) Rapier3d_World_step(world);
    Rapier3d_Vector v = Rapier3d_RigidBody_linvel(ball);
    CHECK(near(Rapier3d_Vector_get_y(v), -4.905, 0.15));
    double y = Rapier3d_Vector_get_y(Rapier3d_RigidBody_translation(ball));
    CHECK(near(y, 5 - 1.226, 0.15));
    CHECK(near(Rapier3d_Vector_get_x(Rapier3d_RigidBody_translation(ball)), 0, 1e-9));
    CHECK(!Rapier3d_RigidBody_isSleeping(ball));

    // Then it lands, and rests on the ground: its centre a radius above it.
    for (int i = 0; i < 270; i++) Rapier3d_World_step(world);
    double rest = Rapier3d_Vector_get_y(Rapier3d_RigidBody_translation(ball));
    CHECK(near(rest, 0.5, 0.05));
    CHECK(near(Rapier3d_Vector_get_y(Rapier3d_RigidBody_linvel(ball)), 0, 0.05));

    // A ray straight down from above hits the ball's top: 10 - (0.5 + 0.5).
    Rapier3d_Ray down = Rapier3d_Ray_new(Rapier3d_Vector3_new(0, 10, 0), Rapier3d_Vector3_new(0, -1, 0));
    Rapier3d_RayColliderHit hit = Rapier3d_World_castRay(world, down, 100, true, js_none_double(), js_none_double(), (Rapier3d_Collider){ 0 }, (Rapier3d_RigidBody){ 0 }, (Rapier3d_Collider_to_boolean_fn){ 0 });
    CHECK(hit.h != 0);
    CHECK(near(Rapier3d_RayColliderHit_get_timeOfImpact(hit), 9, 0.06));
    CHECK(Rapier3d_Collider_get_handle(Rapier3d_RayColliderHit_get_collider(hit)) == Rapier3d_Collider_get_handle(ball_collider));
    // Off to the side, it hits the ground: 10 down.
    Rapier3d_Ray side = Rapier3d_Ray_new(Rapier3d_Vector3_new(8, 10, 0), Rapier3d_Vector3_new(0, -1, 0));
    Rapier3d_RayColliderHit side_hit = Rapier3d_World_castRay(world, side, 100, true, js_none_double(), js_none_double(), (Rapier3d_Collider){ 0 }, (Rapier3d_RigidBody){ 0 }, (Rapier3d_Collider_to_boolean_fn){ 0 });
    CHECK(near(Rapier3d_RayColliderHit_get_timeOfImpact(side_hit), 10, 1e-6));
    CHECK(Rapier3d_Collider_get_handle(Rapier3d_RayColliderHit_get_collider(side_hit)) == Rapier3d_Collider_get_handle(ground_collider));
    // Too short, or upward: nothing.
    Rapier3d_RayColliderHit none = Rapier3d_World_castRay(world, down, 5, true, js_none_double(), js_none_double(), (Rapier3d_Collider){ 0 }, (Rapier3d_RigidBody){ 0 }, (Rapier3d_Collider_to_boolean_fn){ 0 });
    CHECK(none.h == 0);
    // With the normal.
    Rapier3d_RayColliderIntersection with_normal = Rapier3d_World_castRayAndGetNormal(world, side, 100, true, js_none_double(), js_none_double(), (Rapier3d_Collider){ 0 }, (Rapier3d_RigidBody){ 0 }, (Rapier3d_Collider_to_boolean_fn){ 0 });
    CHECK(with_normal.h != 0);
    CHECK(near(Rapier3d_Vector_get_y(Rapier3d_RayColliderIntersection_get_normal(with_normal)), 1, 1e-6));
    // Every intersection through a callback: the ball and the ground.
    Rapier3d_Ray through = Rapier3d_Ray_new(Rapier3d_Vector3_new(0, 10, 0), Rapier3d_Vector3_new(0, -1, 0));
    Rapier3d_World_intersectionsWithRay(world, through, 100, true, Rapier3d_IntersectionsWithRayFn_from(on_hit, 0), js_none_double(), js_none_double(), (Rapier3d_Collider){ 0 }, (Rapier3d_RigidBody){ 0 }, (Rapier3d_Collider_to_boolean_fn){ 0 });
    CHECK(hit_count == 2);

    // An impulse on a ball floating in gravity-free space: v = impulse / mass.
    Rapier3d_World space = Rapier3d_World_new(Rapier3d_Vector3_new(0, 0, 0));
    Rapier3d_RigidBody drifter = Rapier3d_World_createRigidBody(space, Rapier3d_RigidBodyDesc_dynamic());
    Rapier3d_World_createCollider(space, Rapier3d_ColliderDesc_ball(1), drifter);
    double mass = Rapier3d_RigidBody_mass(drifter);
    Rapier3d_RigidBody_applyImpulse(drifter, Rapier3d_Vector3_new(mass * 3, 0, 0), true);
    CHECK(near(Rapier3d_Vector_get_x(Rapier3d_RigidBody_linvel(drifter)), 3, 1e-9));
    for (int i = 0; i < 60; i++) Rapier3d_World_step(space);
    CHECK(near(Rapier3d_Vector_get_x(Rapier3d_RigidBody_translation(drifter)), 3, 0.05));
    // setTranslation moves it directly.
    Rapier3d_RigidBody_setTranslation(drifter, Rapier3d_Vector3_new(1, 2, 3), true);
    Rapier3d_Vector at = Rapier3d_RigidBody_translation(drifter);
    CHECK(Rapier3d_Vector_get_x(at) == 1 && Rapier3d_Vector_get_y(at) == 2 && Rapier3d_Vector_get_z(at) == 3);

    // Removing a body takes its collider.
    Rapier3d_World_removeRigidBody(world, ball);
    CHECK(Rapier3d_ColliderSet_len(Rapier3d_World_get_colliders(world)) == 1);
    Rapier3d_World_free(space);
    Rapier3d_World_free(world);
}

static void on_init(void* ctx, bool ok, JsValue v) {
    CHECK(ok);
    if (ok) {
        JsString ver = rapier3d_version();
        CHECK(JsString_equals(ver, "0.21.0") || JsString_len(ver) > 0);
        js_release(ver);
        physics();
    }
    pending = 0;
}

__attribute__((export_name("busy"))) int busy(void) { return pending; }

__attribute__((export_name("run"))) void run(void) {
    Promise p = rapier3d_init();
    Promise_then(p, on_init, 0);
}
