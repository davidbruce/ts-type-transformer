import babylonjs_lite as b
import babylonjs_lite/box_options
import babylonjs_lite/create_pbr_material_props
import babylonjs_lite/directional_light
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_quat
import babylonjs_lite/observable_vec3
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/sphere_options
import babylonjs_lite/runtime as br
import babylonjs_lite/types as bt
import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/rigid_body
import dimforge_rapier3d_compat/rigid_body_desc
import dimforge_rapier3d_compat/rotation
import dimforge_rapier3d_compat/vector
import dimforge_rapier3d_compat/vector3
import dimforge_rapier3d_compat/world
import dimforge_rapier3d_compat/runtime as rp_rt
import dimforge_rapier3d_compat/types as rt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import dom/areas/webgpu/gpu_color_dict
import dom/types.{type HTMLElement}
import dom/areas/html/html_element_tag_name_map as tag
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const rack_count = 10

const ball_count = 11

const ball_radius = 0.36

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

fn set_status_error(status: Option(HTMLElement(Nil)), message: String) -> Nil {
  case status {
    Some(el) ->
      dom_node.set_text_content(el, Some("WebGPU / Babylon Lite error: " <> message))
    None -> Nil
  }
}

fn mk_pbr(
  r: Float,
  g: Float,
  bl: Float,
  metallic: Float,
  roughness: Float,
) -> bt.PbrMaterialProps(Nil) {
  let props =
    create_pbr_material_props.create()
    |> create_pbr_material_props.with_base_color_factor(#(r, g, bl, 1.0))
    |> create_pbr_material_props.with_metallic_factor(metallic)
    |> create_pbr_material_props.with_roughness_factor(roughness)
  b.create_pbr_material(Some(props))
}

pub fn main() -> Nil {
  let doc = core.get_document()
  let status = document.get_element_by_id(doc, "status")
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready

  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref: rp_rt.Ref(Option(rt.World(Nil))) = rp_rt.ref_new(None)
  let balls_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let handles_ref: rp_rt.Ref(List(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil)))) =
    rp_rt.ref_new([])
  let paused_ref = rp_rt.ref_new(False)
  let step_ref = rp_rt.ref_new(0)

  let build_world = fn(shoot_cue: Bool) {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w =
      world.new(
        vector3.new(0.0, -9.81, 0.0),
        None, None, None, None, None, None, None, None, None, None, None, None, None,
      )

    let bed =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed()
        |> rigid_body_desc.set_translation(0.0, -0.25, 0.0),
      )
    world.create_collider(
      w,
      collider_desc.cuboid(4.6, 0.25, 7.6)
      |> collider_desc.set_friction(0.14)
      |> collider_desc.set_restitution(0.35),
      Some(bed),
    )

    let rails = [
      #(-4.35, 0.45, 0.0, 0.25, 0.45, 7.6),
      #(4.35, 0.45, 0.0, 0.25, 0.45, 7.6),
      #(0.0, 0.45, -7.35, 4.6, 0.45, 0.25),
      #(0.0, 0.45, 7.35, 4.6, 0.45, 0.25),
    ]
    list.each(rails, fn(r) {
      let #(rx, ry, rz, hx, hy, hz) = r
      let rb =
        world.create_rigid_body(
          w,
          rigid_body_desc.fixed()
          |> rigid_body_desc.set_translation(rx, ry, rz),
        )
      world.create_collider(
        w,
        collider_desc.cuboid(hx, hy, hz)
        |> collider_desc.set_friction(0.12)
        |> collider_desc.set_restitution(0.88),
        Some(rb),
      )
      Nil
    })

    let rack =
      int.range(from: 0, to: 4, with: [], run: fn(acc, row) {
        int.range(from: 0, to: row + 1, with: acc, run: fn(inner_acc, col) {
          let x = { int.to_float(col) -. int.to_float(row) *. 0.5 } *. 0.74
          let z = -1.6 -. int.to_float(row) *. 0.645
          let rb =
            world.create_rigid_body(
              w,
              rigid_body_desc.dynamic()
              |> rigid_body_desc.set_translation(x, ball_radius, z)
              |> rigid_body_desc.set_linear_damping(0.30)
              |> rigid_body_desc.set_angular_damping(0.30),
            )
          world.create_collider(
            w,
            collider_desc.ball(ball_radius)
            |> collider_desc.set_restitution(0.92)
            |> collider_desc.set_friction(0.10)
            |> collider_desc.set_density(1.0),
            Some(rb),
          )
          [rb, ..inner_acc]
        })
      })
      |> list.reverse

    let base_cue =
      rigid_body_desc.dynamic()
      |> rigid_body_desc.set_translation(0.04, ball_radius, 4.4)
      |> rigid_body_desc.set_linear_damping(0.22)
      |> rigid_body_desc.set_angular_damping(0.22)
    let cue_desc = case shoot_cue {
      True -> rigid_body_desc.set_linvel(base_cue, -0.18, 0.0, -16.0)
      False -> base_cue
    }
    let cue = world.create_rigid_body(w, cue_desc)
    world.create_collider(
      w,
      collider_desc.ball(ball_radius)
      |> collider_desc.set_restitution(0.94)
      |> collider_desc.set_friction(0.10)
      |> collider_desc.set_density(1.25),
      Some(cue),
    )

    rp_rt.ref_set(balls_ref, list.append(rack, [cue]))
    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(step_ref, 0)
  }

  let sync_meshes = fn() {
    list.each(
      list.zip(rp_rt.ref_get(balls_ref), rp_rt.ref_get(handles_ref)),
      fn(item) {
        let #(rb, #(pos, rot)) = item
        let p = rigid_body.translation(rb, None)
        let q = rigid_body.rotation(rb, None)
        observable_vec3.set(pos, vector.get_x(p), vector.get_y(p), vector.get_z(p))
        observable_quat.set(
          rot,
          rotation.get_x(q),
          rotation.get_y(q),
          rotation.get_z(q),
          rotation.get_w(q),
        )
      },
    )
  }

  let update_meta = fn() {
    let #(moving, ke) =
      list.fold(rp_rt.ref_get(balls_ref), #(0, 0.0), fn(acc, rb) {
        let #(cur_m, cur_ke) = acc
        let v = rigid_body.linvel(rb, None)
        let vx = vector.get_x(v)
        let vy = vector.get_y(v)
        let vz = vector.get_z(v)
        let s2 = vx *. vx +. vy *. vy +. vz *. vz
        let next_m = case s2 >. 0.05 {
          True -> cur_m + 1
          False -> cur_m
        }
        #(next_m, cur_ke +. 0.5 *. s2)
      })
    let state_text = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "11 spheres · moving="
        <> int.to_string(moving)
        <> " · ke="
        <> fmt1(ke)
        <> " · step="
        <> int.to_string(rp_rt.ref_get(step_ref))
        <> " ("
        <> state_text
        <> ")",
      ),
    )
  }

  let step_sim = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    sync_meshes()
    update_meta()
  }

  build_world(True)
  int.range(from: 0, to: 26, with: Nil, run: fn(_, _) {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    Nil
  })
  update_meta()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(btn) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(btn, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-break", fn() {
    rp_rt.ref_set(paused_ref, False)
    dom_node.set_text_content(pause_btn, Some("Pause"))
    build_world(True)
    sync_meshes()
    update_meta()
  })

  wire_click("btn-scatter", fn() {
    list.index_fold(rp_rt.ref_get(balls_ref), Nil, fn(_, rb, idx) {
      let ang = int.to_float(idx) *. 1.15
      rigid_body.apply_impulse(rb, vector3.new(cos(ang) *. 1.4, 0.25, sin(ang) *. 1.4), True)
    })
    update_meta()
  })

  wire_click("btn-pause", fn() {
    let p = rp_rt.ref_update(paused_ref, fn(cur) { !cur })
    dom_node.set_text_content(
      pause_btn,
      Some(case p {
        True -> "Resume"
        False -> "Pause"
      }),
    )
    update_meta()
  })

  wire_click("btn-reset", fn() {
    rp_rt.ref_set(paused_ref, False)
    dom_node.set_text_content(pause_btn, Some("Pause"))
    build_world(False)
    sync_meshes()
    update_meta()
  })

  case document.query_selector(doc, tag.canvas) {
    None -> set_status_error(status, "missing <canvas> element")
    Some(canvas) -> {
      let on_err = fn(err) { set_status_error(status, br.js_error_message(err)) }
      use engine <- br.promise_try(
        b.create_engine_with_engine_options(
          bt.RenderCanvasHTMLCanvasElement(canvas),
          None,
        ),
        on_err,
      )

      let scene = b.create_scene_context(engine, None)
      let clear =
        gpu_color_dict.create()
        |> gpu_color_dict.with_r(0.05)
        |> gpu_color_dict.with_g(0.07)
        |> gpu_color_dict.with_b(0.09)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera = b.create_arc_rotate_camera(1.18, 0.92, 13.5, b.vec3(0.0, 0.3, -0.4))
      b.add_to_scene(scene, br.dynamic(camera))
      b.attach_control(camera, canvas, Some(scene), None)

      let hemi = b.create_hemispheric_light(Some(#(0.0, 1.0, 0.2)), Some(0.72))
      hemispheric_light.set_diffuse_color(hemi, #(0.85, 0.9, 0.95))
      hemispheric_light.set_ground_color(hemi, #(0.12, 0.14, 0.16))
      b.add_to_scene(scene, br.dynamic(hemi))

      let dir = b.create_directional_light(#(-1.0, -2.0, -1.0), Some(2.1))
      directional_light.set_diffuse(dir, #(1.0, 0.97, 0.92))
      b.add_to_scene(scene, br.dynamic(dir))

      let table_opts =
        box_options.create()
        |> box_options.with_width(9.2)
        |> box_options.with_height(0.5)
        |> box_options.with_depth(15.2)
      let table = b.create_box(engine, Some(bt.NumberOrBoxOptionsBoxOptions(table_opts)))
      observable_vec3.set(scene_node.get_position(table), 0.0, -0.25, 0.0)
      mesh.set_material(table, mk_pbr(0.08, 0.24, 0.18, 0.05, 0.78))
      b.add_to_scene(scene, br.dynamic(table))

      let rail_mat = mk_pbr(0.18, 0.18, 0.20, 0.35, 0.42)
      let rail_meshes = [
        #(-4.35, 0.45, 0.0, 0.5, 0.9, 15.2),
        #(4.35, 0.45, 0.0, 0.5, 0.9, 15.2),
        #(0.0, 0.45, -7.35, 9.2, 0.9, 0.5),
        #(0.0, 0.45, 7.35, 9.2, 0.9, 0.5),
      ]
      list.each(rail_meshes, fn(rm) {
        let #(rx, ry, rz, wr, hr, dr) = rm
        let ropts =
          box_options.create()
          |> box_options.with_width(wr)
          |> box_options.with_height(hr)
          |> box_options.with_depth(dr)
        let r = b.create_box(engine, Some(bt.NumberOrBoxOptionsBoxOptions(ropts)))
        observable_vec3.set(scene_node.get_position(r), rx, ry, rz)
        mesh.set_material(r, rail_mat)
        b.add_to_scene(scene, br.dynamic(r))
      })

      let cue_mat = mk_pbr(0.96, 0.96, 0.95, 0.15, 0.18)
      let m0 = mk_pbr(0.85, 0.24, 0.22, 0.2, 0.22)
      let m1 = mk_pbr(0.22, 0.48, 0.85, 0.2, 0.22)
      let m2 = mk_pbr(0.92, 0.68, 0.18, 0.2, 0.22)
      let m3 = mk_pbr(0.20, 0.20, 0.22, 0.25, 0.2)

      let sopts =
        sphere_options.create()
        |> sphere_options.with_diameter(ball_radius *. 2.0)
        |> sphere_options.with_segments(24.0)
      let handles =
        int.range(from: 0, to: ball_count, with: [], run: fn(acc, i) {
          let m = b.create_sphere(engine, Some(sopts))
          let mat = case i == rack_count {
            True -> cue_mat
            False ->
              case i % 4 {
                0 -> m0
                1 -> m1
                2 -> m2
                _ -> m3
              }
          }
          mesh.set_material(m, mat)
          b.add_to_scene(scene, br.dynamic(m))
          [
            #(scene_node.get_position(m), scene_node.get_rotation_quaternion(m)),
            ..acc
          ]
        })
        |> list.reverse
      rp_rt.ref_set(handles_ref, handles)

      sync_meshes()

      b.on_before_render(scene, fn(_) {
        case !rp_rt.ref_get(paused_ref) {
          True -> step_sim()
          False -> Nil
        }
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
