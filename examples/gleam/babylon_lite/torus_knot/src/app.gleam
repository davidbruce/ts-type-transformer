import babylonjs_lite/areas/main as b
import babylonjs_lite/areas/main/create_pbr_material_props
import babylonjs_lite/areas/main/directional_light
import babylonjs_lite/areas/main/euler_proxy
import babylonjs_lite/areas/main/ground_options
import babylonjs_lite/areas/main/hemispheric_light
import babylonjs_lite/areas/main/mesh
import babylonjs_lite/areas/main/observable_vec3
import babylonjs_lite/areas/main/point_light
import babylonjs_lite/areas/main/scene_context
import babylonjs_lite/areas/main/scene_node
import babylonjs_lite/areas/main/sphere_options
import babylonjs_lite/areas/main/torus_knot_options
import babylonjs_lite/runtime as br
import babylonjs_lite/types as bt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
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
      node.set_text_content(el, Some("WebGPU / Babylon Lite error: " <> message))
    None -> Nil
  }
}

pub fn main() -> Nil {
  let doc = core.get_document()
  let status = document.get_element_by_id(doc, "status")
  case document.query_selector_canvas(doc) {
    None -> set_status_error(status, "missing <canvas> element")
    Some(canvas) -> {
      br.promise_then(
        b.create_engine_with_engine_options(
          bt.RenderCanvasHTMLCanvasElement(canvas),
          None,
        ),
        fn(engine_res) {
          case engine_res {
            Error(err) -> set_status_error(status, br.js_error_message(err))
            Ok(engine) -> {
              let scene = b.create_scene_context(engine, None)
              let clear = gpu_color_dict.create()
              gpu_color_dict.set_r(clear, 0.05)
              gpu_color_dict.set_g(clear, 0.07)
              gpu_color_dict.set_b(clear, 0.11)
              gpu_color_dict.set_a(clear, 1.0)
              scene_context.set_clear_color(scene, clear)

              let camera =
                b.create_arc_rotate_camera(
                  0.7853981633974483,
                  1.0471975511965976,
                  8.5,
                  b.vec3(0.0, 1.6, 0.0),
                )
              b.add_to_scene(scene, br.dynamic(camera))
              let _ = b.attach_control(camera, canvas, Some(scene), None)

              let hemi =
                b.create_hemispheric_light(
                  Some(#(0.0, 1.0, 0.2)),
                  Some(0.75),
                )
              hemispheric_light.set_diffuse_color(hemi, #(0.75, 0.85, 1.0))
              hemispheric_light.set_ground_color(hemi, #(0.12, 0.14, 0.22))
              b.add_to_scene(scene, br.dynamic(hemi))

              let dir =
                b.create_directional_light(#(-1.0, -2.0, -1.2), Some(2.2))
              directional_light.set_diffuse(dir, #(1.0, 0.96, 0.9))
              b.add_to_scene(scene, br.dynamic(dir))

              let orb_light = b.create_point_light(#(2.8, 1.6, 0.0), Some(8.0))
              point_light.set_diffuse(orb_light, #(1.0, 0.45, 0.35))
              point_light.set_range(orb_light, 12.0)
              b.add_to_scene(scene, br.dynamic(orb_light))

              let knot_props = create_pbr_material_props.create()
              create_pbr_material_props.set_base_color_factor(
                knot_props,
                Some(#(0.18, 0.78, 0.95, 1.0)),
              )
              create_pbr_material_props.set_metallic_factor(
                knot_props,
                Some(0.75),
              )
              create_pbr_material_props.set_roughness_factor(
                knot_props,
                Some(0.22),
              )
              let knot_mat = b.create_pbr_material(Some(knot_props))

              let knot_opts = torus_knot_options.create()
              torus_knot_options.set_radius(knot_opts, Some(1.15))
              torus_knot_options.set_tube(knot_opts, Some(0.34))
              torus_knot_options.set_radial_segments(knot_opts, Some(48.0))
              torus_knot_options.set_tubular_segments(knot_opts, Some(160.0))
              let knot = b.create_torus_knot(engine, Some(knot_opts))
              observable_vec3.set(scene_node.get_position(knot), 0.0, 2.0, 0.0)
              mesh.set_material(knot, knot_mat)
              b.add_to_scene(scene, br.dynamic(knot))

              let orb_props = create_pbr_material_props.create()
              create_pbr_material_props.set_base_color_factor(
                orb_props,
                Some(#(1.0, 0.42, 0.32, 1.0)),
              )
              create_pbr_material_props.set_metallic_factor(
                orb_props,
                Some(0.2),
              )
              create_pbr_material_props.set_roughness_factor(
                orb_props,
                Some(0.35),
              )
              let orb_mat = b.create_pbr_material(Some(orb_props))

              let orb_opts = sphere_options.create()
              sphere_options.set_diameter(orb_opts, Some(0.55))
              sphere_options.set_segments(orb_opts, Some(32.0))
              let orb = b.create_sphere(engine, Some(orb_opts))
              mesh.set_material(orb, orb_mat)
              b.add_to_scene(scene, br.dynamic(orb))

              let ground_mat = b.create_grid_material(None)
              let ground_opts = ground_options.create()
              ground_options.set_width(ground_opts, Some(16.0))
              ground_options.set_height(ground_opts, Some(16.0))
              let ground = b.create_ground(engine, Some(ground_opts))
              mesh.set_material(ground, ground_mat)
              b.add_to_scene(scene, br.dynamic(ground))

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

              br.promise_then(b.register_scene(scene), fn(reg_res) {
                case reg_res {
                  Error(err) -> set_status_error(status, br.js_error_message(err))
                  Ok(_) ->
                    br.promise_then(b.start_engine(engine), fn(start_res) {
                      case start_res {
                        Error(err) ->
                          set_status_error(status, br.js_error_message(err))
                        Ok(_) -> Nil
                      }
                    })
                }
              })
            }
          }
        },
      )
    }
  }
}
