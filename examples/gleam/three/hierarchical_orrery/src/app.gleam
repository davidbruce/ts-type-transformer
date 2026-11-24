import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import three/ambient_light
import three/color
import three/cylinder_geometry
import three/directional_light
import three/euler
import three/grid_helper
import three/group
import three/icosahedron_geometry
import three/mesh
import three/mesh_basic_material
import three/mesh_basic_material_parameters
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/octahedron_geometry
import three/perspective_camera
import three/runtime as rt
import three/scene
import three/sphere_geometry
import three/torus_geometry
import three/types as t
import three/vector3
import three/web_gl_renderer
import three/web_gl_renderer_parameters

const pi = 3.141592653589793

fn hex(value: Int) -> Float {
  int.to_float(value)
}

fn std_mat(
  color_hex: Int,
  rough: Float,
  metal: Float,
  flat: Bool,
) -> t.MaterialOrMaterialArray {
  mesh_standard_material_parameters.create()
  |> mesh_standard_material_parameters.with_color(
    t.ColorRepresentationNumber(hex(color_hex)),
  )
  |> mesh_standard_material_parameters.with_roughness(rough)
  |> mesh_standard_material_parameters.with_metalness(metal)
  |> mesh_standard_material_parameters.with_flat_shading(flat)
  |> Some
  |> mesh_standard_material.new_with
  |> t.material_or_material_array_material
}

fn fmt_v3(v: t.Vector3(Nil)) -> String {
  "("
  <> rt.to_fixed(vector3.get_x(v), 2)
  <> ", "
  <> rt.to_fixed(vector3.get_y(v), 2)
  <> ", "
  <> rt.to_fixed(vector3.get_z(v), 2)
  <> ")"
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0

  let rp =
    web_gl_renderer_parameters.create()
    |> web_gl_renderer_parameters.with_antialias(True)
    |> web_gl_renderer_parameters.with_preserve_drawing_buffer(True)
  let renderer = web_gl_renderer.new_with(Some(rp))
  web_gl_renderer.set_pixel_ratio(renderer, 1.0)
  web_gl_renderer.set_size(renderer, width, height)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  dom_node.append_child(stage, web_gl_renderer.get_dom_element(renderer))

  let sc = scene.new()
  scene.set_background(
    sc,
    Some(
      t.ColorOrTextureColor(
        color.new_with(Some(t.ColorRepresentationNumber(hex(0xf7f7f7)))),
      ),
    ),
  )
  let cam =
    perspective_camera.new_with(
      Some(42.0),
      Some(width /. height),
      Some(0.1),
      Some(100.0),
    )
  object3d.get_position(cam)
  |> vector3.set_with(5.6, 5.2, Some(7.6))
  object3d.look_at(cam, vector3.new_with(Some(0.0), Some(-0.1), Some(0.0)))

  object3d.add(sc, [
    ambient_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(1.2),
    ),
  ])
  let key =
    directional_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(2.2),
    )
  object3d.get_position(key)
  |> vector3.set_with(6.0, 10.0, Some(8.0))
  object3d.add(sc, [key])
  let rim =
    directional_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(0.9),
    )
  object3d.get_position(rim)
  |> vector3.set_with(-7.0, 5.0, Some(-6.0))
  object3d.add(sc, [rim])

  let grid =
    grid_helper.new_with(
      Some(12.0),
      Some(12.0),
      Some(t.ColorRepresentationNumber(hex(0x999999))),
      Some(t.ColorRepresentationNumber(hex(0xe5e5e5))),
    )
  vector3.set_y(object3d.get_position(grid), -1.5)
  object3d.add(sc, [grid])

  let pedestal =
    mesh.new_with(
      Some(cylinder_geometry.new_with(
        Some(0.55),
        Some(0.75),
        Some(0.35),
        Some(24.0),
        None,
        None,
        None,
        None,
      )),
      Some(std_mat(0x262626, 0.7, 0.05, False)),
    )
  vector3.set_y(object3d.get_position(pedestal), -1.3)
  object3d.add(sc, [pedestal])

  let core_sphere =
    mesh.new_with(
      Some(sphere_geometry.new_with(
        Some(0.65),
        Some(24.0),
        Some(18.0),
        None,
        None,
        None,
        None,
      )),
      Some(std_mat(0x111111, 0.45, 0.2, False)),
    )
  object3d.add(sc, [core_sphere])

  let inner_pivot = group.new()
  object3d.add(sc, [inner_pivot])
  let tp1 =
    mesh_basic_material_parameters.create()
    |> mesh_basic_material_parameters.with_color(
      t.ColorRepresentationNumber(hex(0x999999)),
    )
  let inner_track =
    mesh.new_with(
      Some(torus_geometry.new_with(
        Some(2.1),
        Some(0.025),
        Some(8.0),
        Some(64.0),
        None,
        None,
        None,
      )),
      Some(
        t.material_or_material_array_material(
          mesh_basic_material.new_with(Some(tp1)),
        ),
      ),
    )
  euler.set_x(object3d.get_rotation(inner_track), pi /. 2.0)
  object3d.add(inner_pivot, [inner_track])

  let inner_planet = group.new()
  object3d.get_position(inner_planet)
  |> vector3.set_with(2.1, 0.0, Some(0.0))
  object3d.add(inner_pivot, [inner_planet])
  let planet =
    mesh.new_with(
      Some(sphere_geometry.new_with(
        Some(0.36),
        Some(20.0),
        Some(16.0),
        None,
        None,
        None,
        None,
      )),
      Some(std_mat(0x555555, 0.55, 0.1, False)),
    )
  object3d.add(inner_planet, [planet])

  let moon_pivot = group.new()
  object3d.add(inner_planet, [moon_pivot])
  let moon =
    mesh.new_with(
      Some(octahedron_geometry.new_with(Some(0.18), Some(0.0))),
      Some(std_mat(0xd4d4d4, 0.4, 0.1, True)),
    )
  object3d.get_position(moon)
  |> vector3.set_with(0.75, 0.0, Some(0.0))
  object3d.add(moon_pivot, [moon])

  let outer_pivot = group.new()
  object3d.add(sc, [outer_pivot])
  let tp2 =
    mesh_basic_material_parameters.create()
    |> mesh_basic_material_parameters.with_color(
      t.ColorRepresentationNumber(hex(0xb5b5b5)),
    )
  let outer_track =
    mesh.new_with(
      Some(torus_geometry.new_with(
        Some(3.8),
        Some(0.025),
        Some(8.0),
        Some(80.0),
        None,
        None,
        None,
      )),
      Some(
        t.material_or_material_array_material(
          mesh_basic_material.new_with(Some(tp2)),
        ),
      ),
    )
  euler.set_x(object3d.get_rotation(outer_track), pi /. 2.0)
  object3d.add(outer_pivot, [outer_track])

  let outer_station = group.new()
  object3d.get_position(outer_station)
  |> vector3.set_with(3.8, 0.0, Some(0.0))
  object3d.add(outer_pivot, [outer_station])
  let station_body =
    mesh.new_with(
      Some(icosahedron_geometry.new_with(Some(0.42), Some(0.0))),
      Some(std_mat(0x404040, 0.5, 0.1, True)),
    )
  object3d.add(outer_station, [station_body])

  let halo =
    mesh.new_with(
      Some(torus_geometry.new_with(
        Some(0.68),
        Some(0.06),
        Some(10.0),
        Some(32.0),
        None,
        None,
        None,
      )),
      Some(std_mat(0x111111, 0.4, 0.1, False)),
    )
  euler.set_x(object3d.get_rotation(halo), 0.8)
  object3d.add(outer_station, [halo])

  let assert Some(tilt_btn) = document.get_element_by_id(doc, "btn-tilt")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let epoch_ref = rt.ref_new(0.6)
  let tilt_ref = rt.ref_new(False)

  let update_orrery = fn() {
    let epoch = rt.ref_get(epoch_ref)
    let tilted = rt.ref_get(tilt_ref)
    euler.set_y(object3d.get_rotation(inner_pivot), epoch)
    euler.set_y(object3d.get_rotation(moon_pivot), epoch *. 2.5)
    euler.set_y(object3d.get_rotation(outer_pivot), 0.0 -. epoch *. 0.55)
    euler.set_z(object3d.get_rotation(outer_pivot), case tilted {
      True -> 0.38
      False -> 0.0
    })
    euler.set_y(object3d.get_rotation(station_body), epoch *. 1.4)

    object3d.update_matrix_world_with(sc, Some(True))
    let mpos =
      object3d.get_world_position(
        moon,
        vector3.new_with(Some(0.0), Some(0.0), Some(0.0)),
      )
    let spos =
      object3d.get_world_position(
        outer_station,
        vector3.new_with(Some(0.0), Some(0.0), Some(0.0)),
      )

    dom_node.set_text_content(
      tilt_btn,
      Some(case tilted {
        True -> "Outer Tilt: On"
        False -> "Outer Tilt: Off"
      }),
    )
    web_gl_renderer.render(renderer, sc, cam)

    let deg = float.round(epoch *. 180.0 /. pi)
    dom_node.set_text_content(
      meta_el,
      Some(
        "epoch: "
        <> int.to_string(deg)
        <> " deg · moon world: "
        <> fmt_v3(mpos)
        <> " · station world: "
        <> fmt_v3(spos),
      ),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() })
  }

  wire_click("btn-step", fn() {
    rt.ref_update(epoch_ref, fn(e) { e +. pi /. 6.0 })
    update_orrery()
  })
  wire_click("btn-tilt", fn() {
    rt.ref_update(tilt_ref, fn(t) { !t })
    update_orrery()
  })
  wire_click("btn-reset", fn() {
    rt.ref_set(epoch_ref, 0.6)
    rt.ref_set(tilt_ref, False)
    update_orrery()
  })

  update_orrery()
}
