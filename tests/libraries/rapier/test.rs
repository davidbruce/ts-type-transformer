// Rapier (3D physics) from Rust, in Node: `init()` loads the engine
// asynchronously, then a world with gravity, a ball falling onto a fixed
// ground, stepped, and a ray cast. Every check! must hold.
#![no_main]
use bindings::*;
use std::cell::Cell;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

thread_local! {
    static DONE: Cell<bool> = const { Cell::new(false) };
}

// Nonzero until the async work is done: the runner waits for it.
#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    !DONE.with(Cell::get) as i32
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() < tol
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    js::spawn(async {
        let ready = init().await;
        check!(ready.is_ok());
        if ready.is_ok() {
            physics();
        }
        DONE.with(|d| d.set(true));
    });
}

fn physics() {
    check!(!version().is_empty());

    // Gravity down, and a world made from it.
    let gravity = Vector3::new(0.0, -9.81, 0.0);
    let world = World::new(&gravity, None, None, None, None, None, None, None, None, None, None, None, None, None);
    check!(near(world.gravity().y(), -9.81, 1e-9));
    check!(near(world.timestep(), 1.0 / 60.0, 1e-9));

    // A fixed ground slab: a body at y = 0 with a 10 x 0.5 x 10 cuboid.
    let ground = world.create_rigid_body(&RigidBodyDesc::new_static());
    world.create_collider(&ColliderDesc::cuboid(10.0, 0.5, 10.0), Some(&ground));
    check!(ground.is_fixed() && !ground.is_dynamic());

    // A dynamic ball of radius 0.5, dropped from y = 5, resting on the slab
    // (whose top is at y = 0.5) when its centre is at y = 1.
    let desc = RigidBodyDesc::new_dynamic();
    desc.set_translation(0.0, 5.0, 0.0);
    let ball = world.create_rigid_body(&desc);
    let collider = world.create_collider(&ColliderDesc::ball(0.5).set_restitution(0.0).set_density(1.0), Some(&ball));
    check!(ball.is_dynamic() && !ball.is_fixed());
    check!(ball.translation(None).y() == 5.0);
    check!(collider.handle() == world.get_collider(collider.handle()).handle());
    check!(ball.mass() > 0.0);
    check!(world.bodies().len() == 2.0 && world.colliders().len() == 2.0);

    // The first step moves it down; the speed is downward.
    world.step(None, None);
    let after_one = ball.translation(None).y();
    check!(after_one < 5.0);
    check!(ball.linvel(None).y() < 0.0);
    // Falling for 0.5 s of free fall takes it about 1.2 m: 5 - 0.5*g*t^2.
    for _ in 0..29 {
        world.step(None, None);
    }
    let y = ball.translation(None).y();
    check!(y < after_one && near(y, 5.0 - 0.5 * 9.81 * 0.25, 0.3));
    check!(near(ball.translation(None).x(), 0.0, 1e-6) && near(ball.translation(None).z(), 0.0, 1e-6));

    // Then it lands and rests on the ground.
    for _ in 0..300 {
        world.step(None, None);
    }
    let rest = ball.translation(None).y();
    check!(near(rest, 1.0, 0.05));
    check!(near(ball.linvel(None).y(), 0.0, 0.05));
    check!(ground.translation(None).y() == 0.0);

    // A ray cast straight down from y = 10 hits the ball first, 9 - 0.5 = 8.5 away.
    let ray = Ray::new(&Vector3::new(0.0, 10.0, 0.0), &Vector3::new(0.0, -1.0, 0.0));
    let hit = world.cast_ray(&ray, 100.0, true, None, None, None, None, None);
    check!(hit.as_ref().is_some_and(|h| near(h.time_of_impact(), 10.0 - rest - 0.5, 0.06)));
    check!(hit.as_ref().is_some_and(|h| h.collider().handle() == collider.handle()));
    check!(near(ray.point_at(2.0).y(), 8.0, 1e-9));
    // Excluding the ball, the ray reaches the slab's top at 0.5.
    let ground_hit = world.cast_ray(&ray, 100.0, true, None, None, Some(&collider), None, None);
    check!(ground_hit.is_some_and(|h| near(h.time_of_impact(), 9.5, 1e-6)));
    // A ray aimed away, or too short, hits nothing.
    check!(world.cast_ray(&Ray::new(&Vector3::new(0.0, 10.0, 0.0), &Vector3::new(0.0, 1.0, 0.0)), 100.0, true, None, None, None, None, None).is_none());
    check!(world.cast_ray(&ray, 1.0, true, None, None, None, None, None).is_none());

    // An impulse sets the ball moving sideways.
    ball.apply_impulse(&Vector3::new(1.0, 0.0, 0.0), true);
    check!(ball.linvel(None).x() > 0.0);
    world.step(None, None);
    check!(ball.translation(None).x() > 0.0);

    // Removing the ball leaves one body.
    world.remove_rigid_body(&ball);
    check!(world.bodies().len() == 1.0);
    world.free();
}
