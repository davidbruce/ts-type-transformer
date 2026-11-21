// Rapier (3D) from F#, in Node: a ball falls onto a fixed ground in a World,
// stepped by hand, and rays are cast at it. `init` is a promise: the checks
// run once it settles. Every check must hold.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

module Async =
    let map f a = async { let! x = a in return f x }

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let near (a: float) (b: float) (eps: float) = abs (a - b) < eps

async {
    do! Async.AwaitPromise (init ())
    check (version () <> "") "version"

    // A world with gravity, a fixed ground and a ball above it.
    let world = World.Create (Vector.Create (0.0, -9.81, 0.0))
    check (world.gravity.y = -9.81) "gravity"
    check (world.timestep > 0.0) "timestep"

    let ground = world.createRigidBody (RigidBodyDesc.``fixed``().setTranslation (0.0, 0.0, 0.0))
    check (ground.isFixed () && not (ground.isDynamic ())) "a fixed body"
    let groundShape = world.createCollider (ColliderDesc.cuboid(10.0, 0.5, 10.0), ground)
    check (groundShape.parent().Value.handle = ground.handle) "the collider's parent"

    let ball = world.createRigidBody (RigidBodyDesc.dynamic().setTranslation (0.0, 5.0, 0.0))
    check (ball.isDynamic () && ball.bodyType () = RigidBodyType.Dynamic) "a dynamic body"
    let ballShape = world.createCollider (ColliderDesc.ball(0.5).setRestitution(0.0).setFriction (0.5), ball)
    check (near (ballShape.radius ()) 0.5 1e-6) "ball radius"
    check (near (ballShape.restitution ()) 0.0 1e-6 && near (ballShape.friction ()) 0.5 1e-6) "restitution, friction"
    check (ball.numColliders () = 1.0) "numColliders"
    check (world.bodies.len () = 2.0 && world.colliders.len () = 2.0) "two bodies and two colliders"
    check (near (ball.mass ()) (4.0 / 3.0 * System.Math.PI * 0.125) 1e-3) "mass from the default density"

    // Before any step: where it was put, at rest.
    let start = ball.translation ()
    check (start.x = 0.0 && start.y = 5.0 && start.z = 0.0) "the start position"
    check ((ball.linvel ()).y = 0.0) "at rest"

    // One step: it has begun to fall.
    world.step ()
    check ((ball.translation ()).y < 5.0) "one step: falling"
    check ((ball.linvel ()).y < 0.0) "one step: velocity downward"

    // Falling: after half a second, near y = 5 - g t^2 / 2.
    for _ in 1..29 do world.step ()
    let y = (ball.translation ()).y
    check (y < 4.0 && y > 3.5) "half a second of falling"
    check (near (ball.linvel ()).y (-9.81 * 0.5) 0.2) "velocity is about g t"

    // Landed: resting on the ground, whose top is at y = 0.5, so centre at 1.0.
    for _ in 1..200 do world.step ()
    let rest = ball.translation ()
    check (near rest.y 1.0 0.05) "resting on the ground"
    check (near rest.x 0.0 1e-3 && near rest.z 0.0 1e-3) "no sideways drift"
    check (abs ((ball.linvel ()).y) < 0.1) "at rest again"
    check ((ground.translation ()).y = 0.0) "the ground did not move"

    // Forces and impulses move a body.
    ball.applyImpulse (Vector.Create (2.0, 0.0, 0.0), true)
    check ((ball.linvel ()).x > 0.0) "applyImpulse"
    ball.setLinvel (Vector.Create (0.0, 0.0, 0.0), true)
    ball.setTranslation (Vector.Create (3.0, 4.0, 0.0), true)
    let moved = ball.translation ()
    check (moved.x = 3.0 && moved.y = 4.0) "setTranslation"

    // A ray cast: straight down from above the ground, away from the ball.
    ball.setTranslation (Vector.Create (0.0, 20.0, 0.0), true)
    world.step ()
    let ray = Ray.Create (Vector.Create (5.0, 10.0, 0.0), Vector.Create (0.0, -1.0, 0.0))
    check (near (ray.pointAt 2.0).y 8.0 1e-6) "Ray.pointAt"
    let hit = world.castRay (ray, 100.0, true)
    check hit.IsSome "the ray hits"
    check (near hit.Value.timeOfImpact 9.5 1e-3) "time of impact: the top of the ground"
    check (hit.Value.collider.handle = groundShape.handle) "the collider hit"
    let miss = world.castRay (Ray.Create (Vector.Create (50.0, 10.0, 0.0), Vector.Create (0.0, -1.0, 0.0)), 100.0, true)
    check miss.IsNone "a ray that misses"
    let toi = groundShape.castRayAndGetNormal (ray, 100.0, true)
    check (near toi.Value.normal.y 1.0 1e-6) "the normal of the ground's top"

    // Removing a body.
    world.removeRigidBody ball
    check (world.bodies.len () = 1.0) "removeRigidBody"
    world.free ()

    printfn "%d checks passed" passed
}
|> Async.Catch
|> Async.map (function Choice2Of2 e -> printfn "FAILED: %s" e.Message; raise e | _ -> ())
|> Async.StartImmediate
