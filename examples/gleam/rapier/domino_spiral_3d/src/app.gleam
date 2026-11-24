import babylonjs_lite as b
import babylonjs_lite/box_options
import babylonjs_lite/create_pbr_material_props
import babylonjs_lite/cylinder_options
import babylonjs_lite/directional_light
import babylonjs_lite/ground_options
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_quat
import babylonjs_lite/observable_vec3
import babylonjs_lite/runtime as br
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/sphere_options
import babylonjs_lite/types as bt
import dimforge_rapier3d_compat as rapier
import dimforge_rapier3d_compat/collider_desc
import dimforge_rapier3d_compat/quaternion
import dimforge_rapier3d_compat/rigid_body
import dimforge_rapier3d_compat/rigid_body_desc
import dimforge_rapier3d_compat/rotation
import dimforge_rapier3d_compat/runtime as rp_rt
import dimforge_rapier3d_compat/types as rt
import dimforge_rapier3d_compat/vector
import dimforge_rapier3d_compat/vector3
import dimforge_rapier3d_compat/world
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import dom/areas/html/html_element_tag_name_map as tag
import dom/areas/webgpu/gpu_color_dict
import dom/types.{type HTMLElement}
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const dom_count = 14

const cyl_count = 3

const dom_hx = 0.09

const dom_hy = 0.65

const dom_hz = 0.34

const marble_radius = 0.36

const ramp_angle = -0.38

const ramp_x = -3.55

const ramp_y = 1.02

const ramp_z = -2.5

fn spiral_pose(i: Int) -> #(Float, Float, Float, Float) {
  let fi = int.to_float(i)
  let t = fi *. 0.255
  let r = 2.6 -. fi *. 0.045
  let x = -1.35 +. r *. sin(t)
  let z = -2.5 +. 2.6 -. r *. cos(t)
  #(x, dom_hy, z, 0.0 -. t)
}

fn set_status_error(status: Option(HTMLElement(Nil)), message: String) -> Nil {
  case status {
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some("WebGPU / Babylon Lite error: " <> message),
      )
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
  b.create_pbr_material_with(Some(props))
}

fn body_ke(rb: rt.RigidBody(Nil)) -> Float {
  let v = rigid_body.linvel(rb)
  let vx = vector.get_x(v)
  let vy = vector.get_y(v)
  let vz = vector.get_z(v)
  0.5 *. { vx *. vx +. vy *. vy +. vz *. vz }
}

pub fn main() -> Nil {
  let doc = core.get_document()
  let status = document.get_element_by_id(doc, "status")
  use ready <- rp_rt.promise_then(rapier.init())
  let assert Ok(_) = ready

  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let assert Some(pause_btn) = document.get_element_by_id(doc, "btn-pause")

  let world_ref: rp_rt.Ref(Option(rt.World(Nil))) = rp_rt.ref_new(None)
  let marble_ref: rp_rt.Ref(Option(rt.RigidBody(Nil))) = rp_rt.ref_new(None)
  let doms_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let cyls_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let marble_handle_ref: rp_rt.Ref(
    Option(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil))),
  ) = rp_rt.ref_new(None)
  let dom_handles_ref: rp_rt.Ref(
    List(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil))),
  ) = rp_rt.ref_new([])
  let cyl_handles_ref: rp_rt.Ref(
    List(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil))),
  ) = rp_rt.ref_new([])
  let paused_ref = rp_rt.ref_new(False)
  let step_ref = rp_rt.ref_new(0)

  let build_world = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w = world.new(vector3.new(0.0, -9.81, 0.0))

    let floor_body =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed()
          |> rigid_body_desc.set_translation(0.0, -0.3, 0.0),
      )
    world.create_collider_with(
      w,
      collider_desc.cuboid(10.0, 0.3, 10.0)
        |> collider_desc.set_friction(0.75)
        |> collider_desc.set_restitution(0.1),
      Some(floor_body),
    )

    let ramp_rot =
      quaternion.new(0.0, 0.0, sin(ramp_angle *. 0.5), cos(ramp_angle *. 0.5))
    let ramp_body =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed()
          |> rigid_body_desc.set_translation(ramp_x, ramp_y, ramp_z)
          |> rigid_body_desc.set_rotation(ramp_rot),
      )
    world.create_collider_with(
      w,
      collider_desc.cuboid(1.75, 0.1, 0.65)
        |> collider_desc.set_friction(0.15)
        |> collider_desc.set_restitution(0.2),
      Some(ramp_body),
    )

    let marble =
      world.create_rigid_body(
        w,
        rigid_body_desc.dynamic()
          |> rigid_body_desc.set_translation(-4.75, 2.02, -2.5)
          |> rigid_body_desc.set_linvel(3.0, 0.0, 0.0),
      )
    world.create_collider_with(
      w,
      collider_desc.ball(marble_radius)
        |> collider_desc.set_density(3.2)
        |> collider_desc.set_restitution(0.32)
        |> collider_desc.set_friction(0.18),
      Some(marble),
    )

    let doms =
      int.range(from: 0, to: dom_count, with: [], run: fn(acc, i) {
        let #(x, y, z, yaw) = spiral_pose(i)
        let q = quaternion.new(0.0, sin(yaw *. 0.5), 0.0, cos(yaw *. 0.5))
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic()
              |> rigid_body_desc.set_translation(x, y, z)
              |> rigid_body_desc.set_rotation(q),
          )
        world.create_collider_with(
          w,
          collider_desc.cuboid(dom_hx, dom_hy, dom_hz)
            |> collider_desc.set_friction(0.55)
            |> collider_desc.set_restitution(0.18)
            |> collider_desc.set_density(1.0),
          Some(rb),
        )
        [rb, ..acc]
      })
      |> list.reverse

    let #(end_x, _, end_z, end_yaw) = spiral_pose(dom_count - 1)
    let fwd_x = cos(0.0 -. end_yaw)
    let fwd_z = sin(0.0 -. end_yaw)
    let perp_x = 0.0 -. fwd_z
    let perp_z = fwd_x
    let cyls =
      int.range(from: 0, to: cyl_count, with: [], run: fn(acc, j) {
        let spread = { int.to_float(j) -. 1.0 } *. 0.52
        let cx = end_x +. fwd_x *. 0.72 +. perp_x *. spread
        let cz = end_z +. fwd_z *. 0.72 +. perp_z *. spread
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic()
              |> rigid_body_desc.set_translation(cx, 0.42, cz),
          )
        world.create_collider_with(
          w,
          collider_desc.cylinder(0.42, 0.25)
            |> collider_desc.set_friction(0.55)
            |> collider_desc.set_restitution(0.25)
            |> collider_desc.set_density(1.0),
          Some(rb),
        )
        [rb, ..acc]
      })
      |> list.reverse

    rp_rt.ref_set(marble_ref, Some(marble))
    rp_rt.ref_set(doms_ref, doms)
    rp_rt.ref_set(cyls_ref, cyls)
    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(step_ref, 0)
  }

  let sync_pair = fn(
    rb: rt.RigidBody(Nil),
    pair: #(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil)),
  ) {
    let #(pos, rot) = pair
    let p = rigid_body.translation(rb)
    let q = rigid_body.rotation(rb)
    observable_vec3.set(pos, vector.get_x(p), vector.get_y(p), vector.get_z(p))
    observable_quat.set(
      rot,
      rotation.get_x(q),
      rotation.get_y(q),
      rotation.get_z(q),
      rotation.get_w(q),
    )
  }

  let sync_meshes = fn() {
    case rp_rt.ref_get(marble_ref), rp_rt.ref_get(marble_handle_ref) {
      Some(rb), Some(pair) -> sync_pair(rb, pair)
      _, _ -> Nil
    }
    list.each(
      list.zip(rp_rt.ref_get(doms_ref), rp_rt.ref_get(dom_handles_ref)),
      fn(item) { sync_pair(item.0, item.1) },
    )
    list.each(
      list.zip(rp_rt.ref_get(cyls_ref), rp_rt.ref_get(cyl_handles_ref)),
      fn(item) { sync_pair(item.0, item.1) },
    )
  }

  let update_meta = fn() {
    let m_ke = case rp_rt.ref_get(marble_ref) {
      Some(mb) -> body_ke(mb)
      None -> 0.0
    }
    let #(fallen, dom_ke) =
      list.fold(rp_rt.ref_get(doms_ref), #(0, 0.0), fn(acc, rb) {
        let #(cur_f, cur_k) = acc
        let p = rigid_body.translation(rb)
        let next_f = case vector.get_y(p) <. 0.48 {
          True -> cur_f + 1
          False -> cur_f
        }
        #(next_f, cur_k +. body_ke(rb))
      })
    let total_ke =
      list.fold(rp_rt.ref_get(cyls_ref), m_ke +. dom_ke, fn(acc, rb) {
        acc +. body_ke(rb)
      })
    let state_text = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "18 bodies (marble + 14 dominoes + 3 cylinders) · fallen="
        <> int.to_string(fallen)
        <> " · ke="
        <> rp_rt.to_fixed(total_ke, 1)
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
      Some(w) -> world.step(w)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    sync_meshes()
    update_meta()
  }

  build_world()
  int.range(from: 0, to: 20, with: Nil, run: fn(_, _) {
    case rp_rt.ref_get(world_ref) {
      Some(w) -> world.step(w)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    Nil
  })
  update_meta()

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(btn) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(btn, ev.click, fn(_) { cb() })
  }

  wire_click("btn-launch", fn() {
    case rp_rt.ref_get(marble_ref) {
      Some(mb) -> {
        rigid_body.set_translation(mb, vector3.new(-4.75, 2.02, -2.5), True)
        rigid_body.set_linvel(mb, vector3.new(3.0, 0.0, 0.0), True)
        rigid_body.set_angvel(mb, vector3.new(0.0, 0.0, 0.0), True)
      }
      None -> Nil
    }
    sync_meshes()
    update_meta()
  })

  wire_click("btn-nudge", fn() {
    case rp_rt.ref_get(doms_ref) {
      [first, ..] ->
        rigid_body.apply_impulse(first, vector3.new(1.6, 0.0, 0.0), True)
      [] -> Nil
    }
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
    build_world()
    sync_meshes()
    update_meta()
  })

  case document.query_selector(doc, tag.canvas) {
    None -> set_status_error(status, "missing <canvas> element")
    Some(canvas) -> {
      let on_err = fn(err) {
        set_status_error(status, br.js_error_message(err))
      }
      use engine <- br.promise_try(
        b.create_engine(bt.RenderCanvasHTMLCanvasElement(canvas)),
        on_err,
      )

      let scene = b.create_scene_context(engine)
      let clear =
        gpu_color_dict.create()
        |> gpu_color_dict.with_r(0.06)
        |> gpu_color_dict.with_g(0.07)
        |> gpu_color_dict.with_b(0.09)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(0.65, 1.05, 12.0, b.vec3(-0.4, 0.8, 0.0))
      b.add_to_scene(scene, bt.scene_entity_camera(camera))
      b.attach_control_with(camera, canvas, Some(scene), None)

      let hemi =
        b.create_hemispheric_light_with(Some(#(0.0, 1.0, 0.2)), Some(0.72))
      hemispheric_light.set_diffuse_color(hemi, #(0.85, 0.88, 0.95))
      hemispheric_light.set_ground_color(hemi, #(0.14, 0.14, 0.16))
      b.add_to_scene(scene, bt.scene_entity_light_base(hemi))

      let dir = b.create_directional_light_with(#(-1.0, -2.0, -1.2), Some(2.1))
      directional_light.set_diffuse(dir, #(1.0, 0.97, 0.92))
      b.add_to_scene(scene, bt.scene_entity_light_base(dir))

      let ground_opts =
        ground_options.create()
        |> ground_options.with_width(20.0)
        |> ground_options.with_height(20.0)
      let ground = b.create_ground_with(engine, Some(ground_opts))
      mesh.set_material(ground, b.create_grid_material())
      b.add_to_scene(scene, bt.scene_entity_mesh(ground))

      let ramp_opts =
        box_options.create()
        |> box_options.with_width(3.5)
        |> box_options.with_height(0.2)
        |> box_options.with_depth(1.3)
      let ramp_mesh =
        b.create_box_with(
          engine,
          Some(bt.NumberOrBoxOptionsBoxOptions(ramp_opts)),
        )
      observable_vec3.set(
        scene_node.get_position(ramp_mesh),
        ramp_x,
        ramp_y,
        ramp_z,
      )
      observable_quat.set(
        scene_node.get_rotation_quaternion(ramp_mesh),
        0.0,
        0.0,
        sin(ramp_angle *. 0.5),
        cos(ramp_angle *. 0.5),
      )
      mesh.set_material(ramp_mesh, mk_pbr(0.28, 0.3, 0.34, 0.35, 0.48))
      b.add_to_scene(scene, bt.scene_entity_mesh(ramp_mesh))

      let sopts =
        sphere_options.create()
        |> sphere_options.with_diameter(marble_radius *. 2.0)
        |> sphere_options.with_segments(24.0)
      let mb = b.create_sphere_with(engine, Some(sopts))
      mesh.set_material(mb, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.2))
      b.add_to_scene(scene, bt.scene_entity_mesh(mb))
      rp_rt.ref_set(
        marble_handle_ref,
        Some(#(
          scene_node.get_position(mb),
          scene_node.get_rotation_quaternion(mb),
        )),
      )

      let mat_a = mk_pbr(0.88, 0.88, 0.88, 0.15, 0.4)
      let mat_b = mk_pbr(0.45, 0.46, 0.48, 0.25, 0.38)
      let dom_opts =
        box_options.create()
        |> box_options.with_width(dom_hx *. 2.0)
        |> box_options.with_height(dom_hy *. 2.0)
        |> box_options.with_depth(dom_hz *. 2.0)
      let dom_handles =
        int.range(from: 0, to: dom_count, with: [], run: fn(acc, i) {
          let m =
            b.create_box_with(
              engine,
              Some(bt.NumberOrBoxOptionsBoxOptions(dom_opts)),
            )
          mesh.set_material(m, case i % 2 == 0 {
            True -> mat_a
            False -> mat_b
          })
          b.add_to_scene(scene, bt.scene_entity_mesh(m))
          [
            #(scene_node.get_position(m), scene_node.get_rotation_quaternion(m)),
            ..acc
          ]
        })
        |> list.reverse
      rp_rt.ref_set(dom_handles_ref, dom_handles)

      let cyl_mat = mk_pbr(0.26, 0.62, 0.88, 0.45, 0.25)
      let cyl_opts =
        cylinder_options.create()
        |> cylinder_options.with_height(0.84)
        |> cylinder_options.with_diameter(0.5)
        |> cylinder_options.with_tessellation(24.0)
      let cyl_handles =
        int.range(from: 0, to: cyl_count, with: [], run: fn(acc, _) {
          let m = b.create_cylinder_with(engine, Some(cyl_opts))
          mesh.set_material(m, cyl_mat)
          b.add_to_scene(scene, bt.scene_entity_mesh(m))
          [
            #(scene_node.get_position(m), scene_node.get_rotation_quaternion(m)),
            ..acc
          ]
        })
        |> list.reverse
      rp_rt.ref_set(cyl_handles_ref, cyl_handles)

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
