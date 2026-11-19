import dimforge_rapier3d_compat/areas/main as rapier
import dimforge_rapier3d_compat/areas/main/collider_desc
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
import gleam/option.{type Option, None, Some}

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

pub fn main() {
  rp_rt.promise_then(rapier.init(), fn(ready) {
    let assert Ok(_) = ready
    let two_pi = 6.283185307179586
    let dom_hx = 0.13
    let dom_hy = 0.85

    let doc = core.get_document()
    let assert Some(canvas_el) = document.get_element_by_id(doc, "canvas")
    let assert Some(ctx) = html_canvas_element.get_context_2d(coerce(canvas_el), None)
    let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
    let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

    let world_cell = new_cell(None)
    let dominoes_cell: Cell(List(rt.RigidBody(Nil))) = new_cell([])
    let striker_cell: Cell(Option(rt.RigidBody(Nil))) = new_cell(None)
    let trail_cell: Cell(List(#(Float, Float))) = new_cell([])
    let step_cell = new_cell(0)
    let paused_cell = new_cell(False)

    let build_scene = fn() {
      case get_cell(world_cell) {
        Some(old_w) -> world.free(old_w)
        None -> Nil
      }
      let w = world.new(vec(0.0, -9.81, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

      let floor = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.fixed(), 0.0, -0.35, 0.0))
      let _ = world.create_collider(w, collider_desc.set_friction_2(collider_desc.cuboid(7.2, 0.35, 2.0), 0.8), Some(floor))

      let doms =
        list.map([0, 1, 2, 3, 4, 5, 6, 7, 8], fn(i) {
          let x = -2.4 +. int.to_float(i) *. 0.88
          let rb = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.dynamic_2(), x, dom_hy, 0.0))
          let cd =
            collider_desc.cuboid(dom_hx, dom_hy, 0.35)
            |> collider_desc.set_friction_2(0.55)
            |> collider_desc.set_restitution_2(0.15)
            |> collider_desc.set_density_2(1.0)
          let _ = world.create_collider(w, cd, Some(rb))
          rb
        })

      let sb = world.create_rigid_body(w, rigid_body_desc.set_translation_2(rigid_body_desc.dynamic_2(), -5.3, 1.05, 0.0))
      let scd =
        collider_desc.ball(0.40)
        |> collider_desc.set_density_2(2.8)
        |> collider_desc.set_restitution_2(0.45)
        |> collider_desc.set_friction_2(0.25)
      let _ = world.create_collider(w, scd, Some(sb))
      rigid_body.apply_impulse(sb, vec(4.8, 0.2, 0.0), True)

      set_cell(world_cell, Some(w))
      set_cell(dominoes_cell, doms)
      set_cell(striker_cell, Some(sb))
      set_cell(trail_cell, [])
      set_cell(step_cell, 0)
    }

    let step_sim = fn() {
      case get_cell(world_cell) {
        Some(w) -> world.step(w, None, None)
        None -> Nil
      }
      set_cell(step_cell, get_cell(step_cell) + 1)
      case get_cell(striker_cell) {
        Some(sb) -> {
          let sp = rigid_body.translation(sb, None)
          set_cell(trail_cell, list.take([#(vector.get_x(sp), vector.get_y(sp)), ..get_cell(trail_cell)], 22))
        }
        None -> Nil
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

      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#f7f7f7"))
      ctx2d.fill_rect(ctx, 0.0, 350.0, 680.0, 50.0)
      ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
      ctx2d.set_line_width(ctx, 1.5)
      ctx2d.begin_path(ctx)
      ctx2d.move_to(ctx, 0.0, 350.0)
      ctx2d.line_to(ctx, 680.0, 350.0)
      ctx2d.stroke(ctx)

      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#b5b5b5"))
      list.each(get_cell(trail_cell), fn(pt) {
        let #(tx, ty) = pt
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 2.0, 0.0, two_pi, None)
        ctx2d.fill(ctx, None)
      })

      let toppled_cell = new_cell(0)
      list.each(get_cell(dominoes_cell), fn(rb) {
        let p = rigid_body.translation(rb, None)
        let q = rigid_body.rotation(rb, None)
        let qz = rotation.get_z(q)
        let tilted = abs(qz) >. 0.25
        case tilted {
          True -> set_cell(toppled_cell, get_cell(toppled_cell) + 1)
          False -> Nil
        }
        draw_rot_box(
          ctx,
          vector.get_x(p),
          vector.get_y(p),
          rotation.get_x(q),
          rotation.get_y(q),
          qz,
          rotation.get_w(q),
          dom_hx,
          dom_hy,
          case tilted { True -> "#d4d4d4" False -> "#f7f7f7" },
        )
      })

      let spx_cell = new_cell(0.0)
      case get_cell(striker_cell) {
        Some(sb) -> {
          let sp = rigid_body.translation(sb, None)
          let px = vector.get_x(sp)
          set_cell(spx_cell, px)
          let scx = to_cx(px)
          let scy = to_cy(vector.get_y(sp))
          ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#262626"))
          ctx2d.begin_path(ctx)
          ctx2d.arc(ctx, scx, scy, 0.40 *. 46.0, 0.0, two_pi, None)
          ctx2d.fill(ctx, None)
          ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
          ctx2d.set_line_width(ctx, 1.6)
          ctx2d.begin_path(ctx)
          ctx2d.arc(ctx, scx, scy, 0.40 *. 46.0, 0.0, two_pi, None)
          ctx2d.stroke(ctx)
          ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#e5e5e5"))
          ctx2d.begin_path(ctx)
          ctx2d.arc(ctx, scx -. 5.0, scy -. 5.0, 2.5, 0.0, two_pi, None)
          ctx2d.fill(ctx, None)
        }
        None -> Nil
      }

      let toppled = get_cell(toppled_cell)
      ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#555555"))
      ctx2d.set_font(ctx, "11px ui-monospace, monospace")
      ctx2d.fill_text(ctx, "Dominoes toppled: " <> int.to_string(toppled) <> " / 9", 18.0, 24.0, None)

      let p_state = case get_cell(paused_cell) {
        True -> "paused"
        False -> "running"
      }
      dom_node.set_text_content(
        meta_el,
        Some(
          "9 cuboids + striker sphere · toppled="
          <> int.to_string(toppled)
          <> "/9 · striker_x="
          <> fmt2(get_cell(spx_cell))
          <> " m · step="
          <> int.to_string(get_cell(step_cell))
          <> " ("
          <> p_state
          <> ")",
        ),
      )
    }

    build_scene()
    times(0, 22, fn(_) { step_sim() })
    render()

    let wire_click = fn(id: String, cb: fn() -> Nil) {
      let assert Some(b) = document.get_element_by_id(doc, id)
      html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
    }

    wire_click("btn-push", fn() {
      case get_cell(striker_cell) {
        Some(sb) -> rigid_body.apply_impulse(sb, vec(3.5, 0.6, 0.0), True)
        None -> Nil
      }
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
      times(0, 12, fn(_) { step_sim() })
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
