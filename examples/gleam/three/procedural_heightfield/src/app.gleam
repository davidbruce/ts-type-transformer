import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{Some}
import gleam/result
import three/ambient_light
import three/box3
import three/buffer_attribute
import three/buffer_geometry
import three/color
import three/color_representation
import three/directional_light
import three/float32_buffer_attribute
import three/grid_helper
import three/mesh
import three/mesh_basic_material
import three/mesh_basic_material_parameters
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/perspective_camera
import three/points
import three/points_material
import three/points_material_parameters
import three/runtime as rt
import three/scene
import three/types as t
import three/vector3
import three/web_gl_renderer
import three/web_gl_renderer_parameters

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const grid_n = 28

const pi = 3.141592653589793

fn hex(value: Int) -> Float {
  int.to_float(value)
}

fn wave_height(mode: Int, x: Float, z: Float) -> Float {
  case mode {
    1 -> {
      let r = float.square_root(x *. x +. z *. z) |> result.unwrap(0.0)
      0.78 *. cos(r *. 2.2) /. { 1.0 +. 0.18 *. r *. r }
    }
    2 -> 0.065 *. { x *. x -. z *. z } +. 0.18 *. sin(x *. 1.6)
    _ ->
      0.55 *. cos(x *. 1.4) *. sin(z *. 1.4) +. 0.25 *. cos({ x +. z } *. 1.1)
  }
}

fn wave_name(mode: Int) -> String {
  case mode {
    1 -> "Radial Ripple"
    2 -> "Saddle Ridge"
    _ -> "Interference"
  }
}

fn build_indices() -> List(Float) {
  int.range(from: 0, to: grid_n - 1, with: [], run: fn(acc, iz) {
    int.range(from: 0, to: grid_n - 1, with: acc, run: fn(acc, ix) {
      let a = int.to_float(iz * grid_n + ix)
      let b = a +. 1.0
      let c = a +. int.to_float(grid_n)
      let d = c +. 1.0
      [d, c, b, b, c, a, ..acc]
    })
  })
  |> list.reverse
}

fn fill_grid(pos_attr: t.Float32BufferAttribute, mode: Int) -> Nil {
  let span = 7.6
  let half = span /. 2.0
  let denom = int.to_float(grid_n - 1)
  int.range(from: 0, to: grid_n * grid_n, with: Nil, run: fn(_, idx) {
    let iz = idx / grid_n
    let ix = idx % grid_n
    let x = 0.0 -. half +. span *. int.to_float(ix) /. denom
    let z = 0.0 -. half +. span *. int.to_float(iz) /. denom
    let y = wave_height(mode, x, z)
    buffer_attribute.set_xyz(pos_attr, int.to_float(idx), x, y, z)
    Nil
  })
}

fn fill_pts(pts_attr: t.Float32BufferAttribute, mode: Int) -> Nil {
  let span = 7.6
  let half = span /. 2.0
  let denom = int.to_float(grid_n - 1)
  let cols = grid_n / 2
  int.range(from: 0, to: cols * cols, with: Nil, run: fn(_, pi) {
    let iz = { pi / cols } * 2
    let ix = { pi % cols } * 2
    let x = 0.0 -. half +. span *. int.to_float(ix) /. denom
    let z = 0.0 -. half +. span *. int.to_float(iz) /. denom
    let y = wave_height(mode, x, z) +. 0.06
    buffer_attribute.set_xyz(pts_attr, int.to_float(pi), x, y, z)
    Nil
  })
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0

  let rp =
    web_gl_renderer_parameters.create()
    |> web_gl_renderer_parameters.with_antialias(True)
    |> web_gl_renderer_parameters.with_preserve_drawing_buffer(True)
  let renderer = web_gl_renderer.new_with(rp)
  web_gl_renderer.set_pixel_ratio(renderer, 1.0)
  web_gl_renderer.set_size(renderer, width, height)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  dom_node.append_child(stage, web_gl_renderer.get_dom_element(renderer))

  let sc = scene.new()
  scene.set_background_color(sc, color.new_number_with(hex(0xf7f7f7)))
  let cam = perspective_camera.new_with(42.0, width /. height, 0.1, 100.0)

  object3d.add(sc, [
    ambient_light.new_number_with(hex(0xffffff), 1.2),
  ])
  let key = directional_light.new_number_with(hex(0xffffff), 2.2)
  object3d.get_position(key)
  |> vector3.set_with(6.0, 10.0, 8.0)
  object3d.add(sc, [key])
  let rim = directional_light.new_number_with(hex(0xffffff), 0.9)
  object3d.get_position(rim)
  |> vector3.set_with(-7.0, 5.0, -6.0)
  object3d.add(sc, [rim])

  let grid =
    grid_helper.new_of_with(
      9.0,
      12.0,
      color_representation.Number(hex(0x999999)),
      color_representation.Number(hex(0xe5e5e5)),
    )
  vector3.set_y(object3d.get_position(grid), -1.1)
  object3d.add(sc, [grid])

  let geo = buffer_geometry.new()
  let vert_count = grid_n * grid_n
  let pos_attr =
    float32_buffer_attribute.new_number(int.to_float(vert_count * 3), 3.0)
  let indices = build_indices()
  let tri_count = list.length(indices) / 3
  buffer_geometry.set_attribute(geo, "position", pos_attr)
  buffer_geometry.set_index(geo, indices)

  let smp =
    mesh_standard_material_parameters.create()
    |> mesh_standard_material_parameters.with_color_number(hex(0xb5b5b5))
    |> mesh_standard_material_parameters.with_roughness(0.72)
    |> mesh_standard_material_parameters.with_metalness(0.05)
    |> mesh_standard_material_parameters.with_flat_shading(True)
  let surf = mesh.new_with(geo, mesh_standard_material.new_with(smp))
  object3d.add(sc, [surf])

  let wmp =
    mesh_basic_material_parameters.create()
    |> mesh_basic_material_parameters.with_color_number(hex(0x262626))
    |> mesh_basic_material_parameters.with_wireframe(True)
  let wire_mesh = mesh.new_with(geo, mesh_basic_material.new_with(wmp))
  vector3.set_y(object3d.get_position(wire_mesh), 0.015)
  object3d.add(sc, [wire_mesh])

  let pt_count = { grid_n / 2 } * { grid_n / 2 }
  let pts_geo = buffer_geometry.new()
  let pts_attr =
    float32_buffer_attribute.new_number(int.to_float(pt_count * 3), 3.0)
  buffer_geometry.set_attribute(pts_geo, "position", pts_attr)
  let pmp =
    points_material_parameters.create()
    |> points_material_parameters.with_color_number(hex(0x111111))
    |> points_material_parameters.with_size(0.14)
  let pts = points.new_with(pts_geo, points_material.new_with(pmp))
  object3d.add(sc, [pts])

  let assert Some(wire_btn) = document.get_element_by_id(doc, "btn-wire")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let mode_ref = rt.ref_new(0)
  let over_ref = rt.ref_new(True)
  let az_ref = rt.ref_new(0.62)

  let rebuild_field = fn() {
    let mode = rt.ref_get(mode_ref)
    let over = rt.ref_get(over_ref)
    let az = rt.ref_get(az_ref)
    fill_grid(pos_attr, mode)
    fill_pts(pts_attr, mode)
    buffer_attribute.set_needs_update(pos_attr, True)
    buffer_attribute.set_needs_update(pts_attr, True)
    buffer_geometry.compute_vertex_normals(geo)
    buffer_geometry.compute_bounding_box(geo)

    let r = 9.2
    object3d.get_position(cam)
    |> vector3.set_with(r *. sin(az), 5.4, r *. cos(az))
    object3d.look_at(cam, vector3.new_with(0.0, 0.0, 0.0))
    object3d.set_visible(wire_mesh, over)
    dom_node.set_text_content(
      wire_btn,
      Some(case over {
        True -> "Wireframe: On"
        False -> "Wireframe: Off"
      }),
    )
    web_gl_renderer.render(renderer, sc, cam)

    let assert Some(bb) = buffer_geometry.get_bounding_box(geo)
    let min_y = vector3.get_y(box3.get_min(bb))
    let max_y = vector3.get_y(box3.get_max(bb))
    dom_node.set_text_content(
      meta_el,
      Some(
        "profile: "
        <> wave_name(mode)
        <> " · "
        <> int.to_string(vert_count)
        <> " vertices · "
        <> int.to_string(tri_count)
        <> " triangles · y=["
        <> rt.to_fixed(min_y, 2)
        <> ", "
        <> rt.to_fixed(max_y, 2)
        <> "] · wireframe: "
        <> case over {
          True -> "on"
          False -> "off"
        },
      ),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() })
  }

  wire_click("btn-p0", fn() {
    rt.ref_set(mode_ref, 0)
    rebuild_field()
  })
  wire_click("btn-p1", fn() {
    rt.ref_set(mode_ref, 1)
    rebuild_field()
  })
  wire_click("btn-p2", fn() {
    rt.ref_set(mode_ref, 2)
    rebuild_field()
  })
  wire_click("btn-wire", fn() {
    rt.ref_update(over_ref, fn(o) { !o })
    rebuild_field()
  })
  wire_click("btn-orbit", fn() {
    rt.ref_update(az_ref, fn(a) { a +. pi /. 4.0 })
    rebuild_field()
  })

  rebuild_field()
}
