import babylonjs_lite as b
import babylonjs_lite/attach_control_options
import babylonjs_lite/box_options
import babylonjs_lite/capsule_options
import babylonjs_lite/clear_coat_props
import babylonjs_lite/create_pbr_material_props
import babylonjs_lite/directional_light
import babylonjs_lite/euler_proxy
import babylonjs_lite/ground_options
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_vec3
import babylonjs_lite/point_light
import babylonjs_lite/runtime as br
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/types as bt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
import dom/areas/html/html_element_tag_name_map as tag
import dom/areas/webgpu/gpu_color_dict
import dom/types.{type HTMLElement}
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import gleam/result

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

fn set_status_error(status: Option(HTMLElement(Nil)), message: String) -> Nil {
  case status {
    Some(el) ->
      node.set_text_content(
        el,
        Some("WebGPU / Babylon Lite error: " <> message),
      )
    None -> Nil
  }
}

fn mk_pbr(
  v: Float,
  metallic: Float,
  roughness: Float,
) -> bt.PbrMaterialProps(Nil) {
  let props =
    create_pbr_material_props.create()
    |> create_pbr_material_props.with_base_color_factor(#(v, v, v, 1.0))
    |> create_pbr_material_props.with_metallic_factor(metallic)
    |> create_pbr_material_props.with_roughness_factor(roughness)
  b.create_pbr_material_with(props)
}

pub fn main() -> Nil {
  let doc = core.get_document()
  let status = document.get_element_by_id(doc, "status")
  case document.query_selector(doc, tag.canvas) {
    None -> set_status_error(status, "missing <canvas> element")
    Some(canvas) -> {
      let on_err = fn(err) {
        set_status_error(status, br.js_error_message(err))
      }
      use engine <- br.promise_try(
        b.create_engine_html_canvas_element(canvas),
        on_err,
      )

      let scene = b.create_scene_context(engine)
      let clear =
        gpu_color_dict.create()
        |> gpu_color_dict.with_r(0.05)
        |> gpu_color_dict.with_g(0.05)
        |> gpu_color_dict.with_b(0.05)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(
          0.7853981633974483,
          3.141592653589793 /. 3.1,
          12.8,
          b.vec3(0.0, 1.4, 0.0),
        )
      b.add_to_scene_camera(scene, camera)
      b.attach_control_with(
        camera,
        canvas,
        scene,
        attach_control_options.create(),
      )

      let hemi = b.create_hemispheric_light_with(#(0.0, 1.0, 0.15), 0.55)
      hemispheric_light.set_diffuse_color(hemi, #(0.76, 0.76, 0.76))
      hemispheric_light.set_ground_color(hemi, #(0.1, 0.1, 0.1))
      b.add_to_scene_light_base(scene, hemi)

      let dir = b.create_directional_light_with(#(-1.0, -2.2, -0.8), 1.35)
      directional_light.set_diffuse(dir, #(0.9, 0.9, 0.9))
      b.add_to_scene_light_base(scene, dir)

      let light1 = b.create_point_light_with(#(5.2, 4.2, 0.0), 10.0)
      point_light.set_diffuse(light1, #(1.0, 1.0, 1.0))
      point_light.set_range(light1, 16.0)
      b.add_to_scene_light_base(scene, light1)

      let light2 = b.create_point_light_with(#(-5.2, 3.4, 0.0), 7.5)
      point_light.set_diffuse(light2, #(0.78, 0.78, 0.78))
      point_light.set_range(light2, 16.0)
      b.add_to_scene_light_base(scene, light2)

      let floor_opts =
        ground_options.create()
        |> ground_options.with_width(18.0)
        |> ground_options.with_height(18.0)
      let floor = b.create_ground_with(engine, floor_opts)
      mesh.set_material(floor, mk_pbr(0.09, 0.1, 0.88))
      b.add_to_scene_mesh(scene, floor)

      let m0 = mk_pbr(0.22, 0.35, 0.36)
      let m1 = mk_pbr(0.44, 0.35, 0.36)
      let m2 = mk_pbr(0.68, 0.35, 0.36)
      let m3 = mk_pbr(0.88, 0.35, 0.36)

      let bopts =
        box_options.create()
        |> box_options.with_width(0.72)
        |> box_options.with_height(1.0)
        |> box_options.with_depth(0.72)

      let col_rows =
        list.map([0, 1, 2, 3, 4, 5], fn(row) {
          list.map([0, 1, 2, 3, 4, 5], fn(col) {
            let fx = int.to_float(col) -. 2.5
            let fz = int.to_float(row) -. 2.5
            let x = fx *. 1.15
            let z = fz *. 1.15
            let m = b.create_box_options_with(engine, bopts)
            let pos = scene_node.get_position(m)
            observable_vec3.set(pos, x, 0.5, z)
            let mat = case { row + col } % 4 {
              0 -> m0
              1 -> m1
              2 -> m2
              _ -> m3
            }
            mesh.set_material(m, mat)
            b.add_to_scene_mesh(scene, m)
            #(
              pos,
              scene_node.get_scaling(m),
              x,
              z,
              float.square_root(fx *. fx +. fz *. fz) |> result.unwrap(0.0),
              fx -. fz,
            )
          })
        })

      let cap_props =
        create_pbr_material_props.create()
        |> create_pbr_material_props.with_base_color_factor(#(
          0.92,
          0.92,
          0.92,
          1.0,
        ))
        |> create_pbr_material_props.with_metallic_factor(0.5)
        |> create_pbr_material_props.with_roughness_factor(0.15)
      let cc =
        clear_coat_props.create()
        |> clear_coat_props.with_is_enabled(True)
        |> clear_coat_props.with_intensity(1.0)
        |> clear_coat_props.with_roughness(0.08)
      b.set_pbr_clear_coat(cap_props, cc)
      b.set_pbr_emissive(cap_props, #(0.16, 0.16, 0.16))
      let cap_mat = b.create_pbr_material_with(cap_props)

      let copts =
        capsule_options.create()
        |> capsule_options.with_height(1.15)
        |> capsule_options.with_radius(0.22)
        |> capsule_options.with_tessellation(20.0)
      let capsules =
        list.map([0, 1, 2, 3], fn(k) {
          let cap = b.create_capsule_with(engine, copts)
          mesh.set_material(cap, cap_mat)
          b.add_to_scene_mesh(scene, cap)
          #(
            int.to_float(k),
            scene_node.get_position(cap),
            scene_node.get_rotation(cap),
          )
        })

      let l1_pos = point_light.get_position(light1)
      let l2_pos = point_light.get_position(light2)
      let perf = core.get_performance()
      let start_ms = performance.now(perf)

      b.on_before_render(scene, fn(_delta_ms) {
        let elapsed = { performance.now(perf) -. start_ms } *. 0.001
        list.each(col_rows, fn(row_items) {
          list.each(row_items, fn(item) {
            let #(pos, scale, x, z, dist, diag) = item
            let h =
              0.55
              +. 1.15
              *. { 0.5 +. 0.5 *. sin(elapsed *. 2.2 -. dist *. 1.15) }
              +. 0.3
              *. cos(elapsed *. 1.3 +. diag *. 0.7)
            observable_vec3.set(scale, 1.0, h, 1.0)
            observable_vec3.set(pos, x, h *. 0.5, z)
          })
        })
        list.each(capsules, fn(item) {
          let #(fk, pos, rot) = item
          let theta = elapsed *. 0.7 +. fk *. 1.5707963267948966
          observable_vec3.set(
            pos,
            cos(theta) *. 4.9,
            2.6 +. sin(elapsed *. 1.6 +. fk *. 1.57) *. 0.55,
            sin(theta) *. 4.9,
          )
          euler_proxy.set_z(rot, sin(elapsed +. fk) *. 0.35)
          euler_proxy.set_y(rot, 0.0 -. theta)
        })
        observable_vec3.set(
          l1_pos,
          cos(elapsed *. 0.65) *. 5.2,
          4.2,
          sin(elapsed *. 0.65) *. 5.2,
        )
        observable_vec3.set(
          l2_pos,
          cos(0.0 -. elapsed *. 0.52) *. 5.2,
          3.4,
          sin(0.0 -. elapsed *. 0.52) *. 5.2,
        )
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
