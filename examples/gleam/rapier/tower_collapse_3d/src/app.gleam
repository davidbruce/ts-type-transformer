import babylonjs_lite as b
import babylonjs_lite/box_options
import babylonjs_lite/create_pbr_material_props
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
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const per_layer = 3

const plank_count = 18

const half_long = 0.85

const half_height = 0.22

const half_thin = 0.26

const sphere_radius = 0.65

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
  let planks_ref: rp_rt.Ref(List(rt.RigidBody(Nil))) = rp_rt.ref_new([])
  let striker_ref: rp_rt.Ref(Option(rt.RigidBody(Nil))) = rp_rt.ref_new(None)
  let plank_handles_ref: rp_rt.Ref(
    List(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil))),
  ) = rp_rt.ref_new([])
  let striker_handle_ref: rp_rt.Ref(
    Option(#(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil))),
  ) = rp_rt.ref_new(None)
  let paused_ref = rp_rt.ref_new(False)
  let step_ref = rp_rt.ref_new(0)
  let shot_ref = rp_rt.ref_new(0)

  let build_world = fn() {
    case rp_rt.ref_get(world_ref) {
      Some(old_w) -> world.free(old_w)
      None -> Nil
    }
    let w =
      world.new(
        vector3.new(0.0, -9.81, 0.0),
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
        None,
      )

    let floor_body =
      world.create_rigid_body(
        w,
        rigid_body_desc.fixed()
          |> rigid_body_desc.set_translation(0.0, -0.3, 0.0),
      )
    world.create_collider(
      w,
      collider_desc.cuboid(10.0, 0.3, 10.0)
        |> collider_desc.set_friction(0.75)
        |> collider_desc.set_restitution(0.1),
      Some(floor_body),
    )

    let planks =
      int.range(from: 0, to: plank_count, with: [], run: fn(acc, idx) {
        let layer = idx / per_layer
        let slot = idx % per_layer
        let offset = { int.to_float(slot) -. 1.0 } *. 0.57
        let y = half_height +. int.to_float(layer) *. 0.445
        let #(x, z, hx, hz) = case layer % 2 == 0 {
          True -> #(0.0, offset, half_long, half_thin)
          False -> #(offset, 0.0, half_thin, half_long)
        }
        let rb =
          world.create_rigid_body(
            w,
            rigid_body_desc.dynamic()
              |> rigid_body_desc.set_translation(x, y, z),
          )
        let cd =
          collider_desc.cuboid(hx, half_height, hz)
          |> collider_desc.set_friction(0.65)
          |> collider_desc.set_restitution(0.12)
          |> collider_desc.set_density(1.0)
        world.create_collider(w, cd, Some(rb))
        [rb, ..acc]
      })
      |> list.reverse

    let striker =
      world.create_rigid_body(
        w,
        rigid_body_desc.dynamic()
          |> rigid_body_desc.set_translation(-6.2, 1.55, 0.12)
          |> rigid_body_desc.set_linvel(13.5, 1.1, -0.25),
      )
    world.create_collider(
      w,
      collider_desc.ball(sphere_radius)
        |> collider_desc.set_density(4.5)
        |> collider_desc.set_restitution(0.35)
        |> collider_desc.set_friction(0.3),
      Some(striker),
    )

    rp_rt.ref_set(planks_ref, planks)
    rp_rt.ref_set(striker_ref, Some(striker))
    rp_rt.ref_set(world_ref, Some(w))
    rp_rt.ref_set(step_ref, 0)
  }

  let sync_pair = fn(
    rb: rt.RigidBody(Nil),
    pair: #(bt.ObservableVec3(Nil), bt.ObservableQuat(Nil)),
  ) {
    let #(pos, rot) = pair
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
  }

  let sync_meshes = fn() {
    list.each(
      list.zip(rp_rt.ref_get(planks_ref), rp_rt.ref_get(plank_handles_ref)),
      fn(item) { sync_pair(item.0, item.1) },
    )
    case rp_rt.ref_get(striker_ref), rp_rt.ref_get(striker_handle_ref) {
      Some(rb), Some(pair) -> sync_pair(rb, pair)
      _, _ -> Nil
    }
  }

  let update_meta = fn() {
    let #(max_y, plank_ke) =
      list.fold(rp_rt.ref_get(planks_ref), #(0.0, 0.0), fn(acc, rb) {
        let #(cur_max, cur_ke) = acc
        let p = rigid_body.translation(rb, None)
        let v = rigid_body.linvel(rb, None)
        let vx = vector.get_x(v)
        let vy = vector.get_y(v)
        let vz = vector.get_z(v)
        let next_max = float.max(cur_max, vector.get_y(p))
        let next_ke = cur_ke +. 0.5 *. { vx *. vx +. vy *. vy +. vz *. vz }
        #(next_max, next_ke)
      })
    let total_ke = case rp_rt.ref_get(striker_ref) {
      Some(rb) -> {
        let v = rigid_body.linvel(rb, None)
        let vx = vector.get_x(v)
        let vy = vector.get_y(v)
        let vz = vector.get_z(v)
        plank_ke +. 2.25 *. { vx *. vx +. vy *. vy +. vz *. vz }
      }
      None -> plank_ke
    }
    let state_text = case rp_rt.ref_get(paused_ref) {
      True -> "paused"
      False -> "running"
    }
    dom_node.set_text_content(
      meta_el,
      Some(
        "19 bodies (18 planks + sphere) · max_y="
        <> rp_rt.to_fixed(max_y, 2)
        <> " m · ke="
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
      Some(w) -> world.step(w, None, None)
      None -> Nil
    }
    rp_rt.ref_update(step_ref, fn(s) { s + 1 })
    sync_meshes()
    update_meta()
  }

  build_world()
  int.range(from: 0, to: 18, with: Nil, run: fn(_, _) {
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

  wire_click("btn-launch", fn() {
    let s = rp_rt.ref_update(shot_ref, fn(cur) { cur + 1 })
    let angle = int.to_float(s) *. 0.9
    let dist = 6.2
    let sx = 0.0 -. dist *. cos(angle)
    let sz = dist *. sin(angle)
    let sy = 1.2 +. int.to_float(s % 3) *. 0.45
    case rp_rt.ref_get(striker_ref) {
      Some(rb) -> {
        rigid_body.set_translation(rb, vector3.new(sx, sy, sz), True)
        rigid_body.set_linvel(
          rb,
          vector3.new(14.4 *. cos(angle), 0.9, -14.4 *. sin(angle)),
          True,
        )
        rigid_body.set_angvel(rb, vector3.new(0.0, 0.0, 0.0), True)
      }
      None -> Nil
    }
    sync_meshes()
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
    int.range(from: 0, to: 12, with: Nil, run: fn(_, _) {
      case rp_rt.ref_get(world_ref) {
        Some(w) -> world.step(w, None, None)
        None -> Nil
      }
      rp_rt.ref_update(step_ref, fn(s) { s + 1 })
      Nil
    })
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
        b.create_engine(bt.RenderCanvasHTMLCanvasElement(canvas), None),
        on_err,
      )

      let scene = b.create_scene_context(engine, None)
      let clear =
        gpu_color_dict.create()
        |> gpu_color_dict.with_r(0.06)
        |> gpu_color_dict.with_g(0.07)
        |> gpu_color_dict.with_b(0.09)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(0.85, 1.12, 11.5, b.vec3(0.0, 1.5, 0.0))
      b.add_to_scene(scene, bt.scene_entity_camera(camera))
      b.attach_control(camera, canvas, Some(scene), None)

      let hemi = b.create_hemispheric_light(Some(#(0.0, 1.0, 0.2)), Some(0.72))
      hemispheric_light.set_diffuse_color(hemi, #(0.85, 0.88, 0.95))
      hemispheric_light.set_ground_color(hemi, #(0.14, 0.14, 0.16))
      b.add_to_scene(scene, bt.scene_entity_light_base(hemi))

      let dir = b.create_directional_light(#(-1.0, -2.0, -1.2), Some(2.1))
      directional_light.set_diffuse(dir, #(1.0, 0.97, 0.92))
      b.add_to_scene(scene, bt.scene_entity_light_base(dir))

      let ground_opts =
        ground_options.create()
        |> ground_options.with_width(20.0)
        |> ground_options.with_height(20.0)
      let ground = b.create_ground(engine, Some(ground_opts))
      mesh.set_material(ground, b.create_grid_material(None))
      b.add_to_scene(scene, bt.scene_entity_mesh(ground))

      let mat0 = mk_pbr(0.86, 0.86, 0.86, 0.15, 0.45)
      let mat1 = mk_pbr(0.58, 0.58, 0.6, 0.25, 0.38)
      let mat2 = mk_pbr(0.3, 0.31, 0.33, 0.35, 0.35)

      let plank_handles =
        int.range(from: 0, to: plank_count, with: [], run: fn(acc, idx) {
          let layer = idx / per_layer
          let #(w_box, d_box) = case layer % 2 == 0 {
            True -> #(half_long *. 2.0, half_thin *. 2.0)
            False -> #(half_thin *. 2.0, half_long *. 2.0)
          }
          let bopts =
            box_options.create()
            |> box_options.with_width(w_box)
            |> box_options.with_height(half_height *. 2.0)
            |> box_options.with_depth(d_box)
          let m =
            b.create_box(engine, Some(bt.NumberOrBoxOptionsBoxOptions(bopts)))
          let mat = case layer % 3 {
            0 -> mat0
            1 -> mat1
            _ -> mat2
          }
          mesh.set_material(m, mat)
          b.add_to_scene(scene, bt.scene_entity_mesh(m))
          [
            #(scene_node.get_position(m), scene_node.get_rotation_quaternion(m)),
            ..acc
          ]
        })
        |> list.reverse
      rp_rt.ref_set(plank_handles_ref, plank_handles)

      let sopts =
        sphere_options.create()
        |> sphere_options.with_diameter(sphere_radius *. 2.0)
        |> sphere_options.with_segments(24.0)
      let sm = b.create_sphere(engine, Some(sopts))
      mesh.set_material(sm, mk_pbr(0.95, 0.42, 0.28, 0.55, 0.22))
      b.add_to_scene(scene, bt.scene_entity_mesh(sm))
      rp_rt.ref_set(
        striker_handle_ref,
        Some(#(
          scene_node.get_position(sm),
          scene_node.get_rotation_quaternion(sm),
        )),
      )

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
