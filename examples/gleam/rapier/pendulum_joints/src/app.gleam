import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/joint_data
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

fn draw_pivot(ctx: CanvasRenderingContext2D(Nil), wx: Float, wy: Float) -> Nil {
  let cx = to_cx(wx)
  let cy = to_cy(wy)
  ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#ffffff"))
  ctx2d.begin_path(ctx)
  ctx2d.arc(ctx, cx, cy, 4.0, 0.0, two_pi, None)
  ctx2d.fill(ctx, None)
  ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
  ctx2d.set_line_width(ctx, 1.6)
  ctx2d.begin_path(ctx)
  ctx2d.arc(ctx, cx, cy, 4.0, 0.0, two_pi, None)
  ctx2d.stroke(ctx)
}

pub fn main() {
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready
  let link_hx = 0.14
  let link_hy = 0.62

  let doc = core.get_document()
  let assert Some(canvas) = document.query_selector(doc, tag.canvas)
  let assert Some(ctx) = html_canvas_element.get_context_2d(canvas, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref = rp_rt.ref_new(None)
  let chain_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let arm_ref: rp_rt.Ref(Option(rt.RigidBody(Nil))) = rp_rt.ref_new(None)
  let bob_ref: rp_rt.Ref(Option(rt.RigidBody(Nil))) = rp_rt.ref_new(None)
  let trail_a_ref: rp_rt.Ref(List(#(Float, Float))) = rp_rt.ref_new([])
  let trail_b_ref: rp_rt.Ref(List(#(Float, Float))) = rp_rt.ref_new([])
  let step_ref = rp_rt.ref_new(0)
  let paused_ref = rp_rt.ref_new(False)

  let build_scene = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w = world.new(vec(0.0, -9.81, 0.0), None, None, None, None, None, None, None, None, None, None, None, None, None)

    let gantry =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed() |> rigid_body_desc.set_translation(0.0, 6.2, 0.0),
      )
    let _ = world.create_collider(w, collider_desc.cuboid(6.0, 0.22, 0.6), Some(gantry))
    let z_axis = vec(0.0, 0.0, 1.0)

    let add_link = fn(x: Float, y: Float, density: Float) {
      let rb =
        world.create_rigid_body(
          w,
          rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(x, y, 0.0),
        )
      let _ =
        world.create_collider(
          w,
          collider_desc.cuboid(link_hx, link_hy, 0.2) |> collider_desc.set_density(density),
          Some(rb),
        )
      rb
    }

    let l1 = add_link(-2.8, 5.4, 1.4)
    let l2 = add_link(-3.3, 4.1, 1.4)
    let l3 = add_link(-3.8, 2.8, 1.6)

    let _ = world.create_impulse_joint(w, joint_data.revolute(vec(-2.2, -0.22, 0.0), vec(0.0, link_hy +. 0.06, 0.0), z_axis), gantry, l1, True)
    let _ = world.create_impulse_joint(w, joint_data.revolute(vec(0.0, 0.0 -. link_hy -. 0.06, 0.0), vec(0.0, link_hy +. 0.06, 0.0), z_axis), l1, l2, True)
    let _ = world.create_impulse_joint(w, joint_data.revolute(vec(0.0, 0.0 -. link_hy -. 0.06, 0.0), vec(0.0, link_hy +. 0.06, 0.0), z_axis), l2, l3, True)
    rigid_body.apply_impulse(l3, vec(-1.4, 0.0, 0.0), True)

    let arm =
      world.create_rigid_body(
        w,
        rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(3.1, 5.2, 0.0),
      )
    let _ =
      world.create_collider(
        w,
        collider_desc.cuboid(0.14, 0.85, 0.2) |> collider_desc.set_density(1.2),
        Some(arm),
      )
    let bob =
      world.create_rigid_body(
        w,
        rigid_body_desc.dynamic() |> rigid_body_desc.set_translation(3.8, 3.6, 0.0),
      )
    let _ =
      world.create_collider(
        w,
        collider_desc.ball(0.42) |> collider_desc.set_density(3.0),
        Some(bob),
      )

    let _ = world.create_impulse_joint(w, joint_data.revolute(vec(2.3, -0.22, 0.0), vec(0.0, 0.88, 0.0), z_axis), gantry, arm, True)
    let _ = world.create_impulse_joint(w, joint_data.revolute(vec(0.0, -0.88, 0.0), vec(0.0, 0.72, 0.0), z_axis), arm, bob, True)
    rigid_body.apply_impulse(bob, vec(1.8, 0.2, 0.0), True)

    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(chain_ref, [l1, l2, l3])
    rp_rt.ref_set(arm_ref, Some(arm))
    rp_rt.ref_set(bob_ref, Some(bob))
    rp_rt.ref_set(trail_a_ref, [])
    rp_rt.ref_set(trail_b_ref, [])
    rp_rt.ref_set(step_ref, 0)
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    let _ = rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    case rp_rt.ref_get(chain_ref), rp_rt.ref_get(bob_ref) {
      [_, _, l3], Some(bob) -> {
        let pa = rigid_body.translation(l3, None)
        let pb = rigid_body.translation(bob, None)
        let _ = rp_rt.ref_update(trail_a_ref, fn(t) { list.take([#(vector.get_x(pa), vector.get_y(pa)), ..t], 36) })
        let _ = rp_rt.ref_update(trail_b_ref, fn(t) { list.take([#(vector.get_x(pb), vector.get_y(pb)), ..t], 36) })
        Nil
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
      ctx2d.line_to(ctx, x, 400.0)
      ctx2d.stroke(ctx)
    })
    int.range(from: 0, to: 8, with: Nil, run: fn(_, gy) {
      let y = to_cy(int.to_float(gy))
      ctx2d.begin_path(ctx)
      ctx2d.move_to(ctx, 0.0, y)
      ctx2d.line_to(ctx, 680.0, y)
      ctx2d.stroke(ctx)
    })

    draw_rot_box(ctx, 0.0, 6.2, 0.0, 0.0, 0.0, 1.0, 6.0, 0.22, "#efefef")

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#b5b5b5"))
    list.each(rp_rt.ref_get(trail_a_ref), fn(pt) {
      let #(tx, ty) = pt
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 1.6, 0.0, two_pi, None)
      ctx2d.fill(ctx, None)
    })
    list.each(rp_rt.ref_get(trail_b_ref), fn(pt) {
      let #(tx, ty) = pt
      ctx2d.begin_path(ctx)
      ctx2d.arc(ctx, to_cx(tx), to_cy(ty), 1.6, 0.0, two_pi, None)
      ctx2d.fill(ctx, None)
    })

    let #(#(tx, ty), pins) =
      list.index_fold(rp_rt.ref_get(chain_ref), #(#(0.0, 0.0), []), fn(acc, rb, i) {
        let #(cur_tip, cur_pins) = acc
        let p = rigid_body.translation(rb, None)
        let q = rigid_body.rotation(rb, None)
        let px = vector.get_x(p)
        let py = vector.get_y(p)
        let qx = rotation.get_x(q)
        let qy = rotation.get_y(q)
        let qz = rotation.get_z(q)
        let qw = rotation.get_w(q)
        draw_rot_box(ctx, px, py, qx, qy, qz, qw, link_hx, link_hy, case i == 2 { True -> "#d4d4d4" False -> "#f7f7f7" })
        case i < 2 {
          True -> {
            let r01 = 2.0 *. { qx *. qy -. qw *. qz }
            let r11 = 1.0 -. 2.0 *. { qx *. qx +. qz *. qz }
            #(cur_tip, [#(px -. r01 *. { link_hy +. 0.06 }, py -. r11 *. { link_hy +. 0.06 }), ..cur_pins])
          }
          False -> #(#(px, py), cur_pins)
        }
      })

    let #(bx, by) = case rp_rt.ref_get(arm_ref), rp_rt.ref_get(bob_ref) {
      Some(arm), Some(bob) -> {
        let ap = rigid_body.translation(arm, None)
        let aq = rigid_body.rotation(arm, None)
        let apx = vector.get_x(ap)
        let apy = vector.get_y(ap)
        let aqx = rotation.get_x(aq)
        let aqy = rotation.get_y(aq)
        let aqz = rotation.get_z(aq)
        let aqw = rotation.get_w(aq)
        draw_rot_box(ctx, apx, apy, aqx, aqy, aqz, aqw, 0.14, 0.85, "#f7f7f7")

        let bp = rigid_body.translation(bob, None)
        let bpx = vector.get_x(bp)
        let bpy = vector.get_y(bp)
        let bcx = to_cx(bpx)
        let bcy = to_cy(bpy)
        ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#262626"))
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, bcx, bcy, 0.42 *. 46.0, 0.0, two_pi, None)
        ctx2d.fill(ctx, None)
        ctx2d.set_stroke_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#111111"))
        ctx2d.set_line_width(ctx, 1.6)
        ctx2d.begin_path(ctx)
        ctx2d.arc(ctx, bcx, bcy, 0.42 *. 46.0, 0.0, two_pi, None)
        ctx2d.stroke(ctx)

        let arm_r01 = 2.0 *. { aqx *. aqy -. aqw *. aqz }
        let arm_r11 = 1.0 -. 2.0 *. { aqx *. aqx +. aqz *. aqz }
        draw_pivot(ctx, apx -. arm_r01 *. 0.88, apy -. arm_r11 *. 0.88)
        #(bpx, bpy)
      }
      _, _ -> #(0.0, 0.0)
    }

    draw_pivot(ctx, -2.2, 5.98)
    list.each(pins, fn(pt) {
      let #(px, py) = pt
      draw_pivot(ctx, px, py)
    })
    draw_pivot(ctx, 2.3, 5.98)

    ctx2d.set_fill_style(ctx, StringOrCanvasGradientOrCanvasPatternString("#555555"))
    ctx2d.set_font(ctx, "11px ui-monospace, monospace")
    ctx2d.fill_text(ctx, "3-Link Revolute Chain", to_cx(-3.5), 36.0, None)
    ctx2d.fill_text(ctx, "2-Body Bob Pendulum", to_cx(1.1), 36.0, None)

    let p_state = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "5 revolute joints · chain_tip=("
        <> fmt2(tx)
        <> ", "
        <> fmt2(ty)
        <> ") · bob=("
        <> fmt2(bx)
        <> ", "
        <> fmt2(by)
        <> ") · step="
        <> int.to_string(rp_rt.ref_get(step_ref))
        <> " ("
        <> p_state
        <> ")",
      ),
    )
  }

  build_scene()
  int.range(from: 0, to: 20, with: Nil, run: fn(_, _) { step_sim() })
  render()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-kick", fn() {
    case rp_rt.ref_get(chain_ref), rp_rt.ref_get(bob_ref) {
      [_, _, l3], Some(bob) -> {
        rigid_body.apply_impulse(l3, vec(1.6, 0.3, 0.0), True)
        rigid_body.apply_impulse(bob, vec(-2.0, 0.4, 0.0), True)
      }
      _, _ -> Nil
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
    int.range(from: 0, to: 15, with: Nil, run: fn(_, _) { step_sim() })
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
