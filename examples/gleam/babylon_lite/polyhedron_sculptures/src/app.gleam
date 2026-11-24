import babylonjs_lite as b
import babylonjs_lite/cylinder_options
import babylonjs_lite/directional_light
import babylonjs_lite/euler_proxy
import babylonjs_lite/ground_options
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_vec3
import babylonjs_lite/polyhedron_options
import babylonjs_lite/runtime as br
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/spot_light
import babylonjs_lite/standard_material_props
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

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

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

fn mk_std(
  diff: Float,
  spec: Float,
  power: Float,
) -> bt.StandardMaterialProps(Nil) {
  let mat = b.create_standard_material()
  standard_material_props.set_diffuse_color(mat, #(diff, diff, diff))
  standard_material_props.set_specular_color(mat, #(spec, spec, spec))
  standard_material_props.set_specular_power(mat, power)
  mat
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
        b.create_engine(bt.RenderCanvasHTMLCanvasElement(canvas), None),
        on_err,
      )

      let scene = b.create_scene_context(engine, None)
      let clear =
        gpu_color_dict.create()
        |> gpu_color_dict.with_r(0.05)
        |> gpu_color_dict.with_g(0.05)
        |> gpu_color_dict.with_b(0.05)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(
          1.5707963267948966,
          3.141592653589793 /. 2.8,
          11.5,
          b.vec3(0.0, 1.6, 0.0),
        )
      b.add_to_scene(scene, bt.scene_entity_camera(camera))
      b.attach_control(camera, canvas, Some(scene), None)

      let hemi = b.create_hemispheric_light(Some(#(0.0, 1.0, 0.0)), Some(0.42))
      hemispheric_light.set_diffuse_color(hemi, #(0.7, 0.7, 0.7))
      hemispheric_light.set_ground_color(hemi, #(0.08, 0.08, 0.08))
      b.add_to_scene(scene, bt.scene_entity_light_base(hemi))

      let spot =
        b.create_spot_light(
          #(0.0, 6.2, 3.0),
          #(0.0, -1.0, -0.55),
          3.141592653589793 /. 3.2,
          3.0,
          Some(26.0),
        )
      spot_light.set_diffuse(spot, #(1.0, 1.0, 1.0))
      spot_light.set_range(spot, 22.0)
      b.add_to_scene(scene, bt.scene_entity_light_base(spot))

      let fill = b.create_directional_light(#(1.2, -1.5, -1.0), Some(0.65))
      directional_light.set_diffuse(fill, #(0.8, 0.8, 0.8))
      b.add_to_scene(scene, bt.scene_entity_light_base(fill))

      let floor_opts =
        ground_options.create()
        |> ground_options.with_width(18.0)
        |> ground_options.with_height(18.0)
      let floor = b.create_ground(engine, Some(floor_opts))
      mesh.set_material(floor, mk_std(0.1, 0.25, 24.0))
      b.add_to_scene(scene, bt.scene_entity_mesh(floor))

      let plinth_mat = mk_std(0.18, 0.3, 32.0)

      let poly_states =
        list.map([0, 1, 2, 3, 4], fn(i) {
          let fi = int.to_float(i)
          let x = { fi -. 2.0 } *. 2.25
          let z = 0.0 -. float.absolute_value(fi -. 2.0) *. 0.55

          let copts =
            cylinder_options.create()
            |> cylinder_options.with_height(1.1)
            |> cylinder_options.with_diameter(1.15)
            |> cylinder_options.with_tessellation(28.0)
          let plinth = b.create_cylinder(engine, Some(copts))
          observable_vec3.set(scene_node.get_position(plinth), x, 0.55, z)
          mesh.set_material(plinth, plinth_mat)
          b.add_to_scene(scene, bt.scene_entity_mesh(plinth))

          let v = 0.9 -. fi *. 0.13
          let popts =
            polyhedron_options.create()
            |> polyhedron_options.with_type(fi)
            |> polyhedron_options.with_size(0.62)
            |> polyhedron_options.with_flat(True)
          let poly = b.create_polyhedron(engine, Some(popts))
          let pos = scene_node.get_position(poly)
          observable_vec3.set(pos, x, 2.05, z)
          mesh.set_material(poly, mk_std(v, 0.95, 18.0 +. fi *. 24.0))
          b.add_to_scene(scene, bt.scene_entity_mesh(poly))
          #(fi, pos, scene_node.get_rotation(poly), x, z)
        })

      let spot_pos = spot_light.get_position(spot)
      let spot_dir = spot_light.get_direction(spot)
      let perf = core.get_performance()
      let start_ms = performance.now(perf)

      b.on_before_render(scene, fn(_delta_ms) {
        let elapsed = { performance.now(perf) -. start_ms } *. 0.001
        let sx = sin(elapsed *. 0.65) *. 5.2
        observable_vec3.set(spot_pos, sx, 6.2, 3.0)
        observable_vec3.set(spot_dir, 0.0 -. sx *. 0.15, -1.0, -0.55)
        list.each(poly_states, fn(st) {
          let #(fi, pos, rot, x, z) = st
          observable_vec3.set(
            pos,
            x,
            2.05 +. sin(elapsed *. 1.3 +. fi *. 1.1) *. 0.18,
            z,
          )
          euler_proxy.set_y(rot, elapsed *. { 0.45 +. 0.12 *. fi })
          euler_proxy.set_x(rot, sin(elapsed *. 0.7 +. fi) *. 0.35)
        })
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
