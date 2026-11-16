// Rapier (3D physics, WebAssembly inside) from C, in Node: `init` is a
// promise; once it settles, a world with gravity, a ball falling onto a fixed
// ground, stepped, and a ray cast. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps) { return (a > b ? a - b : b - a) < eps; }

static int pending = 1;

static int hit_count;
static bool on_hit(void* ctx, RayColliderIntersection hit) {
    hit_count++;
    return true;
}

static void physics(void) {
    // A world with gravity: Vector3 is a Vector.
    World world = World_new(Vector3_new(0, -9.81, 0));
    CHECK(near(Vector_get_y(World_get_gravity(world)), -9.81, 1e-12));
    CHECK(near(World_get_timestep(world), 1.0 / 60, 1e-6));

    // The ground: fixed, a wide slab whose top is at y = 0.
    RigidBody ground = World_createRigidBody(world, RigidBodyDesc_setTranslation(RigidBodyDesc_fixed(), 0, -0.5, 0));
    Collider ground_collider = World_createCollider(world, ColliderDesc_cuboid(20, 0.5, 20), ground);
    CHECK(RigidBody_isFixed(ground) && !RigidBody_isDynamic(ground));
    CHECK(RigidBody_bodyType(ground) == RigidBodyType_Fixed);
    CHECK(RigidBody_numColliders(ground) == 1);
    Vector half = Collider_halfExtents(ground_collider);
    CHECK(Vector_get_x(half) == 20 && Vector_get_y(half) == 0.5);

    // A ball, dynamic, 5 up.
    RigidBody ball = World_createRigidBody(world, RigidBodyDesc_setTranslation(RigidBodyDesc_dynamic(), 0, 5, 0));
    ColliderDesc ball_desc = ColliderDesc_setRestitution(ColliderDesc_ball(0.5), 0);
    Collider ball_collider = World_createCollider(world, ball_desc, ball);
    CHECK(RigidBody_isDynamic(ball));
    CHECK(near(Vector_get_y(RigidBody_translation(ball)), 5, 1e-9));
    CHECK(near(RigidBody_mass(ball), 4.0 / 3.0 * 3.14159265358979 * 0.125, 1e-4));
    CHECK(Collider_shapeType(ball_collider) == ShapeType_Ball);
    CHECK(ColliderSet_len(World_get_colliders(world)) == 2);
    CHECK(js_same(World_getRigidBody(world, RigidBody_get_handle(ball)), ball));

    // Half a second of free fall: v = g t, and about half g t^2 down.
    for (int i = 0; i < 30; i++) World_step(world);
    Vector v = RigidBody_linvel(ball);
    CHECK(near(Vector_get_y(v), -4.905, 0.15));
    double y = Vector_get_y(RigidBody_translation(ball));
    CHECK(near(y, 5 - 1.226, 0.15));
    CHECK(near(Vector_get_x(RigidBody_translation(ball)), 0, 1e-9));
    CHECK(!RigidBody_isSleeping(ball));

    // Then it lands, and rests on the ground: its centre a radius above it.
    for (int i = 0; i < 270; i++) World_step(world);
    double rest = Vector_get_y(RigidBody_translation(ball));
    CHECK(near(rest, 0.5, 0.05));
    CHECK(near(Vector_get_y(RigidBody_linvel(ball)), 0, 0.05));

    // A ray straight down from above hits the ball's top: 10 - (0.5 + 0.5).
    Ray down = Ray_new(Vector3_new(0, 10, 0), Vector3_new(0, -1, 0));
    RayColliderHit hit = World_castRay(world, down, 100, true, js_none_double(), js_none_double(), (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
    CHECK(hit.h != 0);
    CHECK(near(RayColliderHit_get_timeOfImpact(hit), 9, 0.06));
    CHECK(Collider_get_handle(RayColliderHit_get_collider(hit)) == Collider_get_handle(ball_collider));
    // Off to the side, it hits the ground: 10 down.
    Ray side = Ray_new(Vector3_new(8, 10, 0), Vector3_new(0, -1, 0));
    RayColliderHit side_hit = World_castRay(world, side, 100, true, js_none_double(), js_none_double(), (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
    CHECK(near(RayColliderHit_get_timeOfImpact(side_hit), 10, 1e-6));
    CHECK(Collider_get_handle(RayColliderHit_get_collider(side_hit)) == Collider_get_handle(ground_collider));
    // Too short, or upward: nothing.
    RayColliderHit none = World_castRay(world, down, 5, true, js_none_double(), js_none_double(), (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
    CHECK(none.h == 0);
    // With the normal.
    RayColliderIntersection with_normal = World_castRayAndGetNormal(world, side, 100, true, js_none_double(), js_none_double(), (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
    CHECK(with_normal.h != 0);
    CHECK(near(Vector_get_y(RayColliderIntersection_get_normal(with_normal)), 1, 1e-6));
    // Every intersection through a callback: the ball and the ground.
    Ray through = Ray_new(Vector3_new(0, 10, 0), Vector3_new(0, -1, 0));
    World_intersectionsWithRay(world, through, 100, true, RayColliderIntersection_to_boolean_fn_from(on_hit, 0), js_none_double(), js_none_double(), (Collider){ 0 }, (RigidBody){ 0 }, (Collider_to_boolean_fn){ 0 });
    CHECK(hit_count == 2);

    // An impulse on a ball floating in gravity-free space: v = impulse / mass.
    World space = World_new(Vector3_new(0, 0, 0));
    RigidBody drifter = World_createRigidBody(space, RigidBodyDesc_dynamic());
    World_createCollider(space, ColliderDesc_ball(1), drifter);
    double mass = RigidBody_mass(drifter);
    RigidBody_applyImpulse(drifter, Vector3_new(mass * 3, 0, 0), true);
    CHECK(near(Vector_get_x(RigidBody_linvel(drifter)), 3, 1e-9));
    for (int i = 0; i < 60; i++) World_step(space);
    CHECK(near(Vector_get_x(RigidBody_translation(drifter)), 3, 0.05));
    // setTranslation moves it directly.
    RigidBody_setTranslation(drifter, Vector3_new(1, 2, 3), true);
    Vector at = RigidBody_translation(drifter);
    CHECK(Vector_get_x(at) == 1 && Vector_get_y(at) == 2 && Vector_get_z(at) == 3);

    // Removing a body takes its collider.
    World_removeRigidBody(world, ball);
    CHECK(ColliderSet_len(World_get_colliders(world)) == 1);
    World_free(space);
    World_free(world);
}

static void on_init(void* ctx, bool ok, JsValue v) {
    CHECK(ok);
    if (ok) {
        JsString ver = version();
        CHECK(JsString_equals(ver, "0.21.0") || JsString_len(ver) > 0);
        js_release(ver);
        physics();
    }
    pending = 0;
}

__attribute__((export_name("busy"))) int busy(void) { return pending; }

__attribute__((export_name("run"))) void run(void) {
    Promise p = init();
    Promise_then(p, on_init, 0);
}
