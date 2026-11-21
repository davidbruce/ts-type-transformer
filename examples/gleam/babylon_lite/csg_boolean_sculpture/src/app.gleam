import babylonjs_lite as b
import babylonjs_lite/box_options
import babylonjs_lite/clear_coat_props
import babylonjs_lite/create_pbr_material_props
import babylonjs_lite/cylinder_options
import babylonjs_lite/directional_light
import babylonjs_lite/disc_options
import babylonjs_lite/euler_proxy
import babylonjs_lite/ground_options
import babylonjs_lite/hemispheric_light
import babylonjs_lite/mesh
import babylonjs_lite/observable_vec3
import babylonjs_lite/point_light
import babylonjs_lite/scene_context
import babylonjs_lite/scene_node
import babylonjs_lite/sphere_options
import babylonjs_lite/torus_options
import babylonjs_lite/runtime as br
import babylonjs_lite/types as bt
import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node
import dom/areas/core/performance
import dom/areas/webgpu/gpu_color_dict
import dom/types.{type HTMLElement}
import dom/areas/html/html_element_tag_name_map as tag
import gleam/option.{type Option, None, Some}

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

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
  let props =
    create_pbr_material_props.create()
    |> create_pbr_material_props.with_base_color_factor(#(v, v, v, 1.0))
    |> create_pbr_material_props.with_metallic_factor(metallic)
    |> create_pbr_material_props.with_roughness_factor(roughness)
  b.create_pbr_material(Some(props))
}

pub fn main() -> Nil {
  let doc = core.get_document()
  let status = document.get_element_by_id(doc, "status")
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
        |> gpu_color_dict.with_r(0.06)
        |> gpu_color_dict.with_g(0.06)
        |> gpu_color_dict.with_b(0.06)
        |> gpu_color_dict.with_a(1.0)
      scene_context.set_clear_color(scene, clear)

      let camera =
        b.create_arc_rotate_camera(
          3.141592653589793 /. 4.2,
          3.141592653589793 /. 2.9,
          8.8,
          b.vec3(0.0, 2.1, 0.0),
        )
      b.add_to_scene(scene, br.dynamic(camera))
      let _ = b.attach_control(camera, canvas, Some(scene), None)

      let hemi =
        b.create_hemispheric_light(
          Some(#(0.0, 1.0, 0.2)),
          Some(0.72),
        )
      hemispheric_light.set_diffuse_color(hemi, #(0.85, 0.85, 0.85))
      hemispheric_light.set_ground_color(hemi, #(0.14, 0.14, 0.14))
      b.add_to_scene(scene, br.dynamic(hemi))

      let dir = b.create_directional_light(#(-1.2, -2.0, -1.1), Some(2.1))
      directional_light.set_diffuse(dir, #(1.0, 1.0, 1.0))
      b.add_to_scene(scene, br.dynamic(dir))

      let key_light = b.create_point_light(#(3.2, 3.6, 2.4), Some(8.0))
      point_light.set_diffuse(key_light, #(0.92, 0.92, 0.92))
      point_light.set_range(key_light, 14.0)
      b.add_to_scene(scene, br.dynamic(key_light))

      let floor_opts =
        ground_options.create()
        |> ground_options.with_width(16.0)
        |> ground_options.with_height(16.0)
      let floor = b.create_ground(engine, Some(floor_opts))
      mesh.set_material(floor, mk_pbr(0.09, 0.10, 0.88))
      b.add_to_scene(scene, br.dynamic(floor))

      let dopts =
        disc_options.create()
        |> disc_options.with_radius(2.6)
        |> disc_options.with_tessellation(48.0)
      let disc = b.create_disc(engine, Some(dopts))
      euler_proxy.set_x(
        scene_node.get_rotation(disc),
        1.5707963267948966,
      )
      observable_vec3.set(scene_node.get_position(disc), 0.0, 0.03, 0.0)
      mesh.set_material(disc, mk_pbr(0.22, 0.25, 0.65))
      b.add_to_scene(scene, br.dynamic(disc))

      let sbopts =
        box_options.create()
        |> box_options.with_size(2.1)
      let src_box =
        b.create_box(
          engine,
          Some(bt.NumberOrBoxOptionsBoxOptions(sbopts)),
        )
      let copts =
        cylinder_options.create()
        |> cylinder_options.with_height(3.0)
        |> cylinder_options.with_diameter(1.18)
        |> cylinder_options.with_tessellation(16.0)
      let cyl_y = b.create_cylinder(engine, Some(copts))
      let cyl_x = b.create_cylinder(engine, Some(copts))
      euler_proxy.set_z(
        scene_node.get_rotation(cyl_x),
        1.5707963267948966,
      )
      let cyl_z = b.create_cylinder(engine, Some(copts))
      euler_proxy.set_x(
        scene_node.get_rotation(cyl_z),
        1.5707963267948966,
      )

      let s1 =
        b.csg_subtract(
          b.create_csg_from_mesh(src_box, None),
          b.create_csg_from_mesh(cyl_y, None),
        )
      let s2 = b.csg_subtract(s1, b.create_csg_from_mesh(cyl_x, None))
      let s3 = b.csg_subtract(s2, b.create_csg_from_mesh(cyl_z, None))
      let monolith = b.create_mesh_from_csg(engine, s3, Some("monolith"))
      observable_vec3.set(
        scene_node.get_position(monolith),
        0.0,
        2.15,
        0.0,
      )

      let mprops =
        create_pbr_material_props.create()
        |> create_pbr_material_props.with_base_color_factor(#(0.86, 0.86, 0.86, 1.0))
        |> create_pbr_material_props.with_metallic_factor(0.65)
        |> create_pbr_material_props.with_roughness_factor(0.24)
      let cc =
        clear_coat_props.create()
        |> clear_coat_props.with_is_enabled(True)
        |> clear_coat_props.with_intensity(0.90)
        |> clear_coat_props.with_roughness(0.10)
      b.set_pbr_clear_coat(mprops, cc)
      mesh.set_material(monolith, b.create_pbr_material(Some(mprops)))
      b.add_to_scene(scene, br.dynamic(monolith))

      let cbopts =
        box_options.create()
        |> box_options.with_size(0.82)
      let core_box =
        b.create_box(
          engine,
          Some(bt.NumberOrBoxOptionsBoxOptions(cbopts)),
        )
      let csopts =
        sphere_options.create()
        |> sphere_options.with_diameter(1.02)
        |> sphere_options.with_segments(14.0)
      let core_sphere = b.create_sphere(engine, Some(csopts))
      let core_solid =
        b.csg_intersect(
          b.create_csg_from_mesh(core_box, None),
          b.create_csg_from_mesh(core_sphere, None),
        )
      let core = b.create_mesh_from_csg(engine, core_solid, Some("core"))
      observable_vec3.set(scene_node.get_position(core), 0.0, 2.15, 0.0)
      mesh.set_material(core, mk_pbr(0.18, 0.85, 0.16))
      b.add_to_scene(scene, br.dynamic(core))

      let otopts =
        torus_options.create()
        |> torus_options.with_diameter(4.2)
        |> torus_options.with_thickness(0.09)
        |> torus_options.with_tessellation(48.0)
      let outer_ring = b.create_torus(engine, Some(otopts))
      observable_vec3.set(
        scene_node.get_position(outer_ring),
        0.0,
        2.15,
        0.0,
      )
      mesh.set_material(outer_ring, mk_pbr(0.55, 0.80, 0.25))
      b.add_to_scene(scene, br.dynamic(outer_ring))

      let itopts =
        torus_options.create()
        |> torus_options.with_diameter(3.55)
        |> torus_options.with_thickness(0.07)
        |> torus_options.with_tessellation(48.0)
      let inner_ring = b.create_torus(engine, Some(itopts))
      observable_vec3.set(
        scene_node.get_position(inner_ring),
        0.0,
        2.15,
        0.0,
      )
      mesh.set_material(inner_ring, mk_pbr(0.88, 0.40, 0.30))
      b.add_to_scene(scene, br.dynamic(inner_ring))

      let mono_rot = scene_node.get_rotation(monolith)
      let core_rot = scene_node.get_rotation(core)
      let outer_rot = scene_node.get_rotation(outer_ring)
      let inner_rot = scene_node.get_rotation(inner_ring)
      let perf = core.get_performance()
      let start_ms = performance.now(perf)

      b.on_before_render(scene, fn(_delta_ms) {
        let elapsed = { performance.now(perf) -. start_ms } *. 0.001
        euler_proxy.set_y(mono_rot, elapsed *. 0.35)
        euler_proxy.set_x(mono_rot, sin(elapsed *. 0.3) *. 0.20)
        euler_proxy.set_y(core_rot, 0.0 -. elapsed *. 0.90)
        euler_proxy.set_z(core_rot, elapsed *. 0.50)
        euler_proxy.set_x(outer_rot, elapsed *. 0.42)
        euler_proxy.set_y(outer_rot, elapsed *. 0.25)
        euler_proxy.set_z(inner_rot, 0.0 -. elapsed *. 0.58)
        euler_proxy.set_x(inner_rot, elapsed *. 0.31)
      })

      use _ <- br.promise_try(b.register_scene(scene), on_err)
      use _ <- br.promise_try(b.start_engine(engine), on_err)
      Nil
    }
  }
}
