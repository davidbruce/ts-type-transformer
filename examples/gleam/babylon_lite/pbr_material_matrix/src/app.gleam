import babylonjs_lite as b
import babylonjs_lite/attach_control_options
import babylonjs_lite/box_options
import babylonjs_lite/create_pbr_material_props
import babylonjs_lite/directional_light
import babylonjs_lite/ground_options
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_vec3
import babylonjs_lite/point_light
import babylonjs_lite/runtime as br
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/sphere_options
import babylonjs_lite/types as bt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
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
        |> gpu_color_dict.with_r(0.06)
        |> gpu_color_dict.with_g(0.06)
        |> gpu_color_dict.with_b(0.06)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(
          0.7853981633974483,
          3.141592653589793 /. 3.1,
          12.0,
          b.vec3(0.0, 1.1, 0.0),
        )
      b.add_to_scene_camera(scene, camera)
      b.attach_control_with(
        camera,
        canvas,
        scene,
        attach_control_options.create(),
      )

      let hemi = b.create_hemispheric_light_with(#(0.0, 1.0, 0.2), 0.65)
      hemispheric_light.set_diffuse_color(hemi, #(0.82, 0.82, 0.82))
      hemispheric_light.set_ground_color(hemi, #(0.12, 0.12, 0.12))
      b.add_to_scene_light_base(scene, hemi)

      let dir = b.create_directional_light_with(#(-1.0, -2.0, -1.0), 1.8)
      directional_light.set_diffuse(dir, #(0.95, 0.95, 0.95))
      b.add_to_scene_light_base(scene, dir)

      let orb_light = b.create_point_light_with(#(4.5, 3.0, 0.0), 12.0)
      point_light.set_diffuse(orb_light, #(1.0, 1.0, 1.0))
      point_light.set_range(orb_light, 18.0)
      b.add_to_scene_light_base(scene, orb_light)

      let floor_opts =
        ground_options.create()
        |> ground_options.with_width(14.0)
        |> ground_options.with_height(14.0)
      let floor = b.create_ground_with(engine, floor_opts)
      mesh.set_material(floor, mk_pbr(0.11, 0.08, 0.88))
      b.add_to_scene_mesh(scene, floor)

      let plinth_opts =
        box_options.create()
        |> box_options.with_width(8.4)
        |> box_options.with_height(0.08)
        |> box_options.with_depth(8.4)
      let plinth = b.create_box_box_options_with(engine, plinth_opts)
      observable_vec3.set(scene_node.get_position(plinth), 0.0, 0.04, 0.0)
      mesh.set_material(plinth, mk_pbr(0.17, 0.12, 0.78))
      b.add_to_scene_mesh(scene, plinth)

      list.each([0, 1, 2, 3, 4], fn(row) {
        list.each([0, 1, 2, 3, 4], fn(col) {
          let roughness = 0.08 +. 0.22 *. int.to_float(row)
          let metallic = 0.25 *. int.to_float(col)
          let sopts =
            sphere_options.create()
            |> sphere_options.with_diameter(1.05)
            |> sphere_options.with_segments(24.0)
          let sphere = b.create_sphere_with(engine, sopts)
          observable_vec3.set(
            scene_node.get_position(sphere),
            int.to_float(col - 2) *. 1.55,
            0.75,
            int.to_float(row - 2) *. 1.55,
          )
          mesh.set_material(sphere, mk_pbr(0.85, metallic, roughness))
          b.add_to_scene_mesh(scene, sphere)
        })
      })

      let orb_props =
        create_pbr_material_props.create()
        |> create_pbr_material_props.with_base_color_factor(#(
          0.95,
          0.95,
          0.95,
          1.0,
        ))
        |> create_pbr_material_props.with_metallic_factor(0.0)
        |> create_pbr_material_props.with_roughness_factor(0.2)
      b.set_pbr_emissive(orb_props, #(0.95, 0.95, 0.95))
      let orb_mat = b.create_pbr_material_with(orb_props)

      let orb_opts =
        sphere_options.create()
        |> sphere_options.with_diameter(0.36)
        |> sphere_options.with_segments(16.0)
      let orb = b.create_sphere_with(engine, orb_opts)
      let orb_pos = scene_node.get_position(orb)
      observable_vec3.set(orb_pos, 4.5, 3.0, 0.0)
      mesh.set_material(orb, orb_mat)
      b.add_to_scene_mesh(scene, orb)

      let orb_light_pos = point_light.get_position(orb_light)
      let perf = core.get_performance()
      let start_ms = performance.now(perf)

      b.on_before_render(scene, fn(_delta_ms) {
        let elapsed = { performance.now(perf) -. start_ms } *. 0.001
        let x = cos(elapsed *. 0.85) *. 4.5
        let y = 3.0 +. sin(elapsed *. 1.4) *. 0.65
        let z = sin(elapsed *. 0.85) *. 4.5
        observable_vec3.set(orb_pos, x, y, z)
        observable_vec3.set(orb_light_pos, x, y, z)
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
