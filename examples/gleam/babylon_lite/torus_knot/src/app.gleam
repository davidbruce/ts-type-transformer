import babylonjs_lite as b
import babylonjs_lite/attach_control_options
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
import babylonjs_lite/sphere_options
import babylonjs_lite/torus_knot_options
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
import dom/areas/html/html_element_tag_name_map as tag
import dom/areas/webgpu/gpu_color_dict
import dom/types.{type HTMLElement}
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
        |> gpu_color_dict.with_g(0.07)
        |> gpu_color_dict.with_b(0.11)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(
          0.7853981633974483,
          1.0471975511965976,
          8.5,
          b.vec3(0.0, 1.6, 0.0),
        )
      b.add_to_scene_camera(scene, camera)
      b.attach_control_with(
        camera,
        canvas,
        scene,
        attach_control_options.create(),
      )

      let hemi = b.create_hemispheric_light_with(#(0.0, 1.0, 0.2), 0.75)
      hemispheric_light.set_diffuse_color(hemi, #(0.75, 0.85, 1.0))
      hemispheric_light.set_ground_color(hemi, #(0.12, 0.14, 0.22))
      b.add_to_scene_light_base(scene, hemi)

      let dir = b.create_directional_light_with(#(-1.0, -2.0, -1.2), 2.2)
      directional_light.set_diffuse(dir, #(1.0, 0.96, 0.9))
      b.add_to_scene_light_base(scene, dir)

      let orb_light = b.create_point_light_with(#(2.8, 1.6, 0.0), 8.0)
      point_light.set_diffuse(orb_light, #(1.0, 0.45, 0.35))
      point_light.set_range(orb_light, 12.0)
      b.add_to_scene_light_base(scene, orb_light)

      let knot_props =
        create_pbr_material_props.create()
        |> create_pbr_material_props.with_base_color_factor(#(
          0.18,
          0.78,
          0.95,
          1.0,
        ))
        |> create_pbr_material_props.with_metallic_factor(0.75)
        |> create_pbr_material_props.with_roughness_factor(0.22)
      let knot_mat = b.create_pbr_material_with(knot_props)

      let knot_opts =
        torus_knot_options.create()
        |> torus_knot_options.with_radius(1.15)
        |> torus_knot_options.with_tube(0.34)
        |> torus_knot_options.with_radial_segments(48.0)
        |> torus_knot_options.with_tubular_segments(160.0)
      let knot = b.create_torus_knot_with(engine, knot_opts)
      observable_vec3.set(scene_node.get_position(knot), 0.0, 2.0, 0.0)
      mesh.set_material(knot, knot_mat)
      b.add_to_scene_mesh(scene, knot)

      let orb_props =
        create_pbr_material_props.create()
        |> create_pbr_material_props.with_base_color_factor(#(
          1.0,
          0.42,
          0.32,
          1.0,
        ))
        |> create_pbr_material_props.with_metallic_factor(0.2)
        |> create_pbr_material_props.with_roughness_factor(0.35)
      let orb_mat = b.create_pbr_material_with(orb_props)

      let orb_opts =
        sphere_options.create()
        |> sphere_options.with_diameter(0.55)
        |> sphere_options.with_segments(32.0)
      let orb = b.create_sphere_with(engine, orb_opts)
      mesh.set_material(orb, orb_mat)
      b.add_to_scene_mesh(scene, orb)

      let ground_mat = b.create_grid_material()
      let ground_opts =
        ground_options.create()
        |> ground_options.with_width(16.0)
        |> ground_options.with_height(16.0)
      let ground = b.create_ground_with(engine, ground_opts)
      mesh.set_material(ground, ground_mat)
      b.add_to_scene_mesh(scene, ground)

      let knot_rot = scene_node.get_rotation(knot)
      let orb_pos = scene_node.get_position(orb)
      let orb_light_pos = point_light.get_position(orb_light)
      let perf = core.get_performance()
      let start_ms = performance.now(perf)

      b.on_before_render(scene, fn(_delta_ms) {
        let elapsed = { performance.now(perf) -. start_ms } *. 0.001
        euler_proxy.set_y(knot_rot, elapsed *. 0.55)
        euler_proxy.set_x(knot_rot, sin(elapsed *. 0.4) *. 0.25)
        let x = cos(elapsed *. 0.9) *. 2.8
        let y = 2.0 +. sin(elapsed *. 1.8) *. 0.45
        let z = sin(elapsed *. 0.9) *. 2.8
        observable_vec3.set(orb_pos, x, y, z)
        observable_vec3.set(orb_light_pos, x, y, z)
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
