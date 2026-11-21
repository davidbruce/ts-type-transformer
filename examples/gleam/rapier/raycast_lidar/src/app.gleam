import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/ray
import dimforge_rapier3d_compat/ray_collider_intersection
import dimforge_rapier3d_compat/rigid_body_desc
import dimforge_rapier3d_compat/rotation
import dimforge_rapier3d_compat/vector
import dimforge_rapier3d_compat/world
import dimforge_rapier3d_compat/runtime as rp_rt
import dimforge_rapier3d_compat/types as rt
import dom/areas/canvas/canvas_rendering_context2d as ctx2d
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_canvas_element
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import dom/types.{type CanvasRenderingContext2D, StringOrCanvasGradientOrCanvasPatternString}
import dom/areas/html/html_element_tag_name_map as tag
import gleam/float
import gleam/int
import gleam/option.{None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const pi = 3.141592653589793

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

fn fmt2(v: Float) -> String {
  let neg = v <. 0.0
  let n = float.round(float.absolute_value(v) *. 100.0)
  let whole = n / 100
  let frac = n % 100
  let pad = case frac < 10 {
    True -> "0"
    False -> ""
  }
  let sign = case neg && n > 0 {
    True -> "-"
    False -> ""
  }
  sign <> int.to_string(whole) <> "." <> pad <> int.to_string(frac)
}

fn start_loop(tick: fn() -> Nil) -> Nil {
  let _ =
    core.request_animation_frame(fn(_) {
      tick()
      start_loop(tick)
    })
  Nil
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
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready
  let ray_count = 37
  let max_toi = 16.0
  let emitter_x = -5.6
  let emitter_y = 4.6

  let doc = core.get_document()
  let assert Some(canvas) = document.query_selector(doc, tag.canvas)
  let assert Some(ctx) = html_canvas_element.get_context_2d(canvas, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(sweep_btn) = document.get_element_by_id(doc, "btn-sweep")

  let w = world.new(vec(0.0, 0.0, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

  let add_box = fn(cx: Float, cy: Float, theta: Float, hx: Float, hy: Float) {
    let desc =
      rigid_body_desc.fixed()
      |> rigid_body_desc.set_translation(cx, cy, 0.0)
      |> rigid_body_desc.set_rotation(rot_z(theta))
    let rb = world.create_rigid_body(w, desc)
    let _ = world.create_collider(w, collider_desc.cuboid(hx, hy, 1.0), Some(rb))
    Nil
  }
  let add_sphere = fn(cx: Float, cy: Float, r: Float) {
    let rb =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed() |> rigid_body_desc.set_translation(cx, cy, 0.0),
      )
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

  let auto_ref = rp_rt.ref_new(True)
  let phase_ref = rp_rt.ref_new(0.0)
  let offset_ref = rp_rt.ref_new(0.0)

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

    draw_box(ctx, 0.0, -0.35, 0.0, 7.0, 0.35, "#f7f7f7")
    draw_box(ctx, -1.6, 0.85, 0.0, 0.70, 0.85, "#efefef")
    draw_box(ctx, 2.2, 1.35, 0.0, 0.65, 1.35, "#efefef")
    draw_box(ctx, 0.2, 1.2, -0.42, 1.3, 0.18, "#d4d4d4")
    draw_box(ctx, 0.4, 4.4, 0.0, 1.9, 0.22, "#efefef")
    draw_circle(ctx, -2.5, 3.6, 0.65, "#d4d4d4")
    draw_circle(ctx, 4.2, 3.2, 0.75, "#d4d4d4")

    let base_angle = -0.35 +. rp_rt.ref_get(offset_ref) +. 0.28 *. sin(rp_rt.ref_get(phase_ref))
    let ox = to_cx(emitter_x)
    let oy = to_cy(emitter_y)
    let origin = vec(emitter_x, emitter_y, 0.0)

    let #(hits, min_toi) =
      int.range(from: 0, to: ray_count, with: #(0, max_toi), run: fn(acc, i) {
        let #(hit_count, cur_min) = acc
        let frac = int.to_float(i) /. int.to_float(ray_count - 1) -. 0.5
        let angle = base_angle +. frac *. 1.35
        let dx = cos(angle)
        let dy = sin(angle)
        let r = ray.new(origin, vec(dx, dy, 0.0))
        case world.cast_ray_and_get_normal(w, r, max_toi, True, None, None, None, None, None) {
          Some(hit) -> {
            let toi = ray_collider_intersection.get_time_of_impact(hit)
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

            #(hit_count + 1, float.min(cur_min, toi))
          }
          None -> {
            ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#e5e5e5"))
            ctx2d.set_line_width(ctx, 0.9)
            ctx2d.begin_path(ctx)
            ctx2d.move_to(ctx, ox, oy)
            ctx2d.line_to(ctx, to_cx(emitter_x +. dx *. max_toi), to_cy(emitter_y +. dy *. max_toi))
            ctx2d.stroke(ctx)
            acc
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

    let deg = float.round(base_angle *. 180.0 /. pi)
    let mode = case rp_rt.ref_get(auto_ref) {
      True -> "auto"
      False -> "manual"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "37 rays · hits="
        <> int.to_string(hits)
        <> "/37 · nearest_toi="
        <> fmt2(min_toi)
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
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-sweep", fn() {
    let a = rp_rt.ref_update(auto_ref, fn(cur) { !cur })
    dom_node.set_text_content(sweep_btn, Some(case a { True -> "Sweep: Auto" False -> "Sweep: Manual" }))
    render()
  })

  wire_click("btn-angle", fn() {
    let _ =
      rp_rt.ref_update(offset_ref, fn(cur) {
        let next = cur +. pi /. 12.0
        case next >. 0.8 {
          True -> -0.6
          False -> next
        }
      })
    render()
  })

  wire_click("btn-reset", fn() {
    rp_rt.ref_set(auto_ref, True)
    rp_rt.ref_set(phase_ref, 0.0)
    rp_rt.ref_set(offset_ref, 0.0)
    dom_node.set_text_content(sweep_btn, Some("Sweep: Auto"))
    render()
  })

  start_loop(fn() {
    case rp_rt.ref_get(auto_ref) {
      True -> {
        let _ = rp_rt.ref_update(phase_ref, fn(p) { p +. 0.025 })
        render()
      }
      False -> Nil
    }
  })
}
