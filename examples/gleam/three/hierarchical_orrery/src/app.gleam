import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/int
import gleam/option.{None, Some}
import three/areas/main/ambient_light
import three/areas/main/color
import three/areas/main/cylinder_geometry
import three/areas/main/directional_light
import three/areas/main/euler
import three/areas/main/grid_helper
import three/areas/main/group
import three/areas/main/icosahedron_geometry
import three/areas/main/mesh
import three/areas/main/mesh_basic_material
import three/areas/main/mesh_basic_material_parameters
import three/areas/main/mesh_standard_material
import three/areas/main/mesh_standard_material_parameters
import three/areas/main/object3d
import three/areas/main/octahedron_geometry
import three/areas/main/perspective_camera
import three/areas/main/scene
import three/areas/main/sphere_geometry
import three/areas/main/torus_geometry
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

@external(javascript, "./app_ffi.mjs", "roundInt")
fn round_int(x: Float) -> Int

@external(javascript, "./app_ffi.mjs", "fmt2")
fn fmt2(x: Float) -> String

fn std_mat(hex: Float, rough: Float, metal: Float, flat: Bool) -> t.MeshStandardMaterial(Nil) {
  let mp = mesh_standard_material_parameters.create()
  mesh_standard_material_parameters.set_color(mp, Some(t.ColorRepresentationNumber(hex)))
  mesh_standard_material_parameters.set_roughness(mp, Some(rough))
  mesh_standard_material_parameters.set_metalness(mp, Some(metal))
  mesh_standard_material_parameters.set_flat_shading(mp, Some(flat))
  mesh_standard_material.new(Some(mp))
}

fn fmt_v3(v: t.Vector3(Nil)) -> String {
  "(" <> fmt2(vector3.get_x(v)) <> ", " <> fmt2(vector3.get_y(v)) <> ", " <> fmt2(vector3.get_z(v)) <> ")"
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
  let _ = vector3.set(object3d.get_position(cam), 5.6, 5.2, Some(7.6))
  object3d.look_at(cam, vector3.new(Some(0.0), Some(-0.1), Some(0.0)))

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
      Some(t.ColorRepresentationNumber(10_066_329.0)),
      Some(t.ColorRepresentationNumber(15_066_597.0)),
    )
  vector3.set_y(object3d.get_position(grid), -1.5)
  let _ = object3d.add(sc, [grid])

  let pedestal =
    mesh.new(
      Some(cylinder_geometry.new(Some(0.55), Some(0.75), Some(0.35), Some(24.0), None, None, None, None)),
      Some(std_mat(2_500_134.0, 0.7, 0.05, False)),
      t.cylinder_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  vector3.set_y(object3d.get_position(pedestal), -1.3)
  let _ = object3d.add(sc, [pedestal])

  let core_sphere =
    mesh.new(
      Some(sphere_geometry.new(Some(0.65), Some(24.0), Some(18.0), None, None, None, None)),
      Some(std_mat(1_118_481.0, 0.45, 0.2, False)),
      t.sphere_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  let _ = object3d.add(sc, [core_sphere])

  let inner_pivot = group.new()
  let _ = object3d.add(sc, [inner_pivot])
  let tp1 = mesh_basic_material_parameters.create()
  mesh_basic_material_parameters.set_color(tp1, Some(t.ColorRepresentationNumber(10_066_329.0)))
  let inner_track =
    mesh.new(
      Some(torus_geometry.new(Some(2.1), Some(0.025), Some(8.0), Some(64.0), None, None, None)),
      Some(mesh_basic_material.new(Some(tp1))),
      t.torus_geometry_codec(),
      t.mesh_basic_material_codec(),
    )
  euler.set_x(object3d.get_rotation(inner_track), pi /. 2.0)
  let _ = object3d.add(inner_pivot, [inner_track])

  let inner_planet = group.new()
  let _ = vector3.set(object3d.get_position(inner_planet), 2.1, 0.0, Some(0.0))
  let _ = object3d.add(inner_pivot, [inner_planet])
  let planet =
    mesh.new(
      Some(sphere_geometry.new(Some(0.36), Some(20.0), Some(16.0), None, None, None, None)),
      Some(std_mat(5_592_405.0, 0.55, 0.1, False)),
      t.sphere_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  let _ = object3d.add(inner_planet, [planet])

  let moon_pivot = group.new()
  let _ = object3d.add(inner_planet, [moon_pivot])
  let moon =
    mesh.new(
      Some(octahedron_geometry.new(Some(0.18), Some(0.0))),
      Some(std_mat(13_948_116.0, 0.4, 0.1, True)),
      t.octahedron_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  let _ = vector3.set(object3d.get_position(moon), 0.75, 0.0, Some(0.0))
  let _ = object3d.add(moon_pivot, [moon])

  let outer_pivot = group.new()
  let _ = object3d.add(sc, [outer_pivot])
  let tp2 = mesh_basic_material_parameters.create()
  mesh_basic_material_parameters.set_color(tp2, Some(t.ColorRepresentationNumber(11_908_533.0)))
  let outer_track =
    mesh.new(
      Some(torus_geometry.new(Some(3.8), Some(0.025), Some(8.0), Some(80.0), None, None, None)),
      Some(mesh_basic_material.new(Some(tp2))),
      t.torus_geometry_codec(),
      t.mesh_basic_material_codec(),
    )
  euler.set_x(object3d.get_rotation(outer_track), pi /. 2.0)
  let _ = object3d.add(outer_pivot, [outer_track])

  let outer_station = group.new()
  let _ = vector3.set(object3d.get_position(outer_station), 3.8, 0.0, Some(0.0))
  let _ = object3d.add(outer_pivot, [outer_station])
  let station_body =
    mesh.new(
      Some(icosahedron_geometry.new(Some(0.42), Some(0.0))),
      Some(std_mat(4_210_752.0, 0.5, 0.1, True)),
      t.icosahedron_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  let _ = object3d.add(outer_station, [station_body])

  let halo =
    mesh.new(
      Some(torus_geometry.new(Some(0.68), Some(0.06), Some(10.0), Some(32.0), None, None, None)),
      Some(std_mat(1_118_481.0, 0.4, 0.1, False)),
      t.torus_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  euler.set_x(object3d.get_rotation(halo), 0.8)
  let _ = object3d.add(outer_station, [halo])

  let assert Some(tilt_btn) = document.get_element_by_id(doc, "btn-tilt")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let epoch_cell = new_cell(0.6)
  let tilt_cell = new_cell(False)

  let update_orrery = fn() {
    let epoch = get_cell(epoch_cell)
    let tilted = get_cell(tilt_cell)
    euler.set_y(object3d.get_rotation(inner_pivot), epoch)
    euler.set_y(object3d.get_rotation(moon_pivot), epoch *. 2.5)
    euler.set_y(object3d.get_rotation(outer_pivot), 0.0 -. epoch *. 0.55)
    euler.set_z(object3d.get_rotation(outer_pivot), case tilted { True -> 0.38 False -> 0.0 })
    euler.set_y(object3d.get_rotation(station_body), epoch *. 1.4)

    object3d.update_matrix_world(sc, Some(True))
    let mpos = object3d.get_world_position(moon, vector3.new(Some(0.0), Some(0.0), Some(0.0)))
    let spos = object3d.get_world_position(outer_station, vector3.new(Some(0.0), Some(0.0), Some(0.0)))

    dom_node.set_text_content(tilt_btn, Some(case tilted { True -> "Outer Tilt: On" False -> "Outer Tilt: Off" }))
    web_gl_renderer.render(renderer, sc, cam)

    let deg = round_int(epoch *. 180.0 /. pi)
    dom_node.set_text_content(
      meta_el,
      Some("epoch: " <> int.to_string(deg) <> " deg · moon world: " <> fmt_v3(mpos) <> " · station world: " <> fmt_v3(spos)),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
  }

  wire_click("btn-step", fn() {
    set_cell(epoch_cell, get_cell(epoch_cell) +. pi /. 6.0)
    update_orrery()
  })
  wire_click("btn-tilt", fn() {
    set_cell(tilt_cell, !get_cell(tilt_cell))
    update_orrery()
  })
  wire_click("btn-reset", fn() {
    set_cell(epoch_cell, 0.6)
    set_cell(tilt_cell, False)
    update_orrery()
  })

  update_orrery()
}
