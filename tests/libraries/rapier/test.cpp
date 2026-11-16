// Rapier (3D, compat build) from C++, in Node: init is awaited, then a world
// with a ball dropped on a fixed ground, stepped, and rays cast into it.
// Every CHECK must hold.
#include <cmath>

#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool near(double a, double b, double eps = 1e-3) { return std::fabs(a - b) < eps; }

static bool working = true;

static void simulate() {
    CHECK(!version().empty());

    // Gravity as a Vector3, a Vector where one is wanted.
    World world = World::create(Vector3::create(0, -9.81, 0));
    CHECK(near(world.gravity().y(), -9.81));
    CHECK(world.timestep() > 0 && world.timestep() < 0.1);
    CHECK(world.bodies().len() == 0);

    // A fixed ground: a wide slab, its top at y = 0.
    RigidBody ground = world.createRigidBody(RigidBodyDesc::fixed().setTranslation(0, -0.5, 0));
    Collider slab = world.createCollider(ColliderDesc::cuboid(50, 0.5, 50), ground);
    CHECK(ground.isFixed() && !ground.isDynamic());
    CHECK(ground.bodyType() == RigidBodyType::Fixed);
    CHECK(near(ground.translation().y(), -0.5));
    CHECK(slab.shapeType() == ShapeType::Cuboid);
    CHECK(near(slab.halfExtents()->x(), 50));
    CHECK(slab.parent().has_value() && slab.parent()->handle() == ground.handle());

    // A dynamic ball, radius 0.5, dropped from y = 5, bouncy.
    RigidBody ball = world.createRigidBody(RigidBodyDesc::dynamic().setTranslation(0, 5, 0));
    ColliderDesc ballShape = ColliderDesc::ball(0.5).setRestitution(0.0).setFriction(0.7);
    Collider ballCollider = world.createCollider(ballShape, ball);
    CHECK(ball.isDynamic());
    CHECK(near(ball.translation().y(), 5));
    CHECK(near(ballCollider.radius(), 0.5));
    CHECK(near(ballCollider.friction(), 0.7));
    CHECK(ball.numColliders() == 1);
    CHECK(ball.mass() > 0);
    CHECK(world.bodies().len() == 2);
    CHECK(world.colliders().len() == 2);
    CHECK(world.getRigidBody(ball.handle()).handle() == ball.handle());

    // Free fall for 30 steps of 1/60 s: y = 5 - g t^2 / 2 (semi-implicit Euler
    // is close to it), and velocity is about -g t.
    for (int i = 0; i < 30; i++) world.step();
    double t = 30 * world.timestep();
    CHECK(ball.translation().y() < 5);
    CHECK(near(ball.translation().y(), 5 - 0.5 * 9.81 * t * t, 0.2));
    CHECK(near(ball.linvel().y(), -9.81 * t, 0.2));
    CHECK(near(ball.translation().x(), 0) && near(ball.translation().z(), 0));

    // Long enough to land: it rests on the slab, its centre at y = 0.5.
    for (int i = 0; i < 240; i++) world.step();
    CHECK(near(ball.translation().y(), 0.5, 0.05));
    CHECK(std::fabs(ball.linvel().y()) < 0.1);

    // An impulse sends it sideways; setLinvel and setTranslation put it back.
    ball.applyImpulse(Vector3::create(ball.mass() * 2, 0, 0), true);
    CHECK(near(ball.linvel().x(), 2, 0.05));
    ball.setTranslation(Vector3::create(0, 3, 0), true);
    ball.setLinvel(Vector3::create(0, 0, 0), true);
    CHECK(near(ball.translation().y(), 3));
    world.step();
    CHECK(ball.translation().y() < 3);

    // Rays: straight down from above hits the ball first, then the slab.
    world.step();
    Ray down = Ray::create(Vector3::create(0, 10, 0), Vector3::create(0, -1, 0));
    std::optional<RayColliderHit> hit = world.castRay(down, 100, true);
    CHECK(hit.has_value());
    CHECK(hit->collider().handle() == ballCollider.handle());
    CHECK(near(hit->timeOfImpact(), 10 - ball.translation().y() - 0.5, 0.01));
    CHECK(near(down.pointAt(hit->timeOfImpact()).y(), ball.translation().y() + 0.5, 0.01));

    // Away from the ball, the ray finds the ground, with its normal.
    Ray beside = Ray::create(Vector3::create(10, 10, 10), Vector3::create(0, -1, 0));
    std::optional<RayColliderIntersection> ground_hit = world.castRayAndGetNormal(beside, 100, true);
    CHECK(ground_hit.has_value());
    CHECK(ground_hit->collider().handle() == slab.handle());
    CHECK(near(ground_hit->timeOfImpact(), 10));
    CHECK(near(ground_hit->normal().y(), 1));
    // Too short to reach it, or aimed at the sky: nothing.
    CHECK(!world.castRay(beside, 5, true).has_value());
    CHECK(!world.castRay(Ray::create(Vector3::create(10, 10, 10), Vector3::create(0, 1, 0)), 100, true).has_value());

    // Gravity scale zero: the body floats.
    RigidBody floater = world.createRigidBody(RigidBodyDesc::dynamic().setTranslation(20, 8, 0).setGravityScale(0));
    world.createCollider(ColliderDesc::ball(0.25), floater);
    for (int i = 0; i < 20; i++) world.step();
    CHECK(near(floater.translation().y(), 8, 1e-6));

    // Removing a body takes its colliders along.
    world.removeRigidBody(floater);
    CHECK(world.bodies().len() == 2);
    CHECK(world.colliders().len() == 2);
}

// The WebAssembly of the library loads asynchronously.
static ts::Task start() {
    ts::Result<void> r = co_await init();
    CHECK(bool(r));
    if (r) simulate();
    working = false;
}

extern "C" __attribute__((export_name("busy"))) int busy() { return working; }

extern "C" __attribute__((export_name("run"))) void run() {
    start();
}
