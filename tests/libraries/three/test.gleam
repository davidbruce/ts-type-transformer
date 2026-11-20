// three.js from Gleam, in Node: vector, matrix, quaternion, euler and colour
// maths, boxes and spheres, a scene graph with world positions, a mesh, a
// ray cast, and geometry attributes. Every check must hold.
import bindings/areas/main/box3
import bindings/areas/main/box_geometry
import bindings/areas/main/box_geometry_parameters
import bindings/areas/main/buffer_attribute
import bindings/areas/main/buffer_geometry
import bindings/areas/main/color
import bindings/areas/main/euler
import bindings/areas/main/group
import bindings/areas/main/intersection
import bindings/areas/main/matrix4
import bindings/areas/main/mesh
import bindings/areas/main/mesh_basic_material
import bindings/areas/main/normal_buffer_attributes
import bindings/areas/main/object3d
import bindings/areas/main/quaternion
import bindings/areas/main/raycaster
import bindings/areas/main/sphere
import bindings/areas/main/vector3
import bindings/runtime
import bindings/types as t
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float) -> Bool {
  float.absolute_value(a -. b) <. 0.000001
}

fn v3(x: Float, y: Float, z: Float) {
  vector3.new(Some(x), Some(y), Some(z))
}

pub fn main() {
  let half_pi = 3.141592653589793 /. 2.0

  // ---- Vector3 ----
  // Vector3 structurally satisfies Vector3Like, so add/dot/cross/equals work directly.
  let v = v3(3.0, 4.0, 0.0)
  let w = vector3.clone(v)
  vector3.set_x(w, 10.0)
  let sum = vector3.add(vector3.clone(v3(1.0, 2.0, 3.0)), v3(4.0, 5.0, 6.0))
  let cross = vector3.cross(v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0))
  let passed =
    check(vector3.length(v) == 5.0, "length")
    + check(vector3.get_x(v) == 3.0 && vector3.get_y(v) == 4.0, "components")
    + check(vector3.get_x(w) == 10.0 && vector3.get_x(v) == 3.0, "clone is independent")
    + check(vector3.length_sq(v) == 25.0, "length_sq")
    + check(vector3.to_list(sum) == [5.0, 7.0, 9.0], "add Vector3 as Vector3Like")
    + check(vector3.dot(v3(1.0, 2.0, 3.0), v3(4.0, 5.0, 6.0)) == 32.0, "dot Vector3 as Vector3Like")
    + check(vector3.to_list(cross) == [0.0, 0.0, 1.0], "cross Vector3 as Vector3Like")
    + check(vector3.equals(v3(1.0, 2.0, 3.0), v3(1.0, 2.0, 3.0)), "equals Vector3 as Vector3Like")
  let _ = vector3.normalize(w)
  let _ = vector3.multiply_scalar(v, 2.0)
  let passed =
    passed
    + check(near(vector3.length(w), 1.0), "normalize")
    + check(vector3.to_list(v) == [6.0, 8.0, 0.0], "multiply_scalar, to_list")
    + check(near(vector3.angle_to(v3(1.0, 0.0, 0.0), v3(0.0, 1.0, 0.0)), half_pi), "angle_to")
  let projected = vector3.project_on_vector(v3(3.0, 4.0, 0.0), v3(1.0, 0.0, 0.0))
  let passed =
    passed
    + check(vector3.to_list(projected) == [3.0, 0.0, 0.0], "project_on_vector")
    + check(vector3.get_component(v3(7.0, 8.0, 9.0), 2.0) == 9.0, "get_component")

  // ---- Matrix4 ----
  let translate = matrix4.new()
  let _ = matrix4.make_translation(translate, v3(1.0, 2.0, 3.0))
  let moved = v3(1.0, 1.0, 1.0)
  let _ = vector3.apply_matrix4(moved, translate)
  let rot = matrix4.new()
  let _ = matrix4.make_rotation_z(rot, half_pi)
  let turned = v3(1.0, 0.0, 0.0)
  let _ = vector3.apply_matrix4(turned, rot)
  let scale = matrix4.new()
  let _ = matrix4.make_scale(scale, 2.0, 3.0, 4.0)
  let combined = matrix4.new()
  let _ = matrix4.multiply_matrices(combined, translate, scale)
  let elements = fn(m) {
    list.map(matrix4.get_elements(m), fn(e) {
      let assert Ok(f) = decode.run(e, decode.float)
      f
    })
  }
  let passed =
    passed
    + check(vector3.to_list(moved) == [2.0, 3.0, 4.0], "apply a translation")
    + check(near(vector3.get_x(turned), 0.0) && near(vector3.get_y(turned), 1.0), "apply a rotation about z")
    + check(near(matrix4.determinant(scale), 24.0), "determinant of a scale")
    + check(list.length(elements(combined)) == 16, "sixteen elements")
    + check(
      list.take(list.drop(elements(combined), 12), 3) == [1.0, 2.0, 3.0],
      "translation after scale is in the last column",
    )
    + check(list.take(elements(matrix4.new()), 1) == [1.0], "identity")
  let inverse = matrix4.clone(scale)
  let _ = matrix4.invert(inverse)
  let product = matrix4.new()
  let _ = matrix4.multiply_matrices(product, scale, inverse)
  let passed =
    passed
    + check(matrix4.equals(product, matrix4.new()), "m times its inverse is the identity")
  // compose and decompose
  let position = v3(5.0, 6.0, 7.0)
  let q = quaternion.new(None, None, None, None)
  let _ = quaternion.set_from_euler(q, euler.new(Some(0.0), Some(0.0), Some(half_pi), None), None)
  let composed = matrix4.new()
  let _ = matrix4.compose(composed, position, q, v3(2.0, 2.0, 2.0))
  let out_position = v3(0.0, 0.0, 0.0)
  let out_scale = v3(0.0, 0.0, 0.0)
  let out_q = quaternion.new(None, None, None, None)
  let _ = matrix4.decompose(composed, out_position, out_q, out_scale)
  let passed =
    passed
    + check(vector3.to_list(out_position) == [5.0, 6.0, 7.0], "decompose position")
    + check(near(vector3.get_x(out_scale), 2.0), "decompose scale")
    + check(near(quaternion.get_z(out_q), quaternion.get_z(q)), "decompose rotation")

  // ---- Quaternion and Euler ----
  let passed =
    passed
    + check(near(quaternion.get_z(q), 0.7071067811865476), "quaternion z of a quarter turn")
    + check(near(quaternion.get_w(q), 0.7071067811865476), "quaternion w")
    + check(near(quaternion.length(q), 1.0), "unit length")
  let q2 = quaternion.clone(q)
  let _ = quaternion.conjugate(q2)
  let both = quaternion.clone(q)
  let _ = quaternion.multiply(both, q2)
  let back = euler.new(None, None, None, None)
  let _ = euler.set_from_quaternion(back, q, None, None)
  let passed =
    passed
    + check(near(quaternion.get_w(both), 1.0), "q times its conjugate is the identity")
    + check(near(quaternion.angle_to(q, quaternion.new(None, None, None, None)), half_pi), "angle_to identity")
    + check(near(euler.get_z(back), half_pi) && near(euler.get_x(back), 0.0), "euler from a quaternion")
    + check(euler.get_order(back) == t.EulerOrderXYZ, "default order")
  let _ = euler.reorder(back, t.EulerOrderZYX)
  let passed = passed + check(euler.get_order(back) == t.EulerOrderZYX, "reorder")
  let halfway = quaternion.new(None, None, None, None)
  let _ = quaternion.slerp_quaternions(halfway, quaternion.new(None, None, None, None), q, 0.5)
  let passed =
    passed
    + check(near(quaternion.angle_to(halfway, quaternion.new(None, None, None, None)), half_pi /. 2.0), "slerp halfway")

  // ---- Color ----
  let red = color.new(Some(t.ColorRepresentationString("red")))
  let from_hex = color.new(Some(t.ColorRepresentationNumber(16_711_680.0)))
  let passed =
    passed
    + check(color.get_hex(red, None) == 16_711_680.0, "named colour's hex")
    + check(color.get_hex_string(from_hex, None) == "ff0000", "hex string")
    + check(color.equals(red, from_hex), "equal colours")
    + check(color.get_r(red) == 1.0 && color.get_g(red) == 0.0, "rgb components")
  let green = color.new_with_r_and_g_and_b(0.0, 1.0, 0.0)
  let mixed = color.clone(red)
  let _ = color.lerp(mixed, green, 0.5)
  let passed =
    passed
    + check(near(color.get_r(mixed), 0.5) && near(color.get_g(mixed), 0.5), "lerp")
    + check(color.get_hex_string(green, None) == "00ff00", "green")
  let styled = color.new(None)
  let _ = color.set_style(styled, "rgb(0, 0, 255)", None)
  let passed = passed + check(color.get_hex(styled, None) == 255.0, "set_style")

  // ---- Box3 and Sphere ----
  let box = box3.new(Some(v3(-1.0, -1.0, -1.0)), Some(v3(1.0, 1.0, 1.0)))
  let size = box3.get_size(box, v3(0.0, 0.0, 0.0))
  let other = box3.new(Some(v3(0.5, 0.5, 0.5)), Some(v3(3.0, 3.0, 3.0)))
  let far = box3.new(Some(v3(5.0, 5.0, 5.0)), Some(v3(6.0, 6.0, 6.0)))
  let passed =
    passed
    + check(vector3.to_list(size) == [2.0, 2.0, 2.0], "box size")
    + check(vector3.to_list(box3.get_center(box, v3(9.0, 9.0, 9.0))) == [0.0, 0.0, 0.0], "box centre")
    + check(box3.contains_point(box, v3(0.5, 0.0, -0.5)), "contains_point")
    + check(!box3.contains_point(box, v3(2.0, 0.0, 0.0)), "outside the box")
    + check(box3.intersects_box(box, other) && !box3.intersects_box(box, far), "intersects_box")
  let joined = box3.clone(box)
  let _ = box3.union(joined, far)
  let _ = box3.expand_by_scalar(box, 1.0)
  let passed =
    passed
    + check(vector3.to_list(box3.get_max(joined)) == [6.0, 6.0, 6.0], "union")
    + check(vector3.to_list(box3.get_max(box)) == [2.0, 2.0, 2.0], "expand_by_scalar")
    + check(!box3.is_empty(box), "not empty")
  let ball0 = sphere.new(Some(v3(0.0, 0.0, 0.0)), Some(2.0))
  let passed =
    passed
    + check(sphere.contains_point(ball0, v3(1.0, 1.0, 0.0)), "sphere contains")
    + check(sphere.distance_to_point(ball0, v3(5.0, 0.0, 0.0)) == 3.0, "distance to a point")
    + check(sphere.intersects_sphere(ball0, sphere.new(Some(v3(3.0, 0.0, 0.0)), Some(1.5))), "spheres meet")
    + check(sphere.intersects_box(ball0, far) == False, "sphere misses a far box")

  // ---- Scene graph ----
  let parent = group.new()
  vector3.set(object3d.get_position(parent), 1.0, 2.0, Some(3.0))
  object3d.set_name(parent, "parent")
  let child = group.new()
  vector3.set(object3d.get_position(child), 0.0, 0.0, Some(5.0))
  let _ = object3d.add(parent, [child])
  object3d.update_matrix_world(parent, Some(True))
  let world = object3d.get_world_position(child, v3(0.0, 0.0, 0.0))
  let passed =
    passed
    + check(vector3.to_list(world) == [1.0, 2.0, 8.0], "world position adds the parent's")
    + check(list.length(object3d.get_children(parent)) == 1, "children")
    + check(option.is_some(object3d.get_parent(child)), "parent set")
    + check(object3d.get_name(parent) == "parent", "name")
    + check(
      option.is_some(object3d.get_object_by_name(parent, "parent")),
      "get_object_by_name finds itself",
    )
  // rotate the parent a quarter turn about z: a child at (2, 0, 0) is at (0, 2, 0) from it
  let spinner = group.new()
  let arm = group.new()
  vector3.set(object3d.get_position(arm), 2.0, 0.0, Some(0.0))
  let _ = object3d.add(spinner, [arm])
  let _ = object3d.rotate_z(spinner, half_pi)
  object3d.update_matrix_world(spinner, Some(True))
  let tip = object3d.get_world_position(arm, v3(0.0, 0.0, 0.0))
  let passed =
    passed
    + check(near(vector3.get_x(tip), 0.0) && near(vector3.get_y(tip), 2.0), "rotated parent moves the child")
    + check(near(vector3.get_z(object3d.get_world_scale(arm, v3(0.0, 0.0, 0.0))), 1.0), "world scale")
  let _ = object3d.remove(parent, [child])
  let passed = passed + check(object3d.get_children(parent) == [], "remove")

  // ---- Mesh, geometry and a ray ----
  let geometry = box_geometry.new(Some(2.0), Some(2.0), Some(2.0), None, None, None)
  let material = mesh_basic_material.new(None)
  mesh_basic_material.set_color(material, red)
  let cube =
    mesh.new(Some(geometry), Some(material), t.mesh_basic_material_codec())
  object3d.set_name(cube, "cube")
  let _ = object3d.add(parent, [cube])
  object3d.update_matrix_world(parent, Some(True))
  let params = box_geometry.get_parameters(geometry)
  let passed =
    passed
    + check(object3d.get_type(cube) == "Mesh", "mesh type")
    + check(box_geometry.get_type(geometry) == "BoxGeometry", "geometry type")
    + check(box_geometry_parameters.get_width(params) == 2.0, "geometry parameters")
    + check(color.get_hex(mesh_basic_material.get_color(material), None) == 16_711_680.0, "material colour")
    + check(
      color.get_hex(mesh_basic_material.get_color(mesh.get_material(cube, t.mesh_basic_material_codec())), None) == 16_711_680.0,
      "the mesh's material",
    )
    + check(mesh.get_count(cube) == 1.0, "mesh count")

  // A ray from above the cube (the parent is at 1, 2, 3, so the near face is at z = 4).
  let caster = raycaster.new(Some(v3(1.3, 2.2, 13.0)), Some(v3(0.0, 0.0, -1.0)), None, None)
  let hits = raycaster.intersect_object(caster, cube, Some(False), None)
  let passed = passed + check(list.length(hits) == 1, "the ray hits the cube")
  let assert [first, ..] = hits
  let passed =
    passed
    + check(near(intersection.get_distance(first), 9.0), "hit distance")
    + check(near(vector3.get_z(intersection.get_point(first)), 4.0), "hit point")
    + check(
      object3d.get_name(intersection.get_object(first)) == "cube",
      "the hit object",
    )
  let miss = raycaster.new(Some(v3(20.0, 20.0, 13.0)), Some(v3(0.0, 0.0, -1.0)), None, None)
  let passed = passed + check(raycaster.intersect_object(miss, cube, Some(False), None) == [], "a ray that misses")

  // Attributes: positions of 24 vertices, 36 indices.
  let attributes = buffer_geometry.get_attributes(geometry, t.normal_buffer_attributes_codec())
  let assert Some(t.BufferAttributeOrInterleavedBufferAttributeBufferAttribute(position)) =
    normal_buffer_attributes.get_at(attributes, "position")
  let assert Some(t.BufferAttributeOrInterleavedBufferAttributeBufferAttribute(uv)) =
    normal_buffer_attributes.get_at(attributes, "uv")
  let assert Some(index) = buffer_geometry.get_index(geometry)
  let passed =
    passed
    + check(buffer_attribute.get_count(position) == 24.0, "position count")
    + check(buffer_attribute.get_item_size(position) == 3.0, "position item size")
    + check(buffer_attribute.get_item_size(uv) == 2.0, "uv item size")
    + check(buffer_attribute.get_count(index) == 36.0, "index count")
    + check(
      float.absolute_value(buffer_attribute.get_x(position, 0.0)) == 1.0,
      "a corner of the box",
    )
    + check(buffer_geometry.has_attribute(geometry, runtime.dynamic("normal")), "has normals")
    + check(!buffer_geometry.has_attribute(geometry, runtime.dynamic("color")), "no colours")
  buffer_geometry.compute_bounding_box(geometry)
  buffer_geometry.compute_bounding_sphere(geometry)
  let assert Some(bounds) = buffer_geometry.get_bounding_box(geometry)
  let assert Some(ball) = buffer_geometry.get_bounding_sphere(geometry)
  let passed =
    passed
    + check(vector3.to_list(box3.get_size(bounds, v3(0.0, 0.0, 0.0))) == [2.0, 2.0, 2.0], "bounding box")
    + check(near(sphere.get_radius(ball), 1.7320508075688772), "bounding sphere")
  buffer_geometry.translate(geometry, 1.0, 0.0, 0.0)
  buffer_geometry.compute_bounding_box(geometry)
  let assert Some(shifted) = buffer_geometry.get_bounding_box(geometry)
  let passed =
    passed
    + check(vector3.get_x(box3.get_min(shifted)) == 0.0, "translate the geometry")

  io.println(int.to_string(passed) <> " checks passed")
}
