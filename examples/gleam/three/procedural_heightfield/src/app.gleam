import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import three/areas/main/ambient_light
import three/areas/main/box3
import three/areas/main/buffer_attribute
import three/areas/main/buffer_geometry
import three/areas/main/color
import three/areas/main/directional_light
import three/areas/main/float32_buffer_attribute
import three/areas/main/grid_helper
import three/areas/main/mesh
import three/areas/main/mesh_basic_material
import three/areas/main/mesh_basic_material_parameters
import three/areas/main/mesh_standard_material
import three/areas/main/mesh_standard_material_parameters
import three/areas/main/object3d
import three/areas/main/perspective_camera
import three/areas/main/points
import three/areas/main/points_material
import three/areas/main/points_material_parameters
import three/areas/main/scene
import three/areas/main/vector3
import three/areas/main/web_gl_renderer
import three/areas/main/web_gl_renderer_parameters
import three/runtime as rt
import three/types as t

type Cell(a)

@external(javascript, "./app_ffi.mjs", "coerce")
fn coerce(a: a) -> b

@external(javascript, "./app_ffi.mjs", "newCell")
fn new_cell(v: a) -> Cell(a)

@external(javascript, "./app_ffi.mjs", "getCell")
fn get_cell(c: Cell(a)) -> a

@external(javascript, "./app_ffi.mjs", "setCell")
fn set_cell(c: Cell(a), v: a) -> Nil

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "sqrt")
fn sqrt(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "fmt2")
fn fmt2(x: Float) -> String

const grid_n = 28

fn wave_height(mode: Int, x: Float, z: Float) -> Float {
  case mode {
    1 -> {
      let r = sqrt(x *. x +. z *. z)
      0.78 *. cos(r *. 2.2) /. { 1.0 +. 0.18 *. r *. r }
    }
    2 -> 0.065 *. { x *. x -. z *. z } +. 0.18 *. sin(x *. 1.6)
    _ -> 0.55 *. cos(x *. 1.4) *. sin(z *. 1.4) +. 0.25 *. cos({ x +. z } *. 1.1)
  }
}

fn wave_name(mode: Int) -> String {
  case mode {
    1 -> "Radial Ripple"
    2 -> "Saddle Ridge"
    _ -> "Interference"
  }
}

fn build_indices(iz: Int, ix: Int, acc: List(Float)) -> List(Float) {
  case iz >= grid_n - 1 {
    True -> list.reverse(acc)
    False ->
      case ix >= grid_n - 1 {
        True -> build_indices(iz + 1, 0, acc)
        False -> {
          let a = int.to_float(iz * grid_n + ix)
          let b = a +. 1.0
          let c = a +. int.to_float(grid_n)
          let d = c +. 1.0
          build_indices(iz, ix + 1, [d, c, b, b, c, a, ..acc])
        }
      }
  }
}

fn fill_grid(pos_attr: t.Float32BufferAttribute(Nil), mode: Int, iz: Int, ix: Int) -> Nil {
  let span = 7.6
  let half = span /. 2.0
  let denom = int.to_float(grid_n - 1)
  case iz >= grid_n {
    True -> Nil
    False ->
      case ix >= grid_n {
        True -> fill_grid(pos_attr, mode, iz + 1, 0)
        False -> {
          let idx = iz * grid_n + ix
          let x = 0.0 -. half +. span *. int.to_float(ix) /. denom
          let z = 0.0 -. half +. span *. int.to_float(iz) /. denom
          let y = wave_height(mode, x, z)
          let _ = buffer_attribute.set_xyz(pos_attr, int.to_float(idx), x, y, z)
          fill_grid(pos_attr, mode, iz, ix + 1)
        }
      }
  }
}

fn fill_pts(pts_attr: t.Float32BufferAttribute(Nil), mode: Int, iz: Int, ix: Int, pi: Int) -> Nil {
  let span = 7.6
  let half = span /. 2.0
  let denom = int.to_float(grid_n - 1)
  case iz >= grid_n {
    True -> Nil
    False ->
      case ix >= grid_n {
        True -> fill_pts(pts_attr, mode, iz + 2, 0, pi)
        False -> {
          let x = 0.0 -. half +. span *. int.to_float(ix) /. denom
          let z = 0.0 -. half +. span *. int.to_float(iz) /. denom
          let y = wave_height(mode, x, z) +. 0.06
          let _ = buffer_attribute.set_xyz(pts_attr, int.to_float(pi), x, y, z)
          fill_pts(pts_attr, mode, iz, ix + 2, pi + 1)
        }
      }
  }
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0
  let pi = 3.141592653589793

  let rp = web_gl_renderer_parameters.create()
  web_gl_renderer_parameters.set_antialias(rp, Some(True))
  web_gl_renderer_parameters.set_preserve_drawing_buffer(rp, Some(True))
  let renderer = web_gl_renderer.new(Some(rp))
  web_gl_renderer.set_pixel_ratio(renderer, 1.0)
  web_gl_renderer.set_size(renderer, width, height, None)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  let _ = dom_node.append_child(stage, web_gl_renderer.get_dom_element(renderer))

  let sc = scene.new()
  scene.set_background(sc, Some(t.ColorOrTextureColor(color.new(Some(t.ColorRepresentationNumber(16_250_871.0))))))
  let cam = perspective_camera.new(Some(42.0), Some(width /. height), Some(0.1), Some(100.0))

  let _ = object3d.add(sc, [ambient_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(1.2))])
  let key = directional_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(2.2))
  let _ = vector3.set(object3d.get_position(key), 6.0, 10.0, Some(8.0))
  let _ = object3d.add(sc, [key])
  let rim = directional_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(0.9))
  let _ = vector3.set(object3d.get_position(rim), -7.0, 5.0, Some(-6.0))
  let _ = object3d.add(sc, [rim])

  let grid =
    grid_helper.new(
      Some(9.0),
      Some(12.0),
      Some(t.ColorRepresentationNumber(10_066_329.0)),
      Some(t.ColorRepresentationNumber(15_066_597.0)),
    )
  vector3.set_y(object3d.get_position(grid), -1.1)
  let _ = object3d.add(sc, [grid])

  let geo = buffer_geometry.new()
  let vert_count = grid_n * grid_n
  let pos_attr = float32_buffer_attribute.new(rt.dynamic(vert_count * 3), 3.0, None)
  let indices = build_indices(0, 0, [])
  let tri_count = list.length(indices) / 3
  let _ = buffer_geometry.set_attribute(geo, "position", t.WebGLAttributesGetAttributeBufferAttribute(coerce(pos_attr)), rt.codec_string())
  let _ = buffer_geometry.set_index_2(geo, Some(t.BufferAttributeOrNumberArrayArray(indices)))

  let smp = mesh_standard_material_parameters.create()
  mesh_standard_material_parameters.set_color(smp, Some(t.ColorRepresentationNumber(11_908_533.0)))
  mesh_standard_material_parameters.set_roughness(smp, Some(0.72))
  mesh_standard_material_parameters.set_metalness(smp, Some(0.05))
  mesh_standard_material_parameters.set_flat_shading(smp, Some(True))
  let surf = mesh.new(Some(geo), Some(mesh_standard_material.new(Some(smp))), t.buffer_geometry_codec(), t.mesh_standard_material_codec())
  let _ = object3d.add(sc, [surf])

  let wmp = mesh_basic_material_parameters.create()
  mesh_basic_material_parameters.set_color(wmp, Some(t.ColorRepresentationNumber(2_500_134.0)))
  mesh_basic_material_parameters.set_wireframe(wmp, Some(True))
  let wire_mesh = mesh.new(Some(geo), Some(mesh_basic_material.new(Some(wmp))), t.buffer_geometry_codec(), t.mesh_basic_material_codec())
  vector3.set_y(object3d.get_position(wire_mesh), 0.015)
  let _ = object3d.add(sc, [wire_mesh])

  let pt_count = { grid_n / 2 } * { grid_n / 2 }
  let pts_geo = buffer_geometry.new()
  let pts_attr = float32_buffer_attribute.new(rt.dynamic(pt_count * 3), 3.0, None)
  let _ = buffer_geometry.set_attribute(pts_geo, "position", t.WebGLAttributesGetAttributeBufferAttribute(coerce(pts_attr)), rt.codec_string())
  let pmp = points_material_parameters.create()
  points_material_parameters.set_color(pmp, Some(t.ColorRepresentationNumber(1_118_481.0)))
  points_material_parameters.set_size(pmp, Some(0.14))
  let pts = points.new(Some(pts_geo), Some(points_material.new(Some(pmp))), t.buffer_geometry_codec(), t.points_material_codec())
  let _ = object3d.add(sc, [pts])

  let assert Some(wire_btn) = document.get_element_by_id(doc, "btn-wire")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let mode_cell = new_cell(0)
  let over_cell = new_cell(True)
  let az_cell = new_cell(0.62)

  let rebuild_field = fn() {
    let mode = get_cell(mode_cell)
    let over = get_cell(over_cell)
    let az = get_cell(az_cell)
    fill_grid(pos_attr, mode, 0, 0)
    fill_pts(pts_attr, mode, 0, 0, 0)
    buffer_attribute.set_needs_update(pos_attr, True)
    buffer_attribute.set_needs_update(pts_attr, True)
    buffer_geometry.compute_vertex_normals(geo)
    buffer_geometry.compute_bounding_box(geo)

    let r = 9.2
    let _ = vector3.set(object3d.get_position(cam), r *. sin(az), 5.4, Some(r *. cos(az)))
    object3d.look_at(cam, vector3.new(Some(0.0), Some(0.0), Some(0.0)))
    object3d.set_visible(wire_mesh, over)
    dom_node.set_text_content(wire_btn, Some(case over { True -> "Wireframe: On" False -> "Wireframe: Off" }))
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
        <> fmt2(min_y)
        <> ", "
        <> fmt2(max_y)
        <> "] · wireframe: "
        <> case over { True -> "on" False -> "off" },
      ),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
  }

  wire_click("btn-p0", fn() {
    set_cell(mode_cell, 0)
    rebuild_field()
  })
  wire_click("btn-p1", fn() {
    set_cell(mode_cell, 1)
    rebuild_field()
  })
  wire_click("btn-p2", fn() {
    set_cell(mode_cell, 2)
    rebuild_field()
  })
  wire_click("btn-wire", fn() {
    set_cell(over_cell, !get_cell(over_cell))
    rebuild_field()
  })
  wire_click("btn-orbit", fn() {
    set_cell(az_cell, get_cell(az_cell) +. pi /. 4.0)
    rebuild_field()
  })

  rebuild_field()
}
