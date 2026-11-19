import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/int
import gleam/option.{None, Some}
import three/areas/main/ambient_light
import three/areas/main/box_geometry
import three/areas/main/color
import three/areas/main/directional_light
import three/areas/main/edges_geometry
import three/areas/main/euler
import three/areas/main/grid_helper
import three/areas/main/group
import three/areas/main/icosahedron_geometry
import three/areas/main/line_basic_material
import three/areas/main/line_basic_material_parameters
import three/areas/main/line_segments
import three/areas/main/mesh
import three/areas/main/mesh_standard_material
import three/areas/main/mesh_standard_material_parameters
import three/areas/main/object3d
import three/areas/main/octahedron_geometry
import three/areas/main/perspective_camera
import three/areas/main/scene
import three/areas/main/torus_geometry
import three/areas/main/torus_knot_geometry
import three/areas/main/vector3
import three/areas/main/web_gl_renderer
import three/areas/main/web_gl_renderer_parameters
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

@external(javascript, "./app_ffi.mjs", "roundInt")
fn round_int(x: Float) -> Int

fn add_solid(
  sc: t.Scene(t.Object3DEventMap(Nil), Nil),
  wire_grp: t.Group(t.Object3DEventMap(Nil), Nil),
  geo: t.BufferGeometry(t.NormalBufferAttributes(Nil), t.BufferGeometryEventMap(Nil), Nil),
  hex: Float,
  rough: Float,
  metal: Float,
  flat: Bool,
  x: Float,
  y: Float,
  z: Float,
  thresh: Float,
  line_mat: t.LineBasicMaterial(Nil),
) -> Nil {
  let mp = mesh_standard_material_parameters.create()
  mesh_standard_material_parameters.set_color(mp, Some(t.ColorRepresentationNumber(hex)))
  mesh_standard_material_parameters.set_roughness(mp, Some(rough))
  mesh_standard_material_parameters.set_metalness(mp, Some(metal))
  mesh_standard_material_parameters.set_flat_shading(mp, Some(flat))
  let mat = mesh_standard_material.new(Some(mp))

  let m = mesh.new(Some(geo), Some(mat), t.buffer_geometry_codec(), t.mesh_standard_material_codec())
  let _ = vector3.set(object3d.get_position(m), x, y, Some(z))
  euler.set_x(object3d.get_rotation(m), 0.25)
  euler.set_y(object3d.get_rotation(m), 0.45)
  let _ = object3d.add(sc, [m])

  let egeo = edges_geometry.new(Some(geo), Some(thresh), t.buffer_geometry_codec())
  let segs = line_segments.new(Some(egeo), Some(line_mat), t.edges_geometry_codec(), t.line_basic_material_codec())
  let _ = vector3.set(object3d.get_position(segs), x, y, Some(z))
  euler.set_x(object3d.get_rotation(segs), 0.25)
  euler.set_y(object3d.get_rotation(segs), 0.45)
  let _ = object3d.add(wire_grp, [segs])
  Nil
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
      Some(12.0),
      Some(12.0),
      Some(t.ColorRepresentationNumber(7_566_195.0)),
      Some(t.ColorRepresentationNumber(13_948_116.0)),
    )
  vector3.set_y(object3d.get_position(grid), -1.2)
  let _ = object3d.add(sc, [grid])

  let wire_grp = group.new()
  let _ = object3d.add(sc, [wire_grp])

  let lmp = line_basic_material_parameters.create()
  line_basic_material_parameters.set_color(lmp, Some(t.ColorRepresentationNumber(1_118_481.0)))
  let line_mat = line_basic_material.new(Some(lmp))

  add_solid(sc, wire_grp, coerce(box_geometry.new(Some(1.45), Some(1.45), Some(1.45), None, None, None)), 2_500_134.0, 0.65, 0.1, False, -3.2, 0.0, -0.6, 1.0, line_mat)
  add_solid(sc, wire_grp, coerce(icosahedron_geometry.new(Some(0.95), Some(0.0))), 13_948_116.0, 0.55, 0.08, True, -1.55, 0.1, 0.55, 1.0, line_mat)
  add_solid(sc, wire_grp, coerce(torus_knot_geometry.new(Some(0.72), Some(0.24), Some(96.0), Some(16.0), None, None)), 7_566_195.0, 0.45, 0.15, False, 0.0, 0.25, -0.2, 24.0, line_mat)
  add_solid(sc, wire_grp, coerce(octahedron_geometry.new(Some(0.95), Some(0.0))), 15_724_527.0, 0.5, 0.05, True, 1.6, 0.1, 0.55, 1.0, line_mat)
  add_solid(sc, wire_grp, coerce(torus_geometry.new(Some(0.72), Some(0.23), Some(16.0), Some(48.0), None, None, None)), 4_210_752.0, 0.55, 0.12, False, 3.2, 0.05, -0.5, 22.0, line_mat)

  let assert Some(wire_btn) = document.get_element_by_id(doc, "btn-wire")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let wire_cell = new_cell(True)
  let az_cell = new_cell(0.35)

  let update_view = fn() {
    let az = get_cell(az_cell)
    let wv = get_cell(wire_cell)
    let r = 8.6
    let _ = vector3.set(object3d.get_position(cam), r *. sin(az), 3.6, Some(r *. cos(az)))
    object3d.look_at(cam, vector3.new(Some(0.0), Some(0.0), Some(0.0)))
    object3d.set_visible(wire_grp, wv)
    dom_node.set_text_content(wire_btn, Some(case wv { True -> "Wireframes: On" False -> "Wireframes: Off" }))
    web_gl_renderer.render(renderer, sc, cam)

    let deg = round_int(az *. 180.0 /. pi)
    let wlbl = case wv { True -> "on" False -> "off" }
    dom_node.set_text_content(
      meta_el,
      Some("5 monochrome solids · EdgesGeometry wireframes: " <> wlbl <> " · azimuth: " <> int.to_string(deg) <> " deg · 680x400 WebGL"),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
  }

  wire_click("btn-wire", fn() {
    set_cell(wire_cell, !get_cell(wire_cell))
    update_view()
  })
  wire_click("btn-left", fn() {
    set_cell(az_cell, get_cell(az_cell) -. pi /. 6.0)
    update_view()
  })
  wire_click("btn-right", fn() {
    set_cell(az_cell, get_cell(az_cell) +. pi /. 6.0)
    update_view()
  })
  wire_click("btn-reset", fn() {
    set_cell(az_cell, 0.35)
    set_cell(wire_cell, True)
    update_view()
  })

  update_view()
}
