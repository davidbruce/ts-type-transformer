// Rapier (3D, compat build) from Zig, in Node: a ball dropped onto a fixed ground, stepped, and a ray cast.
// init() is async; the runner polls busy() and then calls finish(). Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn near(a: f64, c: f64, eps: f64) bool {
    return @abs(a - c) <= eps;
}

fn vec(x: f64, y: f64, z: f64) b.Vector {
    const v = b.Vector.create();
    v.setX(x);
    v.setY(y);
    v.setZ(z);
    return v;
}

var done: bool = false;
var initialised: bool = false;

export fn busy() i32 {
    return @intFromBool(!done);
}

export fn run() void {
    const p = b.init();
    defer b.jsRelease(p);
    p.then(onInit, null);
    // Nothing has resolved yet: init() is a promise.
    expect(!initialised, @src());
}

fn onInit(_: ?*anyopaque, ok: bool, _: b.JsValue) void {
    initialised = true;
    expect(ok, @src());
    if (ok) simulate();
    done = true;
}

fn simulate() void {
    expect(b.version().len() > 0, @src());

    // A world with gravity.
    const gravity = vec(0, -9.81, 0);
    const world = b.World.new(gravity, .{});
    expect(near(world.getGravity().getY(), -9.81, 1e-9), @src());
    expect(near(world.getTimestep(), 1.0 / 60.0, 1e-9), @src());

    // The ground: a fixed body with a wide, flat box (top surface at y = 0.5).
    const ground_desc = b.RigidBodyDesc.fixed();
    const ground = world.createRigidBody(ground_desc);
    _ = world.createCollider(b.ColliderDesc.cuboid(10, 0.5, 10), .{ .parent = ground });
    expect(ground.isFixed() and !ground.isDynamic(), @src());
    expect(ground.bodyType() == .Fixed, @src());

    // A ball: dynamic, dropped from y = 5.
    const ball_desc = b.RigidBodyDesc.dynamic().setTranslation2(0, 5, 0);
    const ball = world.createRigidBody(ball_desc);
    const ball_collider = world.createCollider(b.ColliderDesc.ball(0.5).setRestitution2(0).setDensity2(2), .{ .parent = ball });
    expect(ball.isDynamic() and ball.bodyType() == .Dynamic, @src());
    expect(near(ball.translation(null).getY(), 5, 1e-9), @src());
    expect(near(ball_collider.radius(), 0.5, 1e-9), @src());
    expect(ball.numColliders() == 1, @src());
    expect(ball_collider.parent() != null, @src());
    expect(b.jsSame(ball_collider.parent().?, ball), @src());
    // Density 2 on a ball of radius 0.5: mass = 2 * 4/3 pi r^3.
    expect(near(ball.mass(), 2.0 * 4.0 / 3.0 * std.math.pi * 0.125, 1e-4), @src());

    // Free fall: after half a second it is lower, and moving down.
    var i: u32 = 0;
    while (i < 30) : (i += 1) world.step(.{});
    const falling = ball.translation(null).getY();
    expect(falling < 4.0 and falling > 3.0, @src());
    expect(ball.linvel(null).getY() < -4.0, @src());
    expect(near(ball.translation(null).getX(), 0, 1e-6), @src());

    // It lands on the ground and comes to rest on top of it: y = 0.5 + 0.5.
    while (i < 300) : (i += 1) world.step(.{});
    const rest = ball.translation(null).getY();
    expect(rest > 0.95 and rest < 1.05, @src());
    expect(@abs(ball.linvel(null).getY()) < 0.1, @src());
    expect(near(ground.translation(null).getY(), 0, 1e-9), @src());

    // A ray straight down away from the ball hits the ground top at 9.5 from y = 10.
    const away = b.Ray.new(vec(5, 10, 0), vec(0, -1, 0));
    const hit = world.castRay(away, 100, true, .{});
    expect(hit != null, @src());
    if (hit) |h| {
        expect(near(h.getTimeOfImpact(), 9.5, 1e-4), @src());
        expect(b.jsSame(h.getCollider().parent().?, ground), @src());
    }
    // Above the ball it hits the ball first: its top is at about y = 1.5.
    const over = b.Ray.new(vec(0, 10, 0), vec(0, -1, 0));
    const hit_ball = world.castRay(over, 100, true, .{});
    expect(hit_ball != null and near(hit_ball.?.getTimeOfImpact(), 10 - (rest + 0.5), 1e-3), @src());
    expect(b.jsSame(hit_ball.?.getCollider().parent().?, ball), @src());
    // The point along a ray.
    expect(near(over.pointAt(2).getY(), 8, 1e-9), @src());
    // Too short to reach anything.
    expect(world.castRay(away, 5, true, .{}) == null, @src());
    // The surface normal comes with castRayAndGetNormal.
    const normal_hit = world.castRayAndGetNormal(away, 100, true, .{});
    expect(normal_hit != null and near(normal_hit.?.getNormal().getY(), 1, 1e-6), @src());

    // An impulse kicks it sideways; a kinematic-free check on velocity.
    ball.applyImpulse(vec(2, 0, 0), true);
    expect(ball.linvel(null).getX() > 0.5, @src());
    ball.setLinvel(vec(0, 0, 0), true);
    expect(ball.linvel(null).getX() == 0, @src());

    // setTranslation teleports; removing the body removes its collider.
    ball.setTranslation(vec(3, 7, 0), true);
    expect(near(ball.translation(null).getX(), 3, 1e-9) and near(ball.translation(null).getY(), 7, 1e-9), @src());
    world.removeRigidBody(ball);
    expect(world.getBodies().len() == 1, @src());

    world.free();
}
