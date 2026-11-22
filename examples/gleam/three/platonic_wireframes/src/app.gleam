import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import three/ambient_light
import three/box_geometry
import three/buffer_geometry
import three/color
import three/directional_light
import three/edges_geometry
import three/euler
import three/grid_helper
import three/group
import three/icosahedron_geometry
import three/line_basic_material
import three/line_basic_material_parameters
import three/line_segments
import three/mesh
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/octahedron_geometry
import three/perspective_camera
import three/scene
import three/torus_geometry
import three/torus_knot_geometry
import three/vector3
import three/web_gl_renderer
import three/web_gl_renderer_parameters
import three/runtime as rt
import three/types as t

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const pi = 3.141592653589793

fn hex(value: Int) -> Float {
  int.to_float(value)
}

fn add_solid(
  sc: t.Scene(t.Object3DEventMap(Nil), Nil),
  wire_grp: t.Group(t.Object3DEventMap(Nil), Nil),
  geo: t.BufferGeometry(t.NormalBufferAttributes(Nil), t.BufferGeometryEventMap(Nil), Nil),
  color_hex: Int,
  rough: Float,
  metal: Float,
  flat: Bool,
  x: Float,
  y: Float,
  z: Float,
  thresh: Float,
  line_mat: t.LineBasicMaterial(Nil),
) -> Nil {
  let mat =
    mesh_standard_material_parameters.create()
    |> mesh_standard_material_parameters.with_color(t.ColorRepresentationNumber(hex(color_hex)))
    |> mesh_standard_material_parameters.with_roughness(rough)
    |> mesh_standard_material_parameters.with_metalness(metal)
    |> mesh_standard_material_parameters.with_flat_shading(flat)
    |> Some
    |> mesh_standard_material.new

  let m = mesh.new(Some(geo), Some(mat), t.mesh_standard_material_codec())
  object3d.get_position(m)
  |> vector3.set(x, y, Some(z))
  euler.set_x(object3d.get_rotation(m), 0.25)
  euler.set_y(object3d.get_rotation(m), 0.45)
  object3d.add(sc, [m])

  let egeo = edges_geometry.new(Some(geo), Some(thresh))
  let segs = line_segments.new(Some(egeo), Some(line_mat), t.line_basic_material_codec())
  object3d.get_position(segs)
  |> vector3.set(x, y, Some(z))
  euler.set_x(object3d.get_rotation(segs), 0.25)
  euler.set_y(object3d.get_rotation(segs), 0.45)
  object3d.add(wire_grp, [segs])
  Nil
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0

  let rp =
    web_gl_renderer_parameters.create()
    |> web_gl_renderer_parameters.with_antialias(True)
    |> web_gl_renderer_parameters.with_preserve_drawing_buffer(True)
  let renderer = web_gl_renderer.new(Some(rp))
  web_gl_renderer.set_pixel_ratio(renderer, 1.0)
  web_gl_renderer.set_size(renderer, width, height, None)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  dom_node.append_child(stage, web_gl_renderer.get_dom_element(renderer))

  let sc = scene.new()
  scene.set_background(sc, Some(t.ColorOrTextureColor(color.new(Some(t.ColorRepresentationNumber(hex(0xf7f7f7)))))))
  let cam = perspective_camera.new(Some(42.0), Some(width /. height), Some(0.1), Some(100.0))

  object3d.add(sc, [ambient_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(1.2))])
  let key = directional_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(2.2))
  object3d.get_position(key)
  |> vector3.set(6.0, 10.0, Some(8.0))
  object3d.add(sc, [key])
  let rim = directional_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(0.9))
  object3d.get_position(rim)
  |> vector3.set(-7.0, 5.0, Some(-6.0))
  object3d.add(sc, [rim])

  let grid =
    grid_helper.new(
      Some(12.0),
      Some(12.0),
      Some(t.ColorRepresentationNumber(hex(0x737373))),
      Some(t.ColorRepresentationNumber(hex(0xd4d4d4))),
    )
  vector3.set_y(object3d.get_position(grid), -1.2)
  object3d.add(sc, [grid])

  let wire_grp = group.new()
  object3d.add(sc, [wire_grp])

  let lmp =
    line_basic_material_parameters.create()
    |> line_basic_material_parameters.with_color(t.ColorRepresentationNumber(hex(0x111111)))
  let line_mat = line_basic_material.new(Some(lmp))

  add_solid(sc, wire_grp, buffer_geometry.upcast(box_geometry.new(Some(1.45), Some(1.45), Some(1.45), None, None, None)), 0x262626, 0.65, 0.1, False, -3.2, 0.0, -0.6, 1.0, line_mat)
  add_solid(sc, wire_grp, buffer_geometry.upcast(icosahedron_geometry.new(Some(0.95), Some(0.0))), 0xd4d4d4, 0.55, 0.08, True, -1.55, 0.1, 0.55, 1.0, line_mat)
  add_solid(sc, wire_grp, buffer_geometry.upcast(torus_knot_geometry.new(Some(0.72), Some(0.24), Some(96.0), Some(16.0), None, None)), 0x737373, 0.45, 0.15, False, 0.0, 0.25, -0.2, 24.0, line_mat)
  add_solid(sc, wire_grp, buffer_geometry.upcast(octahedron_geometry.new(Some(0.95), Some(0.0))), 0xefefef, 0.5, 0.05, True, 1.6, 0.1, 0.55, 1.0, line_mat)
  add_solid(sc, wire_grp, buffer_geometry.upcast(torus_geometry.new(Some(0.72), Some(0.23), Some(16.0), Some(48.0), None, None, None)), 0x404040, 0.55, 0.12, False, 3.2, 0.05, -0.5, 22.0, line_mat)

  let assert Some(wire_btn) = document.get_element_by_id(doc, "btn-wire")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let wire_ref = rt.ref_new(True)
  let az_ref = rt.ref_new(0.35)

  let update_view = fn() {
    let az = rt.ref_get(az_ref)
    let wv = rt.ref_get(wire_ref)
    let r = 8.6
    object3d.get_position(cam)
    |> vector3.set(r *. sin(az), 3.6, Some(r *. cos(az)))
    object3d.look_at(cam, vector3.new(Some(0.0), Some(0.0), Some(0.0)))
    object3d.set_visible(wire_grp, wv)
    dom_node.set_text_content(wire_btn, Some(case wv { True -> "Wireframes: On" False -> "Wireframes: Off" }))
    web_gl_renderer.render(renderer, sc, cam)

    let deg = float.round(az *. 180.0 /. pi)
    let wlbl = case wv { True -> "on" False -> "off" }
    dom_node.set_text_content(
      meta_el,
      Some("5 monochrome solids · EdgesGeometry wireframes: " <> wlbl <> " · azimuth: " <> int.to_string(deg) <> " deg · 680x400 WebGL"),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-wire", fn() {
    rt.ref_update(wire_ref, fn(w) { !w })
    update_view()
  })
  wire_click("btn-left", fn() {
    rt.ref_update(az_ref, fn(a) { a -. pi /. 6.0 })
    update_view()
  })
  wire_click("btn-right", fn() {
    rt.ref_update(az_ref, fn(a) { a +. pi /. 6.0 })
    update_view()
  })
  wire_click("btn-reset", fn() {
    rt.ref_set(az_ref, 0.35)
    rt.ref_set(wire_ref, True)
    update_view()
  })

  update_view()
}
