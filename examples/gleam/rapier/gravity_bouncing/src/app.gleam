import dimforge_rapier3d_compat/areas/main as rapier
import dimforge_rapier3d_compat/areas/main/collider_desc
import dimforge_rapier3d_compat/areas/main/rigid_body
import dimforge_rapier3d_compat/areas/main/rigid_body_desc
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

@external(javascript, "./app_ffi.mjs", "abs")
fn abs(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "roundInt")
fn round_int(x: Float) -> Int

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

fn fmt2(v: Float) -> String {
  let sign = case v <. 0.0 {
    True -> "-"
    False -> ""
  }
  let n = round_int(abs(v) *. 100.0)
  let whole = n / 100
  let frac = n % 100
  let pad = case frac < 10 {
    True -> "0"
    False -> ""
  }
  sign <> int.to_string(whole) <> "." <> pad <> int.to_string(frac)
}

fn fmt1(v: Float) -> String {
  let sign = case v <. 0.0 {
    True -> "-"
    False -> ""
  }
  let n = round_int(abs(v) *. 10.0)
  let whole = n / 10
  let frac = n % 10
  sign <> int.to_string(whole) <> "." <> int.to_string(frac)
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

fn draw_dashed(
  ctx: CanvasRenderingContext2D(Nil),
  x1: Float,
  y1: Float,
  x2: Float,
  y2: Float,
  segs: Int,
) -> Nil {
  let dx = x2 -. x1
  let dy = y2 -. y1
  let total = int.to_float(segs)
  times(0, segs, fn(i) {
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
    let restitutions = [0.00, 0.35, 0.60, 0.80, 0.95]
    let x_coords = [-4.2, -2.1, 0.0, 2.1, 4.2]
    let ball_hex = ["#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626"]
    let rest_labels = ["e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95"]

    let doc = core.get_document()
    let assert Some(canvas_el) = document.get_element_by_id(doc, "canvas")
    let assert Some(ctx) = html_canvas_element.get_context_2d(coerce(canvas_el), None)
    let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
    let assert Some(grav_btn) = document.get_element_by_id(doc, "btn-gravity")
    let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

    let world_cell = new_cell(None)
    let balls_cell: Cell(List(rt.RigidBody(Nil))) = new_cell([])
    let trails_cell: Cell(List(#(Int, Float, Float))) = new_cell([])
    let step_cell = new_cell(0)
    let earth_cell = new_cell(True)
    let paused_cell = new_cell(False)

    let build_scene = fn() {
      case get_cell(world_cell) {
        Some(old_w) -> world.free(old_w)
        None -> Nil
      }
      let gy = case get_cell(earth_cell) {
        True -> -9.81
        False -> -1.62
      }
      let w = world.new(vec(0.0, gy, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

      let ground = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), 0.0, -0.4, 0.0))
      let _ = world.create_collider(w, collider_desc.cuboid(7.0, 0.4, 2.0), Some(ground))
      let lw = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), -6.8, 3.5, 0.0))
      let _ = world.create_collider(w, collider_desc.cuboid(0.25, 4.0, 2.0), Some(lw))
      let rw = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), 6.8, 3.5, 0.0))
      let _ = world.create_collider(w, collider_desc.cuboid(0.25, 4.0, 2.0), Some(rw))

      let rbs =
        list.map(list.zip(x_coords, restitutions), fn(pair) {
          let #(wx, rest) = pair
          let rb = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.dynamic_2(), wx, 5.6, 0.0))
          let cd =
            collider_desc.ball(0.42)
            |> collider_desc.set_restitution_2(rest)
            |> collider_desc.set_friction_2(0.2)
            |> collider_desc.set_density_2(1.2)
          let _ = world.create_collider(w, cd, Some(rb))
          rb
        })
      set_cell(world_cell, Some(w))
      set_cell(balls_cell, rbs)
      set_cell(trails_cell, [])
      set_cell(step_cell, 0)
    }

    let step_sim = fn() {
      case get_cell(world_cell) {
        Some(w) -> world.step(w, None, None)
        None -> Nil
      }
      set_cell(step_cell, get_cell(step_cell) + 1)
      let pts =
        list.index_map(get_cell(balls_cell), fn(rb, i) {
          let p = rigid_body.translation(rb, None)
          #(i, vector.get_x(p), vector.get_y(p))
        })
      let combined = list.take(list.append(pts, get_cell(trails_cell)), 100)
      set_cell(trails_cell, combined)
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

      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#f7f7f7"))
      ctx2d.fill_rect(ctx, 0.0, 350.0, 680.0, 50.0)
      ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#d4d4d4"))
      times(0, 40, fn(i) {
        let hx = -20.0 +. int.to_float(i) *. 18.0
        ctx2d.begin_path(ctx)
        ctx2d.move_to(ctx, hx, 350.0)
        ctx2d.line_to(ctx, hx -. 24.0, 400.0)
        ctx2d.stroke(ctx)
      })
      ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
      ctx2d.set_line_width(ctx, 1.5)
      ctx2d.begin_path(ctx)
      ctx2d.move_to(ctx, 0.0, 350.0)
      ctx2d.line_to(ctx, 680.0, 350.0)
      ctx2d.stroke(ctx)

      ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#d4d4d4"))
      ctx2d.set_line_width(ctx, 1.0)
      draw_dashed(ctx, to_cx(-5.0), to_cy(5.6), to_cx(5.0), to_cy(5.6), 40)
      list.each(x_coords, fn(wx) {
        let cx = to_cx(wx)
        draw_dashed(ctx, cx, to_cy(5.6), cx, 350.0, 20)
      })

      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#b5b5b5"))
      list.each(get_cell(trails_cell), fn(pt) {
        let #(_, tx, ty) = pt
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 1.6, 0.0, two_pi, None)
        ctx2d.fill(ctx, None)
      })

      let energy_cell = new_cell(0.0)
      let max_y_cell = new_cell(0.0)
      ctx2d.set_font(ctx, "11px ui-monospace, monospace")

      let meta_items = list.zip(get_cell(balls_cell), list.zip(ball_hex, rest_labels))
      list.index_map(meta_items, fn(item, i) {
        let #(rb, #(hex, lbl)) = item
        let p = rigid_body.translation(rb, None)
        let v = rigid_body.linvel(rb, None)
        let py = vector.get_y(p)
        let vx = vector.get_x(v)
        let vy = vector.get_y(v)
        set_cell(energy_cell, get_cell(energy_cell) +. 0.5 *. { vx *. vx +. vy *. vy })
        case py >. get_cell(max_y_cell) {
          True -> set_cell(max_y_cell, py)
          False -> Nil
        }

        let cx = to_cx(vector.get_x(p))
        let cy = to_cy(py)
        let rpx = 0.42 *. 46.0

        ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString(hex))
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, cx, cy, rpx, 0.0, two_pi, None)
        ctx2d.fill(ctx, None)

        ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
        ctx2d.set_line_width(ctx, 1.6)
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, cx, cy, rpx, 0.0, two_pi, None)
        ctx2d.stroke(ctx)

        let hl = case i >= 3 {
          True -> "#d4d4d4"
          False -> "#737373"
        }
        ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString(hl))
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, cx -. 5.0, cy -. 5.0, 2.8, 0.0, two_pi, None)
        ctx2d.fill(ctx, None)

        ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
        ctx2d.fill_text(ctx, lbl, cx -. 22.0, 368.0, None)
        Nil
      })

      let g_mode = case get_cell(earth_cell) {
        True -> "Earth (-9.81)"
        False -> "Moon (-1.62)"
      }
      let p_state = case get_cell(paused_cell) {
        True -> "paused"
        False -> "running"
      }
      dom_node.set_text_content(
        meta_el,
        Some(
          "5 spheres · g="
          <> g_mode
          <> " m/s² · max_y="
          <> fmt2(get_cell(max_y_cell))
          <> " m · ke="
          <> fmt1(get_cell(energy_cell))
          <> " · step="
          <> int.to_string(get_cell(step_cell))
          <> " ("
          <> p_state
          <> ")",
        ),
      )
    }

    build_scene()
    times(0, 28, fn(_) { step_sim() })
    render()

    let wire_click = fn(id: String, cb: fn() -> Nil) {
      let assert Some(b) = document.get_element_by_id(doc, id)
      html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
    }

    wire_click("btn-gravity", fn() {
      let earth = !get_cell(earth_cell)
      set_cell(earth_cell, earth)
      case get_cell(world_cell) {
        Some(w) -> world.set_gravity(w, vec(0.0, case earth { True -> -9.81 False -> -1.62 }, 0.0))
        None -> Nil
      }
      list.each(get_cell(balls_cell), fn(rb) { rigid_body.apply_impulse(rb, vec(0.0, 0.25, 0.0), True) })
      dom_node.set_text_content(grav_btn, Some(case earth { True -> "Gravity: Earth" False -> "Gravity: Moon" }))
      render()
    })

    wire_click("btn-kick", fn() {
      let _ = list.index_map(get_cell(balls_cell), fn(rb, i) {
        let dx = { int.to_float(i) -. 2.0 } *. 0.28
        rigid_body.apply_impulse(rb, vec(dx, 3.4, 0.0), True)
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
      times(0, 18, fn(_) { step_sim() })
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
