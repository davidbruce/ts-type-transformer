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

type Cell(a)

@external(javascript, "./app_ffi.mjs", "coerce")
fn coerce(a: a) -> b

@external(javascript, "./app_ffi.mjs", "newCell")
fn new_cell(v: a) -> Cell(a)

@external(javascript, "./app_ffi.mjs", "getCell")
fn get_cell(c: Cell(a)) -> a

@external(javascript, "./app_ffi.mjs", "setCell")
fn set_cell(c: Cell(a), v: a) -> Nil

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

fn to_cx(wx: Float) -> Float {
  340.0 +. wx *. 46.0
}

fn to_cy(wy: Float) -> Float {
  350.0 -. wy *. 46.0
}

fn vec(x: Float, y: Float, z: Float) -> rt.Vector(Nil) {
  let v = vector.create()
  vector.set_x(v, x)
  vector.set_y(v, y)
  vector.set_z(v, z)
  v
}

fn rot_z(theta: Float) -> rt.Rotation(Nil) {
  let r = rotation.create()
  rotation.set_x(r, 0.0)
  rotation.set_y(r, 0.0)
  rotation.set_z(r, sin(theta *. 0.5))
  rotation.set_w(r, cos(theta *. 0.5))
  r
}

fn times(i: Int, n: Int, f: fn(Int) -> Nil) -> Nil {
  case i < n {
    True -> {
      f(i)
      times(i + 1, n, f)
    }
    False -> Nil
  }
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
  times(0, n, fn(i) {
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
  rp_rt.promise_then(rapier.init(), fn(ready) {
    let assert Ok(_) = ready
    let two_pi = 6.283185307179586
    let is_box = [False, False, True, False, True, False]

    let doc = core.get_document()
    let assert Some(canvas_el) = document.get_element_by_id(doc, "canvas")
    let assert Some(ctx) = html_canvas_element.get_context_2d(coerce(canvas_el), None)
    let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
    let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

    let world_cell = new_cell(None)
    let queue_cell = new_cell(None)
    let bodies_cell: Cell(List(#(Bool, rt.RigidBody(Nil)))) = new_cell([])
    let sensor_handle_cell = new_cell(0.0)
    let step_cell = new_cell(0)
    let contacts_cell = new_cell(0)
    let crossings_cell = new_cell(0)
    let active_cell = new_cell(0)
    let paused_cell = new_cell(False)

    let build_scene = fn() {
      case get_cell(queue_cell) {
        Some(old_q) -> event_queue.free(old_q)
        None -> Nil
      }
      case get_cell(world_cell) {
        Some(old_w) -> world.free(old_w)
        None -> Nil
      }
      let w = world.new(vec(0.0, -9.81, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)
      let q = event_queue.new(True, None)

      let add_ramp = fn(cx: Float, cy: Float, hx: Float, hy: Float, theta: Float) {
        let desc =
          rigid_body_desc.fixed()
          |> rigid_body_desc.set_translation_2(cx, cy, 0.0)
          |> rigid_body_desc.set_rotation_2(rot_z(theta))
        let rb = world.create_rigid_body(w, desc)
        let cd =
          collider_desc.cuboid(hx, hy, 1.0)
          |> collider_desc.set_friction_2(0.15)
          |> collider_desc.set_restitution_2(0.35)
        let _ = world.create_collider(w, cd, Some(rb))
        Nil
      }

      add_ramp(0.0, -0.35, 6.8, 0.35, 0.0)
      add_ramp(-5.4, 2.8, 0.25, 3.2, 0.0)
      add_ramp(5.4, 2.8, 0.25, 3.2, 0.0)
      add_ramp(-2.1, 4.2, 2.2, 0.18, -0.36)
      add_ramp(1.8, 2.6, 2.2, 0.18, 0.36)

      let sbody = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), 0.0, 1.35, 0.0))
      let scd =
        collider_desc.cuboid(1.85, 0.30, 1.2)
        |> collider_desc.set_sensor(True)
        |> collider_desc.set_active_events_2(rt.ActiveEventsCOLLISIONEVENTS)
      let sc = world.create_collider(w, scd, Some(sbody))
      set_cell(sensor_handle_cell, collider.get_handle(sc))

      let b_list =
        list.index_map(is_box, fn(box_flag, i) {
          let x = -3.7 +. int.to_float(i) *. 0.52
          let y = 5.4 +. int.to_float(i % 3) *. 0.58
          let rb = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.dynamic_2(), x, y, 0.0))
          let base_cd = case box_flag {
            True -> collider_desc.cuboid(0.28, 0.28, 0.28)
            False -> collider_desc.ball(0.32)
          }
          let cd =
            base_cd
            |> collider_desc.set_restitution_2(0.35)
            |> collider_desc.set_friction_2(0.25)
            |> collider_desc.set_active_events_2(rt.ActiveEventsCOLLISIONEVENTS)
          let _ = world.create_collider(w, cd, Some(rb))
          #(box_flag, rb)
        })

      set_cell(world_cell, Some(w))
      set_cell(queue_cell, Some(q))
      set_cell(bodies_cell, b_list)
      set_cell(step_cell, 0)
      set_cell(contacts_cell, 0)
      set_cell(crossings_cell, 0)
      set_cell(active_cell, 0)
    }

    let step_sim = fn() {
      case get_cell(world_cell), get_cell(queue_cell) {
        Some(w), Some(q) -> {
          world.step(w, Some(q), None)
          set_cell(step_cell, get_cell(step_cell) + 1)
          let sh = get_cell(sensor_handle_cell)
          event_queue.drain_collision_events(q, fn(h1, h2, started) {
            case h1 == sh || h2 == sh {
              True -> case started {
                True -> {
                  set_cell(crossings_cell, get_cell(crossings_cell) + 1)
                  set_cell(active_cell, get_cell(active_cell) + 1)
                }
                False -> case get_cell(active_cell) > 0 {
                  True -> set_cell(active_cell, get_cell(active_cell) - 1)
                  False -> Nil
                }
              }
              False -> case started {
                True -> set_cell(contacts_cell, get_cell(contacts_cell) + 1)
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
      times(-6, 7, fn(gx) {
        let x = to_cx(int.to_float(gx))
        ctx2d.begin_path(ctx)
        ctx2d.move_to(ctx, x, 0.0)
        ctx2d.line_to(ctx, x, 350.0)
        ctx2d.stroke(ctx)
      })
      times(1, 8, fn(gy) {
        let y = to_cy(int.to_float(gy))
        ctx2d.begin_path(ctx)
        ctx2d.move_to(ctx, 0.0, y)
        ctx2d.line_to(ctx, 680.0, y)
        ctx2d.stroke(ctx)
      })

      let active = get_cell(active_cell)
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

      list.each(get_cell(bodies_cell), fn(entry) {
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

      let p_state = case get_cell(paused_cell) {
        True -> "paused"
        False -> "running"
      }
      dom_node.set_text_content(
        meta_el,
        Some(
          "EventQueue · solid_contacts="
          <> int.to_string(get_cell(contacts_cell))
          <> " · sensor_crossings="
          <> int.to_string(get_cell(crossings_cell))
          <> " · inside="
          <> int.to_string(active)
          <> " · step="
          <> int.to_string(get_cell(step_cell))
          <> " ("
          <> p_state
          <> ")",
        ),
      )
    }

    build_scene()
    times(0, 40, fn(_) { step_sim() })
    render()

    let wire_click = fn(id: String, cb: fn() -> Nil) {
      let assert Some(b) = document.get_element_by_id(doc, id)
      html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
    }

    wire_click("btn-drop", fn() {
      let v = vec(0.4, 0.0, 0.0)
      let _ = list.index_map(get_cell(bodies_cell), fn(entry, i) {
        let #(_, rb) = entry
        let x = -3.7 +. int.to_float(i) *. 0.52
        let y = 5.4 +. int.to_float(i % 3) *. 0.58
        rigid_body.set_translation(rb, vec(x, y, 0.0), True)
        rigid_body.set_linvel(rb, v, True)
      })
      render()
    })

    wire_click("btn-pause", fn() {
      let p = !get_cell(paused_cell)
      set_cell(paused_cell, p)
      dom_node.set_text_content(pause_btn, Some(case p { True -> "Resume" False -> "Pause" }))
      render()
    })

    wire_click("btn-reset", fn() {
      set_cell(paused_cell, False)
      dom_node.set_text_content(pause_btn, Some("Pause"))
      build_scene()
      times(0, 25, fn(_) { step_sim() })
      render()
    })

    let loop_cell = new_cell(fn(_) { Nil })
    set_cell(loop_cell, fn(_) {
      case !get_cell(paused_cell) {
        True -> {
          step_sim()
          render()
        }
        False -> Nil
      }
      let _ = core.request_animation_frame(get_cell(loop_cell))
      Nil
    })
    let _ = core.request_animation_frame(get_cell(loop_cell))
    Nil
  })
}
