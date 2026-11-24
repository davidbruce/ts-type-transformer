// Rapier (3D, WebAssembly build) from Gleam, in Node: init is awaited, then a
// world with gravity, bodies falling onto a fixed ground, stepped, with rays
// and collision events. Every check must hold.
import bindings as rapier
import bindings/collider
import bindings/collider_desc
import bindings/event_queue
import bindings/ray
import bindings/ray_collider_hit
import bindings/rigid_body
import bindings/rigid_body_desc
import bindings/runtime
import bindings/types as t
import bindings/vector
import bindings/world
import cell
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float, within: Float) -> Bool {
  float.absolute_value(a -. b) <. within
}

// A Vector is an interface: made empty, then given its coordinates.
fn vec(x: Float, y: Float, z: Float) {
  let v = vector.create()
  vector.set_x(v, x)
  vector.set_y(v, y)
  vector.set_z(v, z)
  v
}

pub fn main() {
  runtime.promise_then(rapier.init(), fn(ready) {
    let assert Ok(_) = ready
    let passed = run()
    io.println(int.to_string(passed) <> " checks passed")
  })
}

fn run() -> Int {
  let passed = check(string.length(rapier.version()) > 0, "a version")

  // A world with gravity.
  let w = world.new(vec(0.0, -9.81, 0.0))
  let gravity = world.get_gravity(w)
  let passed =
    passed
    + check(
      vector.get_y(gravity) == -9.81 && vector.get_x(gravity) == 0.0,
      "gravity",
    )
    + check(
      near(world.get_timestep(w), 1.0 /. 60.0, 0.000001),
      "default timestep",
    )

  // A fixed ground: a wide slab with its top at y = 0.5.
  let ground =
    world.create_rigid_body(
      w,
      rigid_body_desc.set_translation(rigid_body_desc.fixed(), 0.0, 0.0, 0.0),
    )
  let ground_collider =
    world.create_collider_with(
      w,
      collider_desc.cuboid(10.0, 0.5, 10.0),
      Some(ground),
    )
  let passed =
    passed
    + check(rigid_body.is_fixed(ground), "the ground is fixed")
    + check(rigid_body.body_type(ground) == t.RigidBodyTypeFixed, "body type")
    + check(rigid_body.num_colliders(ground) == 1.0, "one collider")
    + check(
      collider.shape_type(ground_collider)
        == collider.shape_type(world.get_collider(
        w,
        collider.get_handle(ground_collider),
      )),
      "get_collider by handle",
    )

  // A dynamic box, dropped from y = 5, with collision events on.
  let box_desc =
    rigid_body_desc.set_translation(rigid_body_desc.dynamic(), 0.0, 5.0, 0.0)
  let box_body = world.create_rigid_body(w, box_desc)
  let box_shape =
    collider_desc.cuboid(0.5, 0.5, 0.5)
    |> collider_desc.set_active_events(t.ActiveEventsCollisionEvents)
  let box_collider = world.create_collider_with(w, box_shape, Some(box_body))
  let passed =
    passed
    + check(rigid_body.is_dynamic(box_body), "the box is dynamic")
    + check(
      near(vector.get_y(rigid_body.translation(box_body)), 5.0, 0.000001),
      "starts at y = 5",
    )
    + check(
      near(rigid_body.mass(box_body), 1.0, 0.000001),
      "mass from the default density",
    )
    + check(
      near(
        collider.half_extents(box_collider)
          |> option.map(vector.get_x)
          |> option.unwrap(0.0),
        0.5,
        0.000001,
      ),
      "half extents",
    )
    + check(
      rigid_body.num_colliders(box_body) == 1.0,
      "the box has its collider",
    )
    + check(!rigid_body.is_sleeping(box_body), "awake")

  // Free fall: after half a second it has fallen about g t^2 / 2 = 1.2.
  let queue = event_queue.new(True)
  let started = cell.new([])
  let step_with_events = fn() {
    world.step_with(w, Some(queue), None)
    event_queue.drain_collision_events(queue, fn(a, b, began) {
      cell.set(started, [#(a, b, began), ..cell.get(started)])
    })
  }
  repeat(30, step_with_events)
  let y_half = vector.get_y(rigid_body.translation(box_body))
  let vy = vector.get_y(rigid_body.linvel(box_body))
  let passed =
    passed
    + check(y_half <. 5.0 && y_half >. 3.0, "falling")
    + check(near(vy, -9.81 *. 0.5, 0.2), "velocity after half a second")
    + check(cell.get(started) == [], "no contact yet")

  // After two more seconds it rests on the ground: the box's centre is at 1.0.
  repeat(120, step_with_events)
  let rest = rigid_body.translation(box_body)
  let passed =
    passed
    + check(near(vector.get_y(rest), 1.0, 0.02), "the box rests on the ground")
    + check(
      near(vector.get_x(rest), 0.0, 0.05) && near(vector.get_z(rest), 0.0, 0.05),
      "it fell straight down",
    )
    + check(
      near(vector.get_y(rigid_body.linvel(box_body)), 0.0, 0.05),
      "and is at rest",
    )
    + check(list.any(cell.get(started), fn(e) { e.2 }), "a contact began")
    + check(
      list.any(cell.get(started), fn(e) {
        let #(a, b, _) = e
        let ids = [a, b]
        list.contains(ids, collider.get_handle(box_collider))
        && list.contains(ids, collider.get_handle(ground_collider))
      }),
      "between the box and the ground",
    )

  // Rays: straight down onto the box, then onto the bare ground.
  let down = ray.new(vec(0.0, 10.0, 0.0), vec(0.0, -1.0, 0.0))
  let assert Some(hit) = world.cast_ray(w, down, 100.0, True)
  let passed =
    passed
    + check(
      near(ray_collider_hit.get_time_of_impact(hit), 8.5, 0.05),
      "ray hits the box's top",
    )
    + check(
      collider.get_handle(ray_collider_hit.get_collider(hit))
        == collider.get_handle(box_collider),
      "the ray's collider is the box's",
    )
    + check(near(vector.get_y(ray.point_at(down, 8.5)), 1.5, 0.001), "point_at")
  let aside = ray.new(vec(5.0, 10.0, 0.0), vec(0.0, -1.0, 0.0))
  let assert Some(hit2) = world.cast_ray(w, aside, 100.0, True)
  let passed =
    passed
    + check(
      near(ray_collider_hit.get_time_of_impact(hit2), 9.5, 0.001),
      "ray hits the ground",
    )
    + check(
      collider.get_handle(ray_collider_hit.get_collider(hit2))
        == collider.get_handle(ground_collider),
      "the ground's collider",
    )
    + check(
      world.cast_ray(
        w,
        ray.new(vec(0.0, 10.0, 0.0), vec(0.0, 1.0, 0.0)),
        100.0,
        True,
      )
        == None,
      "a ray away from everything",
    )
    + check(world.cast_ray(w, down, 3.0, True) == None, "out of range")

  // An impulse sends the box up; the ground is not affected by gravity.
  rigid_body.apply_impulse(box_body, vec(0.0, 10.0, 0.0), True)
  let up = vector.get_y(rigid_body.linvel(box_body))
  repeat(10, step_with_events)
  let passed =
    passed
    + check(
      near(up, 10.0, 0.01),
      "an impulse of 10 on a mass of 1 gives a velocity of 10",
    )
    + check(
      vector.get_y(rigid_body.translation(box_body)) >. 1.5,
      "the box rises",
    )
    + check(
      near(vector.get_y(rigid_body.translation(ground)), 0.0, 0.000001),
      "the ground does not move",
    )

  // Moved by hand.
  rigid_body.set_translation(box_body, vec(0.0, 20.0, 0.0), True)
  let passed =
    passed
    + check(
      near(vector.get_y(rigid_body.translation(box_body)), 20.0, 0.000001),
      "set_translation",
    )

  // Bodies and colliders are counted by the world's sets.
  let count = cell.new(0)
  world.for_each_rigid_body(w, fn(_) { cell.set(count, cell.get(count) + 1) })
  let passed = passed + check(cell.get(count) == 2, "for_each_rigid_body")
  world.remove_rigid_body(w, box_body)
  let count2 = cell.new(0)
  world.for_each_rigid_body(w, fn(_) { cell.set(count2, cell.get(count2) + 1) })
  let passed = passed + check(cell.get(count2) == 1, "remove_rigid_body")
  world.free(w)
  passed
}

fn repeat(n: Int, f: fn() -> Nil) -> Nil {
  case n {
    0 -> Nil
    _ -> {
      f()
      repeat(n - 1, f)
    }
  }
}
