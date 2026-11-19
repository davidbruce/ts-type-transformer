// @babylonjs/lite from Gleam, in Node: vectors, cameras, lights, materials,
// procedural mesh buffers, accessibility tree, and math tuples. Every check must hold.
import bindings/areas/main
import bindings/areas/main/accessibility_node
import bindings/areas/main/accessibility_node_options
import bindings/areas/main/accessibility_tag
import bindings/areas/main/accessibility_tree
import bindings/areas/main/arc_rotate_camera
import bindings/areas/main/create_pbr_material_props
import bindings/areas/main/directional_light
import bindings/areas/main/hemispheric_light
import bindings/areas/main/pbr_material_props
import bindings/areas/main/point_light
import bindings/areas/main/shader_material
import bindings/areas/main/sphere_mesh_data
import bindings/areas/main/sphere_options
import bindings/areas/main/torus_knot_data
import bindings/areas/main/torus_knot_options
import bindings/areas/main/vec3
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float) -> Bool {
  float.absolute_value(a -. b) <. 0.000001
}

pub fn main() {
  // Vectors.
  let target = main.vec3(1.0, 2.0, 3.0)
  let c1 = check(vec3.get_x(target) == 1.0 && vec3.get_y(target) == 2.0 && vec3.get_z(target) == 3.0, "vec3")
  vec3.set_y(target, 1.6)
  let c2 = check(near(vec3.get_y(target), 1.6), "vec3 mutation")

  // Camera.
  let camera = main.create_arc_rotate_camera(0.75, 1.0, 8.5, target)
  let c3 = check(near(arc_rotate_camera.get_alpha(camera), 0.75) && near(arc_rotate_camera.get_beta(camera), 1.0), "camera angles")
  let c4 = check(arc_rotate_camera.get_radius(camera) == 8.5 && near(vec3.get_y(arc_rotate_camera.get_target(camera)), 1.6), "camera target")

  // Lights.
  let hemi = main.create_hemispheric_light(Some(#(0.0, 1.0, 0.2)), Some(0.75))
  hemispheric_light.set_diffuse_color(hemi, #(0.75, 0.85, 1.0))
  hemispheric_light.set_ground_color(hemi, #(0.12, 0.14, 0.22))
  let #(dr, dg, db) = hemispheric_light.get_diffuse_color(hemi)
  let #(gr, _, _) = hemispheric_light.get_ground_color(hemi)
  let c5 = check(near(hemispheric_light.get_intensity(hemi), 0.75) && near(dr, 0.75) && near(dg, 0.85) && near(db, 1.0) && near(gr, 0.12), "hemispheric light")

  let dir = main.create_directional_light(#(-1.0, -2.0, -1.2), Some(2.2))
  directional_light.set_diffuse(dir, #(1.0, 0.96, 0.9))
  let #(_, d1, _) = directional_light.get_diffuse(dir)
  let c6 = check(near(directional_light.get_intensity(dir), 2.2) && near(d1, 0.96), "directional light")

  let pt = main.create_point_light(#(2.8, 1.6, 0.0), Some(8.0))
  point_light.set_range(pt, 12.0)
  let c7 = check(near(point_light.get_intensity(pt), 8.0) && near(point_light.get_range(pt), 12.0), "point light")

  // Materials.
  let props = create_pbr_material_props.create()
  create_pbr_material_props.set_base_color_factor(props, Some(#(0.18, 0.78, 0.95, 1.0)))
  create_pbr_material_props.set_metallic_factor(props, Some(0.75))
  create_pbr_material_props.set_roughness_factor(props, Some(0.22))
  let pbr = main.create_pbr_material(Some(props))
  let c8 = check(pbr_material_props.get_metallic_factor(pbr) == Some(0.75) && pbr_material_props.get_roughness_factor(pbr) == Some(0.22), "pbr material")

  let grid = main.create_grid_material(None)
  let c9 = check(!string.is_empty(shader_material.get_vertex_source(grid)) && !string.is_empty(shader_material.get_fragment_source(grid)), "grid shader")

  // Procedural geometry data.
  let sopts = sphere_options.create()
  sphere_options.set_diameter(sopts, Some(2.0))
  sphere_options.set_segments(sopts, Some(8.0))
  let sphere = main.create_sphere_data(Some(sopts))
  let c10 = check(sphere_mesh_data.get_vertex_count(sphere) >. 0.0 && sphere_mesh_data.get_index_count(sphere) >. 0.0, "sphere counts")

  let topts = torus_knot_options.create()
  torus_knot_options.set_radius(topts, Some(1.0))
  torus_knot_options.set_tube(topts, Some(0.3))
  let knot = main.create_torus_knot_data(Some(topts))
  let _ = torus_knot_data.get_positions(knot)
  let _ = torus_knot_data.get_indices(knot)

  // Accessibility tree.
  let tree = main.create_accessibility_tree()
  let tag = accessibility_tag.create()
  accessibility_tag.set_name(tag, Some("hero"))
  accessibility_tag.set_role(tag, Some("button"))
  let nopts = accessibility_node_options.create()
  accessibility_node_options.set_tag(nopts, Some(tag))
  let node = main.add_accessibility_node(tree, nopts)
  let c11 = check(list.length(accessibility_tree.get_roots(tree)) == 1 && !accessibility_tree.get_disposed(tree), "accessibility tree")
  let c12 = case accessibility_node.get_tag(node) {
    Some(got) -> check(accessibility_tag.get_name(got) == Some("hero") && accessibility_tag.get_role(got) == Some("button"), "accessibility tag")
    None -> check(False, "expected accessibility tag")
  }

  // Quaternion -> Euler tuple conversion.
  let #(ex, ey, ez) = main.quat_to_euler_xyz_tuple(0.0, 0.0, 0.0, 1.0)
  let c13 = check(near(ex, 0.0) && near(ey, 0.0) && near(ez, 0.0), "euler tuple")

  let total = c1 + c2 + c3 + c4 + c5 + c6 + c7 + c8 + c9 + c10 + c11 + c12 + c13
  io.println(int.to_string(total) <> " checks passed")
}
