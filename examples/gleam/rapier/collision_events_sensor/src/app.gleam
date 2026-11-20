import dimforge_rapier3d_compat/areas/main as rapier
import dimforge_rapier3d_compat/areas/main/collider
import dimforge_rapier3d_compat/areas/main/collider_desc
import dimforge_rapier3d_compat/areas/main/event_queue
import dimforge_rapier3d_compat/areas/main/rigid_body
import dimforge_rapier3d_compat/areas/main/rigid_body_desc
import dimforge_rapier3d_compat/areas/main/rotation
import dimforge_rapier3d_compat/areas/main/vector
import dimforge_rapier3d_compat/areas/main/world
import dimforge_rapier3d_compat/runtime as rp_rt
import dimforge_rapier3d_compat/types as rt
import dom/areas/canvas/canvas_rendering_context2d as ctx2d
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_canvas_element
import dom/areas/html/html_element
import dom/types.{type CanvasRenderingContext2D, StringOrCanvasGradientOrCanvasPatternString}
import gleam/int
import gleam/list
import gleam/option.{None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const two_pi = 6.283185307179586

fn to_cx(wx: Float) -> Float {
  340.0 +. wx *. 46.0
}

fn to_cy(wy: Float) -> Float {
  350.0 -. wy *. 46.0
}

fn vec(x: Float, y: Float, z: Float) -> rt.Vector(Nil) {
  vector.create()
  |> vector.with_x(x)
  |> vector.with_y(y)
  |> vector.with_z(z)
}

fn rot_z(theta: Float) -> rt.Rotation(Nil) {
  rotation.create()
  |> rotation.with_x(0.0)
  |> rotation.with_y(0.0)
  |> rotation.with_z(sin(theta *. 0.5))
  |> rotation.with_w(cos(theta *. 0.5))
}

fn start_loop(tick: fn() -> Nil) -> Nil {
  let _ =
    core.request_animation_frame(fn(_) {
      tick()
      start_loop(tick)
    })
  Nil
}

fn draw_rot_box(
  ctx: CanvasRenderingContext2D(Nil),
  cx: Float,
  cy: Float,
  qx: Float,
  qy: Float,
  qz: Float,
  qw: Float,
  hx: Float,
  hy: Float,
  fill_hex: String,
) -> Nil {
  let r00 = 1.0 -. 2.0 *. { qy *. qy +. qz *. qz }
  let r01 = 2.0 *. { qx *. qy -. qw *. qz }
  let r10 = 2.0 *. { qx *. qy +. qw *. qz }
  let r11 = 1.0 -. 2.0 *. { qx *. qx +. qz *. qz }

  let px0 = to_cx(cx +. r00 *. { 0.0 -. hx } +. r01 *. { 0.0 -. hy })
  let py0 = to_cy(cy +. r10 *. { 0.0 -. hx } +. r11 *. { 0.0 -. hy })
  let px1 = to_cx(cx +. r00 *. hx +. r01 *. { 0.0 -. hy })
  let py1 = to_cy(cy +. r10 *. hx +. r11 *. { 0.0 -. hy })
  let px2 = to_cx(cx +. r00 *. hx +. r01 *. hy)
  let py2 = to_cy(cy +. r10 *. hx +. r11 *. hy)
  let px3 = to_cx(cx +. r00 *. { 0.0 -. hx } +. r01 *. hy)
  let py3 = to_cy(cy +. r10 *. { 0.0 -. hx } +. r11 *. hy)

  ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString(fill_hex))
  ctx2d.begin_path(ctx)
  ctx2d.move_to(ctx, px0, py0)
  ctx2d.line_to(ctx, px1, py1)
  ctx2d.line_to(ctx, px2, py2)
  ctx2d.line_to(ctx, px3, py3)
  ctx2d.close_path(ctx)
  ctx2d.fill(ctx, None)

  ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
  ctx2d.set_line_width(ctx, 1.5)
  ctx2d.begin_path(ctx)
  ctx2d.move_to(ctx, px0, py0)
  ctx2d.line_to(ctx, px1, py1)
  ctx2d.line_to(ctx, px2, py2)
  ctx2d.line_to(ctx, px3, py3)
  ctx2d.close_path(ctx)
  ctx2d.stroke(ctx)
}

fn draw_segs(
  ctx: CanvasRenderingContext2D(Nil),
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  n: Int,
) -> Nil {
  let dx = x2 -. x1
  let dy = y2 -. y1
  let total = int.to_float(n)
  int.range(from: 0, to: n, with: Nil, run: fn(_, i) {
    case i % 2 == 0 {
      True -> {
        let t0 = int.to_float(i) /. total
        let t1 = int.to_float(i + 1) /. total
        ctx2d.begin_path(ctx)
        ctx2d.move_to(ctx, x1 +. dx *. t0, y1 +. dy *. t0)
        ctx2d.line_to(ctx, x1 +. dx *. t1, y1 +. dy *. t1)
        ctx2d.stroke(ctx)
      }
      False -> Nil
    }
  })
}

pub fn main() {
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready
  let is_box = [False, False, True, False, True, False]

  let doc = core.get_document()
  let assert Some(canvas) = document.query_selector_canvas(doc)
  let assert Some(ctx) = html_canvas_element.get_context_2d(canvas, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref = rp_rt.ref_new(None)
  let queue_ref = rp_rt.ref_new(None)
  let bodies_ref: rp_rt.Ref(List(#(Bool, rt.RigidBody(Nil)))) = rp_rt.ref_new([])
  let sensor_handle_ref = rp_rt.ref_new(0.0)
  let step_ref = rp_rt.ref_new(0)
  let contacts_ref = rp_rt.ref_new(0)
  let crossings_ref = rp_rt.ref_new(0)
  let active_ref = rp_rt.ref_new(0)
  let paused_ref = rp_rt.ref_new(False)

  let build_scene = fn() {
    case rp_rt.ref_get(queue_ref) {
      Some(old_q) -> event_queue.free(old_q)
      None -> Nil
    }
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w = world.new(vec(0.0, -9.81, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)
    let q = event_queue.new(True, None)

    let add_ramp = fn(cx: Float, cy: Float, hx: Float, hy: Float, theta: Float) {
      let desc =
        rigid_body_desc.fixed()
        |> rigid_body_desc.set_translation(cx, cy, 0.0)
        |> rigid_body_desc.set_rotation(rot_z(theta))
      let rb = world.create_rigid_body(w, desc)
      let cd =
        collider_desc.cuboid(hx, hy, 1.0)
        |> collider_desc.set_friction(0.15)
        |> collider_desc.set_restitution(0.35)
      let _ = world.create_collider(w, cd, Some(rb))
      Nil
    }

    add_ramp(0.0, -0.35, 6.8, 0.35, 0.0)
    add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0)
    add_ramp(5.4, 2.8, 0.25, 3.2, 0.0)
    add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36)
    add_ramp(1.8, 2.6, 2.2, 0.18, 0.36)

    let sbody =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed() |> rigid_body_desc.set_translation(0.0, 1.35, 0.0),
      )
    let scd =
      collider_desc.cuboid(1.85, 0.30, 1.2)
      |> collider_desc.set_sensor(True)
      |> collider_desc.set_active_events(rt.ActiveEventsCollisionEvents)
    let sc = world.create_collider(w, scd, Some(sbody))
    rp_rt.ref_set(sensor_handle_ref, collider.get_handle(sc))

    let b_list =
      list.index_map(is_box, fn(box_flag, i) {
        let x = -3.7 +. int.to_float(i) *. 0.52
        let y = 5.4 +. int.to_float(i % 3) *. 0.58
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(x, y, 0.0),
          )
        let base_cd = case box_flag {
          True -> collider_desc.cuboid(0.28, 0.28, 0.28)
          False -> collider_desc.ball(0.32)
        }
        let cd =
          base_cd
          |> collider_desc.set_restitution(0.35)
          |> collider_desc.set_friction(0.25)
          |> collider_desc.set_active_events(rt.ActiveEventsCollisionEvents)
        let _ = world.create_collider(w, cd, Some(rb))
        #(box_flag, rb)
      })

    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(queue_ref, Some(q))
    rp_rt.ref_set(bodies_ref, b_list)
    rp_rt.ref_set(step_ref, 0)
    rp_rt.ref_set(contacts_ref, 0)
    rp_rt.ref_set(crossings_ref, 0)
    rp_rt.ref_set(active_ref, 0)
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref), rp_rt.ref_get(queue_ref) {
      Some(w), Some(q) -> {
        world.step(w, Some(q), None)
        let _ = rp_rt.ref_update(step_ref, fn(s) { s + 1 })
        let sh = rp_rt.ref_get(sensor_handle_ref)
        event_queue.drain_collision_events(q, fn(h1, h2, started) {
          case h1 == sh || h2 == sh {
            True -> case started {
              True -> {
                let _ = rp_rt.ref_update(crossings_ref, fn(c) { c + 1 })
                let _ = rp_rt.ref_update(active_ref, fn(a) { a + 1 })
                Nil
              }
              False -> {
                let _ = rp_rt.ref_update(active_ref, fn(a) { int.max(0, a - 1) })
                Nil
              }
            }
            False -> case started {
              True -> {
                let _ = rp_rt.ref_update(contacts_ref, fn(c) { c + 1 })
                Nil
              }
              False -> Nil
            }
          }
        })
      }
      _, _ -> Nil
    }
  }

  let render = fn() {
    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#ffffff"))
    ctx2d.fill_rect(ctx, 0.0, 0.0, 680.0, 400.0)

    ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#efefef"))
    ctx2d.set_line_width(ctx, 1.0)
    int.range(from: -6, to: 7, with: Nil, run: fn(_, gx) {
      let x = to_cx(int.to_float(gx))
      ctx2d.begin_path(ctx)
      ctx2d.move_to(ctx, x, 0.0)
      ctx2d.line_to(ctx, x, 350.0)
      ctx2d.stroke(ctx)
    })
    int.range(from: 1, to: 8, with: Nil, run: fn(_, gy) {
      let y = to_cy(int.to_float(gy))
      ctx2d.begin_path(ctx)
      ctx2d.move_to(ctx, 0.0, y)
      ctx2d.line_to(ctx, 680.0, y)
      ctx2d.stroke(ctx)
    })

    let active = rp_rt.ref_get(active_ref)
    let sx = to_cx(-1.85)
    let sy = to_cy(1.65)
    let sw = 3.70 *. 46.0
    let sh = 0.60 *. 46.0
    case active > 0 {
      True -> {
        ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#efefef"))
        ctx2d.fill_rect(ctx, sx, sy, sw, sh)
        ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
        ctx2d.set_line_width(ctx, 2.0)
        ctx2d.stroke_rect(ctx, sx, sy, sw, sh)
      }
      False -> {
        ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#737373"))
        ctx2d.set_line_width(ctx, 1.4)
        draw_segs(ctx, sx, sy, sx +. sw, sy, 14)
        draw_segs(ctx, sx +. sw, sy, sx +. sw, sy +. sh, 4)
        draw_segs(ctx, sx +. sw, sy +. sh, sx, sy +. sh, 14)
        draw_segs(ctx, sx, sy +. sh, sx, sy, 4)
      }
    }

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#555555"))
    ctx2d.set_font(ctx, "11px ui-monospace, monospace")
    ctx2d.fill_text(ctx, "SENSOR GATE (" <> int.to_string(active) <> " inside)", sx +. 8.0, sy +. 17.0, None)

    draw_rot_box(ctx, 0.0, -0.35, 0.0, 0.0, 0.0, 1.0, 6.8, 0.35, "#f7f7f7")
    draw_rot_box(ctx, -5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef")
    draw_rot_box(ctx, 5.4, 2.8, 0.0, 0.0, 0.0, 1.0, 0.25, 3.2, "#efefef")
    draw_rot_box(ctx, -2.1, 4.2, 0.0, 0.0, sin(-0.18), cos(-0.18), 2.2, 0.18, "#e5e5e5")
    draw_rot_box(ctx, 1.8, 2.6, 0.0, 0.0, sin(0.18), cos(0.18), 2.2, 0.18, "#e5e5e5")

    list.each(rp_rt.ref_get(bodies_ref), fn(entry) {
      let #(box_flag, rb) = entry
      let p = rigid_body.translation(rb, None)
      let q = rigid_body.rotation(rb, None)
      let px = vector.get_x(p)
      let py = vector.get_y(p)
      case box_flag {
        True -> draw_rot_box(ctx, px, py, rotation.get_x(q), rotation.get_y(q), rotation.get_z(q), rotation.get_w(q), 0.28, 0.28, "#d4d4d4")
        False -> {
          let cx = to_cx(px)
          let cy = to_cy(py)
          ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#262626"))
          ctx2d.begin_path(ctx)
          ctx2d.arc(ctx, cx, cy, 0.32 *. 46.0, 0.0, two_pi, None)
          ctx2d.fill(ctx, None)
          ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
          ctx2d.set_line_width(ctx, 1.5)
          ctx2d.begin_path(ctx)
          ctx2d.arc(ctx, cx, cy, 0.32 *. 46.0, 0.0, two_pi, None)
          ctx2d.stroke(ctx)
        }
      }
    })

    let p_state = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "EventQueue · solid_contacts="
        <> int.to_string(rp_rt.ref_get(contacts_ref))
        <> " · sensor_crossings="
        <> int.to_string(rp_rt.ref_get(crossings_ref))
        <> " · inside="
        <> int.to_string(active)
        <> " · step="
        <> int.to_string(rp_rt.ref_get(step_ref))
        <> " ("
        <> p_state
        <> ")",
      ),
    )
  }

  build_scene()
  int.range(from: 0, to: 40, with: Nil, run: fn(_, _) { step_sim() })
  render()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener_click(b, fn(_) { cb() }, None)
  }

  wire_click("btn-drop", fn() {
    let v = vec(0.4, 0.0, 0.0)
    list.index_fold(rp_rt.ref_get(bodies_ref), Nil, fn(_, entry, i) {
      let #(_, rb) = entry
      let x = -3.7 +. int.to_float(i) *. 0.52
      let y = 5.4 +. int.to_float(i % 3) *. 0.58
      rigid_body.set_translation(rb, vec(x, y, 0.0), True)
      rigid_body.set_linvel(rb, v, True)
    })
    render()
  })

  wire_click("btn-pause", fn() {
    let p = rp_rt.ref_update(paused_ref, fn(cur) { !cur })
    dom_node.set_text_content(pause_btn, Some(case p { True -> "Resume" False -> "Pause" }))
    render()
  })

  wire_click("btn-reset", fn() {
    rp_rt.ref_set(paused_ref, False)
    dom_node.set_text_content(pause_btn, Some("Pause"))
    build_scene()
    int.range(from: 0, to: 25, with: Nil, run: fn(_, _) { step_sim() })
    render()
  })

  start_loop(fn() {
    case !rp_rt.ref_get(paused_ref) {
      True -> {
        step_sim()
        render()
      }
      False -> Nil
    }
  })
}
