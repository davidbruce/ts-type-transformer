import dom/areas/core
import dom/areas/core/document
import dom/areas/core/dom_rect_read_only
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/events/mouse_event
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import three/ambient_light
import three/box3
import three/box3_helper
import three/box_geometry
import three/buffer_geometry
import three/color
import three/cone_geometry
import three/cylinder_geometry
import three/directional_light
import three/dodecahedron_geometry
import three/euler
import three/grid_helper
import three/icosahedron_geometry
import three/intersection
import three/mesh
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/octahedron_geometry
import three/perspective_camera
import three/raycaster
import three/runtime as rt
import three/scene
import three/sphere_geometry
import three/tetrahedron_geometry
import three/torus_geometry
import three/types as t
import three/vector2
import three/vector3
import three/web_gl_renderer
import three/web_gl_renderer_parameters

@external(javascript, "./app_ffi.mjs", "sin")
fn sin(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "cos")
fn cos(x: Float) -> Float

const pi = 3.141592653589793

fn hex(value: Int) -> Float {
  int.to_float(value)
}

type Specimen {
  Specimen(
    name: String,
    mat: t.MeshStandardMaterial(Nil),
    obj: t.Mesh(
      t.BufferGeometry(
        t.NormalBufferAttributes(Nil),
        t.BufferGeometryEventMap(Nil),
        Nil,
      ),
      t.MeshStandardMaterial(Nil),
      t.Object3DEventMap(Nil),
      Nil,
    ),
  )
}

fn find_by_id(
  specs: List(Specimen),
  target_id: Float,
  idx: Int,
) -> Option(#(Int, Specimen)) {
  case specs {
    [] -> None
    [s, ..rest] ->
      case object3d.get_id(s.obj) == target_id {
        True -> Some(#(idx, s))
        False -> find_by_id(rest, target_id, idx + 1)
      }
  }
}

fn nth_specimen(specs: List(Specimen), idx: Int) -> Specimen {
  let assert Ok(sp) = specs |> list.drop(idx) |> list.first
  sp
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
  let canvas = web_gl_renderer.get_dom_element(renderer)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  dom_node.append_child(stage, canvas)

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
      Some(9.0),
      Some(9.0),
      Some(t.ColorRepresentationNumber(hex(0x999999))),
      Some(t.ColorRepresentationNumber(hex(0xd4d4d4))),
    )
  vector3.set_y(object3d.get_position(grid), -0.85)
  object3d.add(sc, [grid])

  let defs: List(
    #(
      String,
      t.BufferGeometry(
        t.NormalBufferAttributes(Nil),
        t.BufferGeometryEventMap(Nil),
        Nil,
      ),
    ),
  ) = [
    #(
      "Box",
      buffer_geometry.upcast(box_geometry.new_with(
        Some(1.05),
        Some(1.05),
        Some(1.05),
        None,
        None,
        None,
      )),
    ),
    #(
      "Cone",
      buffer_geometry.upcast(cone_geometry.new_with(
        Some(0.62),
        Some(1.25),
        Some(20.0),
        None,
        None,
        None,
        None,
      )),
    ),
    #(
      "Cylinder",
      buffer_geometry.upcast(cylinder_geometry.new_with(
        Some(0.48),
        Some(0.48),
        Some(1.2),
        Some(24.0),
        None,
        None,
        None,
        None,
      )),
    ),
    #(
      "Sphere",
      buffer_geometry.upcast(sphere_geometry.new_with(
        Some(0.62),
        Some(24.0),
        Some(18.0),
        None,
        None,
        None,
        None,
      )),
    ),
    #(
      "Octahedron",
      buffer_geometry.upcast(octahedron_geometry.new_with(Some(0.68), Some(0.0))),
    ),
    #(
      "Dodecahedron",
      buffer_geometry.upcast(dodecahedron_geometry.new_with(
        Some(0.62),
        Some(0.0),
      )),
    ),
    #(
      "Torus",
      buffer_geometry.upcast(torus_geometry.new_with(
        Some(0.5),
        Some(0.18),
        Some(14.0),
        Some(36.0),
        None,
        None,
        None,
      )),
    ),
    #(
      "Icosahedron",
      buffer_geometry.upcast(icosahedron_geometry.new_with(
        Some(0.64),
        Some(0.0),
      )),
    ),
    #(
      "Tetrahedron",
      buffer_geometry.upcast(tetrahedron_geometry.new_with(
        Some(0.72),
        Some(0.0),
      )),
    ),
  ]

  let specs =
    list.index_map(defs, fn(item, i) {
      let #(name, geo) = item
      let gx = int.to_float(i % 3 - 1)
      let gz = int.to_float(i / 3 - 1)
      let mat =
        mesh_standard_material_parameters.create()
        |> mesh_standard_material_parameters.with_color(
          t.ColorRepresentationNumber(hex(0xd4d4d4)),
        )
        |> mesh_standard_material_parameters.with_roughness(0.55)
        |> mesh_standard_material_parameters.with_metalness(0.1)
        |> mesh_standard_material_parameters.with_flat_shading(i >= 4)
        |> Some
        |> mesh_standard_material.new_with
      let m = mesh.new(Some(geo), Some(mat), t.mesh_standard_material_codec())
      object3d.set_name(m, name)
      object3d.get_position(m)
      |> vector3.set_with(gx *. 2.1, 0.0, Some(gz *. 2.1))
      euler.set_x(object3d.get_rotation(m), 0.2)
      euler.set_y(object3d.get_rotation(m), 0.35 *. int.to_float(i))
      object3d.add(sc, [m])
      Specimen(name, mat, m)
    })

  let mesh_objs = list.map(specs, fn(s) { s.obj })
  let bx =
    box3.new_with(
      Some(vector3.new_with(Some(-0.5), Some(-0.5), Some(-0.5))),
      Some(vector3.new_with(Some(0.5), Some(0.5), Some(0.5))),
    )
  let helper =
    box3_helper.new_with(bx, Some(t.ColorRepresentationNumber(hex(0x111111))))
  object3d.set_visible(helper, False)
  object3d.add(sc, [helper])

  let caster = raycaster.new()
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let yaw_ref = rt.ref_new(0.45)
  let sel_ref = rt.ref_new(-1)
  let hit_ref = rt.ref_new("none")

  let update_camera = fn() {
    let yaw = rt.ref_get(yaw_ref)
    let r = 8.4
    object3d.get_position(cam)
    |> vector3.set_with(r *. sin(yaw), 5.1, Some(r *. cos(yaw)))
    object3d.look_at(cam, vector3.new_with(Some(0.0), Some(0.0), Some(0.0)))
    object3d.update_matrix_world_with(cam, Some(True))
    object3d.update_matrix_world_with(sc, Some(True))
  }

  let render_scene = fn() {
    let sel = rt.ref_get(sel_ref)
    list.index_fold(specs, Nil, fn(_, s, i) {
      let col_hex = case i == sel {
        True -> hex(0x111111)
        False -> hex(0xd4d4d4)
      }
      mesh_standard_material.get_color(s.mat)
      |> color.set_hex_with(col_hex, None)
      Nil
    })
    case sel >= 0 && sel < 9 {
      True -> {
        let target_sp = nth_specimen(specs, sel)
        box3.set_from_object_with(bx, target_sp.obj, Some(True))
        object3d.set_visible(helper, True)
      }
      False -> object3d.set_visible(helper, False)
    }
    web_gl_renderer.render(renderer, sc, cam)
    dom_node.set_text_content(
      meta_el,
      Some(
        "picked: "
        <> rt.ref_get(hit_ref)
        <> " · 9 specimens · click canvas or buttons",
      ),
    )
  }

  let cast_at_ndc = fn(nx: Float, ny: Float) {
    raycaster.set_from_camera(caster, vector2.new_with(Some(nx), Some(ny)), cam)
    case
      raycaster.intersect_objects_with(caster, mesh_objs, Some(False), None)
    {
      [first, ..] -> {
        let hit_obj = intersection.get_object(first)
        case find_by_id(specs, object3d.get_id(hit_obj), 0) {
          Some(#(idx, sp)) -> {
            rt.ref_set(sel_ref, idx)
            box3.set_from_object_with(bx, sp.obj, Some(True))
            let sz = box3.get_size(bx, vector3.new())
            let pt = intersection.get_point(first)
            rt.ref_set(
              hit_ref,
              sp.name
                <> " at d="
                <> rt.to_fixed(intersection.get_distance(first), 2)
                <> " "
                <> fmt_v3(pt)
                <> " box="
                <> rt.to_fixed(vector3.get_x(sz), 2)
                <> "x"
                <> rt.to_fixed(vector3.get_y(sz), 2)
                <> "x"
                <> rt.to_fixed(vector3.get_z(sz), 2),
            )
          }
          None -> {
            rt.ref_set(sel_ref, -1)
            rt.ref_set(hit_ref, "unknown")
          }
        }
      }
      [] -> {
        rt.ref_set(sel_ref, -1)
        rt.ref_set(
          hit_ref,
          "miss at ndc ("
            <> rt.to_fixed(nx, 2)
            <> ", "
            <> rt.to_fixed(ny, 2)
            <> ")",
        )
      }
    }
    render_scene()
  }

  let pick_specimen = fn(idx: Int) {
    update_camera()
    let sp = nth_specimen(specs, idx)
    let wp = object3d.get_world_position(sp.obj, vector3.new())
    let proj = vector3.project(wp, cam)
    cast_at_ndc(vector3.get_x(proj), vector3.get_y(proj))
  }

  html_element.add_event_listener(canvas, ev.click, fn(e) {
    let rect = element.get_bounding_client_rect(canvas)
    let nx =
      {
        { mouse_event.get_client_x(e) -. dom_rect_read_only.get_left(rect) }
        /. dom_rect_read_only.get_width(rect)
      }
      *. 2.0
      -. 1.0
    let ny =
      0.0
      -. {
        { mouse_event.get_client_y(e) -. dom_rect_read_only.get_top(rect) }
        /. dom_rect_read_only.get_height(rect)
      }
      *. 2.0
      +. 1.0
    update_camera()
    cast_at_ndc(nx, ny)
  })

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() })
  }

  wire_click("btn-p0", fn() { pick_specimen(4) })
  wire_click("btn-p1", fn() { pick_specimen(0) })
  wire_click("btn-p2", fn() { pick_specimen(8) })
  wire_click("btn-orbit", fn() {
    rt.ref_update(yaw_ref, fn(y) { y +. pi /. 4.0 })
    update_camera()
    render_scene()
  })
  wire_click("btn-clear", fn() {
    rt.ref_set(sel_ref, -1)
    rt.ref_set(hit_ref, "none")
    update_camera()
    render_scene()
  })

  pick_specimen(4)
}
