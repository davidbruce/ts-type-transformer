import dimforge_rapier3d_compat/areas/main as rapier
import dimforge_rapier3d_compat/areas/main/collider_desc
import dimforge_rapier3d_compat/areas/main/ray
import dimforge_rapier3d_compat/areas/main/ray_collider_intersection
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

fn rot_z(theta: Float) -> rt.Rotation(Nil) {
  let r = rotation.create()
  rotation.set_x(r, 0.0)
  rotation.set_y(r, 0.0)
  rotation.set_z(r, sin(theta *. 0.5))
  rotation.set_w(r, cos(theta *. 0.5))
  r
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

fn times(i: Int, n: Int, f: fn(Int) -> Nil) -> Nil {
  case i < n {
    True -> {
      f(i)
      times(i + 1, n, f)
    }
    False -> Nil
  }
}

fn draw_box(
  ctx: CanvasRenderingContext2D(Nil),
  cx: Float,
  cy: Float,
  theta: Float,
  hx: Float,
  hy: Float,
  fill_hex: String,
) -> Nil {
  let c = cos(theta)
  let s = sin(theta)
  let px0 = to_cx(cx +. c *. { 0.0 -. hx } -. s *. { 0.0 -. hy })
  let py0 = to_cy(cy +. s *. { 0.0 -. hx } +. c *. { 0.0 -. hy })
  let px1 = to_cx(cx +. c *. hx -. s *. { 0.0 -. hy })
  let py1 = to_cy(cy +. s *. hx +. c *. { 0.0 -. hy })
  let px2 = to_cx(cx +. c *. hx -. s *. hy)
  let py2 = to_cy(cy +. s *. hx +. c *. hy)
  let px3 = to_cx(cx +. c *. { 0.0 -. hx } -. s *. hy)
  let py3 = to_cy(cy +. s *. { 0.0 -. hx } +. c *. hy)

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

fn draw_circle(
  ctx: CanvasRenderingContext2D(Nil),
  wx: Float,
  wy: Float,
  r: Float,
  fill_hex: String,
) -> Nil {
  let two_pi = 6.283185307179586
  let cx = to_cx(wx)
  let cy = to_cy(wy)
  let rpx = r *. 46.0
  ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString(fill_hex))
  ctx2d.begin_path(ctx)
  ctx2d.arc(ctx, cx, cy, rpx, 0.0, two_pi, None)
  ctx2d.fill(ctx, None)
  ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
  ctx2d.set_line_width(ctx, 1.5)
  ctx2d.begin_path(ctx)
  ctx2d.arc(ctx, cx, cy, rpx, 0.0, two_pi, None)
  ctx2d.stroke(ctx)
}

pub fn main() {
  rp_rt.promise_then(rapier.init(), fn(ready) {
    let assert Ok(_) = ready
    let pi = 3.141592653589793
    let two_pi = 6.283185307179586
    let ray_count = 37
    let max_toi = 16.0
    let emitter_x = -5.6
    let emitter_y = 4.6

    let doc = core.get_document()
    let assert Some(canvas_el) = document.get_element_by_id(doc, "canvas")
    let assert Some(ctx) = html_canvas_element.get_context_2d(coerce(canvas_el), None)
    let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
    let assert Some(sweep_btn) = document.get_element_by_id(doc, "btn-sweep")

    let w = world.new(vec(0.0, 0.0, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

    let add_box = fn(cx: Float, cy: Float, theta: Float, hx: Float, hy: Float) {
      let desc =
        rigid_body_desc.fixed()
        |> rigid_body_desc.set_translation_2(cx, cy, 0.0)
        |> rigid_body_desc.set_rotation_2(rot_z(theta))
      let rb = world.create_rigid_body(w, desc)
      let _ = world.create_collider(w, collider_desc.cuboid(hx, hy, 1.0), Some(rb))
      Nil
    }
    let add_sphere = fn(cx: Float, cy: Float, r: Float) {
      let rb = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), cx, cy, 0.0))
      let _ = world.create_collider(w, collider_desc.ball(r), Some(rb))
      Nil
    }

    add_box(0.0, -0.35, 0.0, 7.0, 0.35)
    add_box(-1.6, 0.85, 0.0, 0.70, 0.85)
    add_box(2.2, 1.35, 0.0, 0.65, 1.35)
    add_box(0.2, 1.2, -0.42, 1.3, 0.18)
    add_box(0.4, 4.4, 0.0, 1.9, 0.22)
    add_sphere(-2.5, 3.6, 0.65)
    add_sphere(4.2, 3.2, 0.75)
    world.step(w, None, None)

    let auto_cell = new_cell(True)
    let phase_cell = new_cell(0.0)
    let offset_cell = new_cell(0.0)

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

      draw_box(ctx, 0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7")
      draw_box(ctx, -1.6, 0.85, 0.0, 0.70, 0.85, "#efefef")
      draw_box(ctx, 2.2, 1.35, 0.0, 0.65, 1.35, "#efefef")
      draw_box(ctx, 0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4")
      draw_box(ctx, 0.4, 4.4, 0.0, 1.9, 0.22, "#efefef")
      draw_circle(ctx, -2.5, 3.6, 0.65, "#d4d4d4")
      draw_circle(ctx, 4.2, 3.2, 0.75, "#d4d4d4")

      let base_angle = -0.35 +. get_cell(offset_cell) +. 0.28 *. sin(get_cell(phase_cell))
      let ox = to_cx(emitter_x)
      let oy = to_cy(emitter_y)
      let origin = vec(emitter_x, emitter_y, 0.0)

      let hits_cell = new_cell(0)
      let min_toi_cell = new_cell(max_toi)
      times(0, ray_count, fn(i) {
        let frac = int.to_float(i) /. int.to_float(ray_count - 1) -. 0.5
        let angle = base_angle +. frac *. 1.35
        let dx = cos(angle)
        let dy = sin(angle)
        let r = ray.new(origin, vec(dx, dy, 0.0))
        case world.cast_ray_and_get_normal(w, r, max_toi, True, None, None, None, None, None) {
          Some(hit) -> {
            set_cell(hits_cell, get_cell(hits_cell) + 1)
            let toi = ray_collider_intersection.get_time_of_impact(hit)
            case toi <. get_cell(min_toi_cell) {
              True -> set_cell(min_toi_cell, toi)
              False -> Nil
            }
            let pt = ray.point_at(r, toi)
            let ptx = vector.get_x(pt)
            let pty = vector.get_y(pt)
            let hx = to_cx(ptx)
            let hy = to_cy(pty)

            ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#737373"))
            ctx2d.set_line_width(ctx, 1.0)
            ctx2d.begin_path(ctx)
            ctx2d.move_to(ctx, ox, oy)
            ctx2d.line_to(ctx, hx, hy)
            ctx2d.stroke(ctx)

            let n = ray_collider_intersection.get_normal(hit)
            ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
            ctx2d.set_line_width(ctx, 1.4)
            ctx2d.begin_path(ctx)
            ctx2d.move_to(ctx, hx, hy)
            ctx2d.line_to(ctx, to_cx(ptx +. vector.get_x(n) *. 0.45), to_cy(pty +. vector.get_y(n) *. 0.45))
            ctx2d.stroke(ctx)

            ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
            ctx2d.begin_path(ctx)
            ctx2d.arc(ctx, hx, hy, 2.8, 0.0, two_pi, None)
            ctx2d.fill(ctx, None)
          }
          None -> {
            ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#e5e5e5"))
            ctx2d.set_line_width(ctx, 0.9)
            ctx2d.begin_path(ctx)
            ctx2d.move_to(ctx, ox, oy)
            ctx2d.line_to(ctx, to_cx(emitter_x +. dx *. max_toi), to_cy(emitter_y +. dy *. max_toi))
            ctx2d.stroke(ctx)
          }
        }
      })

      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, ox, oy, 6.0, 0.0, two_pi, None)
      ctx2d.fill(ctx, None)
      ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#ffffff"))
      ctx2d.set_line_width(ctx, 1.5)
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, ox, oy, 3.0, 0.0, two_pi, None)
      ctx2d.stroke(ctx)

      let deg = round_int(base_angle *. 180.0 /. pi)
      let mode = case get_cell(auto_cell) {
        True -> "auto"
        False -> "manual"
      }
      dom_node.set_text_content(
        meta_el,
        Some(
          "37 rays · hits="
          <> int.to_string(get_cell(hits_cell))
          <> "/37 · nearest_toi="
          <> fmt2(get_cell(min_toi_cell))
          <> " m · center="
          <> int.to_string(deg)
          <> " deg ("
          <> mode
          <> ")",
        ),
      )
    }

    render()

    let wire_click = fn(id: String, cb: fn() -> Nil) {
      let assert Some(b) = document.get_element_by_id(doc, id)
      html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
    }

    wire_click("btn-sweep", fn() {
      let a = !get_cell(auto_cell)
      set_cell(auto_cell, a)
      dom_node.set_text_content(sweep_btn, Some(case a { True -> "Sweep: Auto" False -> "Sweep: Manual" }))
      render()
    })

    wire_click("btn-angle", fn() {
      let next = get_cell(offset_cell) +. pi /. 12.0
      set_cell(offset_cell, case next >. 0.8 { True -> -0.6 False -> next })
      render()
    })

    wire_click("btn-reset", fn() {
      set_cell(auto_cell, True)
      set_cell(phase_cell, 0.0)
      set_cell(offset_cell, 0.0)
      dom_node.set_text_content(sweep_btn, Some("Sweep: Auto"))
      render()
    })

    let loop_cell = new_cell(fn(_) { Nil })
    set_cell(loop_cell, fn(_) {
      case get_cell(auto_cell) {
        True -> {
          set_cell(phase_cell, get_cell(phase_cell) +. 0.025)
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
