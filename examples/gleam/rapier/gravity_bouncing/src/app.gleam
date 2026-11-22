import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/rigid_body
import dimforge_rapier3d_compat/rigid_body_desc
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
import gleam/list
import gleam/option.{None, Some}

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

fn fmt1(v: Float) -> String {
  let neg = v <. 0.0
  let n = float.round(float.absolute_value(v) *. 10.0)
  let whole = n / 10
  let frac = n % 10
  let sign = case neg && n > 0 {
    True -> "-"
    False -> ""
  }
  sign <> int.to_string(whole) <> "." <> int.to_string(frac)
}

fn start_loop(tick: fn() -> Nil) -> Nil {
  core.request_animation_frame(fn(_) {
    tick()
    start_loop(tick)
  })
  Nil
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
  int.range(from: 0, to: segs, with: Nil, run: fn(_, i) {
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
  let restitutions = [0.00, 0.35, 0.60, 0.80, 0.95]
  let x_coords = [-4.2, -2.1, 0.0, 2.1, 4.2]
  let ball_hex = ["#efefef", "#d4d4d4", "#b5b5b5", "#737373", "#262626"]
  let rest_labels = ["e=0.00", "e=0.35", "e=0.60", "e=0.80", "e=0.95"]

  let doc = core.get_document()
  let assert Some(canvas) = document.query_selector(doc, tag.canvas)
  let assert Some(ctx) = html_canvas_element.get_context_2d(canvas, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(grav_btn) = document.get_element_by_id(doc, "btn-gravity")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref = rp_rt.ref_new(None)
  let balls_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let trails_ref: rp_rt.Ref(List(#(Int, Float, Float))) = rp_rt.ref_new([])
  let step_ref = rp_rt.ref_new(0)
  let earth_ref = rp_rt.ref_new(True)
  let paused_ref = rp_rt.ref_new(False)

  let build_scene = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let gy = case rp_rt.ref_get(earth_ref) {
      True -> -9.81
      False -> -1.62
    }
    let w = world.new(vec(0.0, gy, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

    let add_wall = fn(x: Float, y: Float, hx: Float, hy: Float) {
      let body =
        world.create_rigid_body(
          w,
          rigid_body_desc.fixed() |> rigid_body_desc.set_translation(x, y, 0.0),
        )
      world.create_collider(w, collider_desc.cuboid(hx, hy, 2.0), Some(body))
      Nil
    }
    add_wall(0.0, -0.4, 7.0, 0.4)
    add_wall(-6.8, 3.5, 0.25, 4.0)
    add_wall(6.8, 3.5, 0.25, 4.0)

    let rbs =
      list.map(list.zip(x_coords, restitutions), fn(pair) {
        let #(wx, rest) = pair
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(wx, 5.6, 0.0),
          )
        let cd =
          collider_desc.ball(0.42)
          |> collider_desc.set_restitution(rest)
          |> collider_desc.set_friction(0.2)
          |> collider_desc.set_density(1.2)
        world.create_collider(w, cd, Some(rb))
        rb
      })
    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(balls_ref, rbs)
    rp_rt.ref_set(trails_ref, [])
    rp_rt.ref_set(step_ref, 0)
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    let pts =
      list.index_map(rp_rt.ref_get(balls_ref), fn(rb, i) {
        let p = rigid_body.translation(rb, None)
        #(i, vector.get_x(p), vector.get_y(p))
      })
    rp_rt.ref_update(trails_ref, fn(old) { list.take(list.append(pts, old), 100) })
    Nil
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

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#f7f7f7"))
    ctx2d.fill_rect(ctx, 0.0, 350.0, 680.0, 50.0)
    ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#d4d4d4"))
    int.range(from: 0, to: 40, with: Nil, run: fn(_, i) {
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
    list.each(rp_rt.ref_get(trails_ref), fn(pt) {
      let #(_, tx, ty) = pt
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 1.6, 0.0, two_pi, None)
      ctx2d.fill(ctx, None)
    })

    ctx2d.set_font(ctx, "11px ui-monospace, monospace")
    let meta_items = list.zip(rp_rt.ref_get(balls_ref), list.zip(ball_hex, rest_labels))
    let #(energy, max_y) =
      list.index_fold(meta_items, #(0.0, 0.0), fn(acc, item, i) {
        let #(e_sum, cur_max_y) = acc
        let #(rb, #(hex, lbl)) = item
        let p = rigid_body.translation(rb, None)
        let v = rigid_body.linvel(rb, None)
        let py = vector.get_y(p)
        let vx = vector.get_x(v)
        let vy = vector.get_y(v)

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

        #(e_sum +. 0.5 *. { vx *. vx +. vy *. vy }, float.max(cur_max_y, py))
      })

    let g_mode = case rp_rt.ref_get(earth_ref) {
      True -> "Earth (-9.81)"
      False -> "Moon (-1.62)"
    }
    let p_state = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "5 spheres · g="
        <> g_mode
        <> " m/s² · max_y="
        <> fmt2(max_y)
        <> " m · ke="
        <> fmt1(energy)
        <> " · step="
        <> int.to_string(rp_rt.ref_get(step_ref))
        <> " ("
        <> p_state
        <> ")",
      ),
    )
  }

  build_scene()
  int.range(from: 0, to: 28, with: Nil, run: fn(_, _) { step_sim() })
  render()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-gravity", fn() {
    let earth = rp_rt.ref_update(earth_ref, fn(cur) { !cur })
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.set_gravity(w, vec(0.0, case earth { True -> -9.81 False -> -1.62 }, 0.0))
      None -> Nil
    }
    list.each(rp_rt.ref_get(balls_ref), fn(rb) { rigid_body.apply_impulse(rb, vec(0.0, 0.25, 0.0), True) })
    dom_node.set_text_content(grav_btn, Some(case earth { True -> "Gravity: Earth" False -> "Gravity: Moon" }))
    render()
  })

  wire_click("btn-kick", fn() {
    list.index_fold(rp_rt.ref_get(balls_ref), Nil, fn(_, rb, i) {
      let dx = { int.to_float(i) -. 2.0 } *. 0.28
      rigid_body.apply_impulse(rb, vec(dx, 3.4, 0.0), True)
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
    int.range(from: 0, to: 18, with: Nil, run: fn(_, _) { step_sim() })
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
