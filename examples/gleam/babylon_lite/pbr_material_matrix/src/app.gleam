import babylonjs_lite/areas/main as b
import babylonjs_lite/areas/main/box_options
import babylonjs_lite/areas/main/create_pbr_material_props
import babylonjs_lite/areas/main/directional_light
import babylonjs_lite/areas/main/ground_options
import babylonjs_lite/areas/main/hemispheric_light
import babylonjs_lite/areas/main/mesh
import babylonjs_lite/areas/main/observable_vec3
import babylonjs_lite/areas/main/point_light
import babylonjs_lite/areas/main/scene_context
import babylonjs_lite/areas/main/scene_node
import babylonjs_lite/areas/main/sphere_options
import babylonjs_lite/runtime as br
import babylonjs_lite/types as bt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
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
      node.set_text_content(el, Some("WebGPU / Babylon Lite error: " <> message))
    None -> Nil
  }
}

fn mk_pbr(
  v: Float,
  metallic: Float,
  roughness: Float,
) -> bt.PbrMaterialProps(Nil) {
  let props = create_pbr_material_props.create()
  create_pbr_material_props.set_base_color_factor(props, Some(#(v, v, v, 1.0)))
  create_pbr_material_props.set_metallic_factor(props, Some(metallic))
  create_pbr_material_props.set_roughness_factor(props, Some(roughness))
  b.create_pbr_material(Some(props))
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
              gpu_color_dict.set_r(clear, 0.06)
              gpu_color_dict.set_g(clear, 0.06)
              gpu_color_dict.set_b(clear, 0.06)
              gpu_color_dict.set_a(clear, 1.0)
              scene_context.set_clear_color(scene, clear)

              let camera =
                b.create_arc_rotate_camera(
                  0.7853981633974483,
                  3.141592653589793 /. 3.1,
                  12.0,
                  b.vec3(0.0, 1.1, 0.0),
                )
              b.add_to_scene(scene, br.dynamic(camera))
              let _ = b.attach_control(camera, canvas, Some(scene), None)

              let hemi =
                b.create_hemispheric_light(
                  Some(#(0.0, 1.0, 0.2)),
                  Some(0.65),
                )
              hemispheric_light.set_diffuse_color(hemi, #(0.82, 0.82, 0.82))
              hemispheric_light.set_ground_color(hemi, #(0.12, 0.12, 0.12))
              b.add_to_scene(scene, br.dynamic(hemi))

              let dir =
                b.create_directional_light(#(-1.0, -2.0, -1.0), Some(1.8))
              directional_light.set_diffuse(dir, #(0.95, 0.95, 0.95))
              b.add_to_scene(scene, br.dynamic(dir))

              let orb_light =
                b.create_point_light(#(4.5, 3.0, 0.0), Some(12.0))
              point_light.set_diffuse(orb_light, #(1.0, 1.0, 1.0))
              point_light.set_range(orb_light, 18.0)
              b.add_to_scene(scene, br.dynamic(orb_light))

              let floor_opts = ground_options.create()
              ground_options.set_width(floor_opts, Some(14.0))
              ground_options.set_height(floor_opts, Some(14.0))
              let floor = b.create_ground(engine, Some(floor_opts))
              mesh.set_material(floor, mk_pbr(0.11, 0.08, 0.88))
              b.add_to_scene(scene, br.dynamic(floor))

              let plinth_opts = box_options.create()
              box_options.set_width(plinth_opts, Some(8.4))
              box_options.set_height(plinth_opts, Some(0.08))
              box_options.set_depth(plinth_opts, Some(8.4))
              let plinth =
                b.create_box(
                  engine,
                  Some(bt.NumberOrBoxOptionsBoxOptions(plinth_opts)),
                )
              observable_vec3.set(
                scene_node.get_position(plinth),
                0.0,
                0.04,
                0.0,
              )
              mesh.set_material(plinth, mk_pbr(0.17, 0.12, 0.78))
              b.add_to_scene(scene, br.dynamic(plinth))

              list.each([0, 1, 2, 3, 4], fn(row) {
                list.each([0, 1, 2, 3, 4], fn(col) {
                  let roughness = 0.08 +. 0.22 *. int.to_float(row)
                  let metallic = 0.25 *. int.to_float(col)
                  let sopts = sphere_options.create()
                  sphere_options.set_diameter(sopts, Some(1.05))
                  sphere_options.set_segments(sopts, Some(24.0))
                  let sphere = b.create_sphere(engine, Some(sopts))
                  observable_vec3.set(
                    scene_node.get_position(sphere),
                    int.to_float(col - 2) *. 1.55,
                    0.75,
                    int.to_float(row - 2) *. 1.55,
                  )
                  mesh.set_material(sphere, mk_pbr(0.85, metallic, roughness))
                  b.add_to_scene(scene, br.dynamic(sphere))
                })
              })

              let orb_props = create_pbr_material_props.create()
              create_pbr_material_props.set_base_color_factor(
                orb_props,
                Some(#(0.95, 0.95, 0.95, 1.0)),
              )
              create_pbr_material_props.set_metallic_factor(
                orb_props,
                Some(0.0),
              )
              create_pbr_material_props.set_roughness_factor(
                orb_props,
                Some(0.2),
              )
              b.set_pbr_emissive(orb_props, #(0.95, 0.95, 0.95))
              let orb_mat = b.create_pbr_material(Some(orb_props))

              let orb_opts = sphere_options.create()
              sphere_options.set_diameter(orb_opts, Some(0.36))
              sphere_options.set_segments(orb_opts, Some(16.0))
              let orb = b.create_sphere(engine, Some(orb_opts))
              let orb_pos = scene_node.get_position(orb)
              observable_vec3.set(orb_pos, 4.5, 3.0, 0.0)
              mesh.set_material(orb, orb_mat)
              b.add_to_scene(scene, br.dynamic(orb))

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
