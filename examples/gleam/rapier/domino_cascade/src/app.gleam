import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/rigid_body
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
import gleam/list
import gleam/option.{type Option, None, Some}

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

pub fn main() {
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready
  let dom_hx = 0.13
  let dom_hy = 0.85

  let doc = core.get_document()
  let assert Some(canvas) = document.query_selector(doc, tag.canvas)
  let assert Some(ctx) = html_canvas_element.get_context_2d(canvas, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref = rp_rt.ref_new(None)
  let dominoes_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let striker_ref: rp_rt.Ref(Option(rt.RigidBody(Nil))) = rp_rt.ref_new(None)
  let trail_ref: rp_rt.Ref(List(#(Float, Float))) = rp_rt.ref_new([])
  let step_ref = rp_rt.ref_new(0)
  let paused_ref = rp_rt.ref_new(False)

  let build_scene = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w = world.new(vec(0.0, -9.81, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

    let floor =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed() |> rigid_body_desc.set_translation(0.0, -0.35, 0.0),
      )
    let _ =
      world.create_collider(
        w,
        collider_desc.cuboid(7.2, 0.35, 2.0) |> collider_desc.set_friction(0.8),
        Some(floor),
      )

    let doms =
      int.range(from: 0, to: 9, with: [], run: fn(acc, i) {
        let x = -2.4 +. int.to_float(i) *. 0.88
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(x, dom_hy, 0.0),
          )
        let cd =
          collider_desc.cuboid(dom_hx, dom_hy, 0.35)
          |> collider_desc.set_friction(0.55)
          |> collider_desc.set_restitution(0.15)
          |> collider_desc.set_density(1.0)
        let _ = world.create_collider(w, cd, Some(rb))
        [rb, ..acc]
      })
      |> list.reverse

    let sb =
      world.create_rigid_body(
        w,
        rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(-5.3, 1.05, 0.0),
      )
    let scd =
      collider_desc.ball(0.40)
      |> collider_desc.set_density(2.8)
      |> collider_desc.set_restitution(0.45)
      |> collider_desc.set_friction(0.25)
    let _ = world.create_collider(w, scd, Some(sb))
    rigid_body.apply_impulse(sb, vec(4.8, 0.2, 0.0), True)

    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(dominoes_ref, doms)
    rp_rt.ref_set(striker_ref, Some(sb))
    rp_rt.ref_set(trail_ref, [])
    rp_rt.ref_set(step_ref, 0)
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    let _ = rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    case rp_rt.ref_get(striker_ref) {
      Some(sb) -> {
        let sp = rigid_body.translation(sb, None)
        let _ =
          rp_rt.ref_update(trail_ref, fn(trail) {
            list.take([#(vector.get_x(sp), vector.get_y(sp)), ..trail], 22)
          })
        Nil
      }
      None -> Nil
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

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#f7f7f7"))
    ctx2d.fill_rect(ctx, 0.0, 350.0, 680.0, 50.0)
    ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
    ctx2d.set_line_width(ctx, 1.5)
    ctx2d.begin_path(ctx)
    ctx2d.move_to(ctx, 0.0, 350.0)
    ctx2d.line_to(ctx, 680.0, 350.0)
    ctx2d.stroke(ctx)

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#b5b5b5"))
    list.each(rp_rt.ref_get(trail_ref), fn(pt) {
      let #(tx, ty) = pt
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 2.0, 0.0, two_pi, None)
      ctx2d.fill(ctx, None)
    })

    let toppled =
      list.fold(rp_rt.ref_get(dominoes_ref), 0, fn(acc, rb) {
        let p = rigid_body.translation(rb, None)
        let q = rigid_body.rotation(rb, None)
        let qz = rotation.get_z(q)
        let tilted = float.absolute_value(qz) >. 0.25
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
        case tilted {
          True -> acc + 1
          False -> acc
        }
      })

    let spx = case rp_rt.ref_get(striker_ref) {
      Some(sb) -> {
        let sp = rigid_body.translation(sb, None)
        let px = vector.get_x(sp)
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
        px
      }
      None -> 0.0
    }

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#555555"))
    ctx2d.set_font(ctx, "11px ui-monospace, monospace")
    ctx2d.fill_text(ctx, "Dominoes toppled: " <> int.to_string(toppled) <> " / 9", 18.0, 24.0, None)

    let p_state = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "9 cuboids + striker sphere · toppled="
        <> int.to_string(toppled)
        <> "/9 · striker_x="
        <> fmt2(spx)
        <> " m · step="
        <> int.to_string(rp_rt.ref_get(step_ref))
        <> " ("
        <> p_state
        <> ")",
      ),
    )
  }

  build_scene()
  int.range(from: 0, to: 22, with: Nil, run: fn(_, _) { step_sim() })
  render()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-push", fn() {
    case rp_rt.ref_get(striker_ref) {
      Some(sb) -> rigid_body.apply_impulse(sb, vec(3.5, 0.6, 0.0), True)
      None -> Nil
    }
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
    int.range(from: 0, to: 12, with: Nil, run: fn(_, _) { step_sim() })
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
