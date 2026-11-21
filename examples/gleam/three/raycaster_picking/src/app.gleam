import dom/areas/core
import dom/areas/core/dom_rect_read_only
import dom/areas/core/document
import dom/areas/core/element
import dom/areas/core/node as dom_node
import dom/areas/events/mouse_event
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/float
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
import three/scene
import three/sphere_geometry
import three/tetrahedron_geometry
import three/torus_geometry
import three/vector2
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

fn fmt2(x: Float) -> String {
  let neg = x <. 0.0
  let scaled = float.round(float.absolute_value(x) *. 100.0)
  let whole = scaled / 100
  let frac = scaled % 100
  let frac_str = case frac < 10 {
    True -> "0" <> int.to_string(frac)
    False -> int.to_string(frac)
  }
  let sign = case neg && scaled > 0 {
    True -> "-"
    False -> ""
  }
  sign <> int.to_string(whole) <> "." <> frac_str
}

type Specimen {
  Specimen(
    name: String,
    mat: t.MeshStandardMaterial(Nil),
    obj: t.Mesh(t.BufferGeometry(t.NormalBufferAttributes(Nil), t.BufferGeometryEventMap(Nil), Nil), t.MeshStandardMaterial(Nil), t.Object3DEventMap(Nil), Nil),
  )
}

fn find_by_id(specs: List(Specimen), target_id: Float, idx: Int) -> Option(#(Int, Specimen)) {
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
  "(" <> fmt2(vector3.get_x(v)) <> ", " <> fmt2(vector3.get_y(v)) <> ", " <> fmt2(vector3.get_z(v)) <> ")"
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
  let canvas = web_gl_renderer.get_dom_element(renderer)

  let assert Some(stage) = document.get_element_by_id(doc, "stage")
  let _ = dom_node.append_child(stage, canvas)

  let sc = scene.new()
  scene.set_background(sc, Some(t.ColorOrTextureColor(color.new(Some(t.ColorRepresentationNumber(hex(0xf7f7f7)))))))
  let cam = perspective_camera.new(Some(42.0), Some(width /. height), Some(0.1), Some(100.0))

  let _ = object3d.add(sc, [ambient_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(1.2))])
  let key = directional_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(2.2))
  let _ = vector3.set(object3d.get_position(key), 6.0, 10.0, Some(8.0))
  let _ = object3d.add(sc, [key])
  let rim = directional_light.new(Some(t.ColorRepresentationNumber(hex(0xffffff))), Some(0.9))
  let _ = vector3.set(object3d.get_position(rim), -7.0, 5.0, Some(-6.0))
  let _ = object3d.add(sc, [rim])

  let grid =
    grid_helper.new(
      Some(9.0),
      Some(9.0),
      Some(t.ColorRepresentationNumber(hex(0x999999))),
      Some(t.ColorRepresentationNumber(hex(0xd4d4d4))),
    )
  vector3.set_y(object3d.get_position(grid), -0.85)
  let _ = object3d.add(sc, [grid])

  let defs: List(#(String, t.BufferGeometry(t.NormalBufferAttributes(Nil), t.BufferGeometryEventMap(Nil), Nil))) = [
    #("Box", buffer_geometry.upcast(box_geometry.new(Some(1.05), Some(1.05), Some(1.05), None, None, None))),
    #("Cone", buffer_geometry.upcast(cone_geometry.new(Some(0.62), Some(1.25), Some(20.0), None, None, None, None))),
    #("Cylinder", buffer_geometry.upcast(cylinder_geometry.new(Some(0.48), Some(0.48), Some(1.2), Some(24.0), None, None, None, None))),
    #("Sphere", buffer_geometry.upcast(sphere_geometry.new(Some(0.62), Some(24.0), Some(18.0), None, None, None, None))),
    #("Octahedron", buffer_geometry.upcast(octahedron_geometry.new(Some(0.68), Some(0.0)))),
    #("Dodecahedron", buffer_geometry.upcast(dodecahedron_geometry.new(Some(0.62), Some(0.0)))),
    #("Torus", buffer_geometry.upcast(torus_geometry.new(Some(0.5), Some(0.18), Some(14.0), Some(36.0), None, None, None))),
    #("Icosahedron", buffer_geometry.upcast(icosahedron_geometry.new(Some(0.64), Some(0.0)))),
    #("Tetrahedron", buffer_geometry.upcast(tetrahedron_geometry.new(Some(0.72), Some(0.0)))),
  ]

  let specs =
    list.index_map(defs, fn(item, i) {
      let #(name, geo) = item
      let gx = int.to_float(i % 3 - 1)
      let gz = int.to_float(i / 3 - 1)
      let mat =
        mesh_standard_material_parameters.create()
        |> mesh_standard_material_parameters.with_color(t.ColorRepresentationNumber(hex(0xd4d4d4)))
        |> mesh_standard_material_parameters.with_roughness(0.55)
        |> mesh_standard_material_parameters.with_metalness(0.1)
        |> mesh_standard_material_parameters.with_flat_shading(i >= 4)
        |> Some
        |> mesh_standard_material.new
      let m = mesh.new(Some(geo), Some(mat), t.mesh_standard_material_codec())
      object3d.set_name(m, name)
      let _ = vector3.set(object3d.get_position(m), gx *. 2.1, 0.0, Some(gz *. 2.1))
      euler.set_x(object3d.get_rotation(m), 0.2)
      euler.set_y(object3d.get_rotation(m), 0.35 *. int.to_float(i))
      let _ = object3d.add(sc, [m])
      Specimen(name, mat, m)
    })

  let mesh_objs = list.map(specs, fn(s) { s.obj })
  let bx = box3.new(Some(vector3.new(Some(-0.5), Some(-0.5), Some(-0.5))), Some(vector3.new(Some(0.5), Some(0.5), Some(0.5))))
  let helper = box3_helper.new(bx, Some(t.ColorRepresentationNumber(hex(0x111111))))
  object3d.set_visible(helper, False)
  let _ = object3d.add(sc, [helper])

  let caster = raycaster.new(None, None, None, None)
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let yaw_ref = rt.ref_new(0.45)
  let sel_ref = rt.ref_new(-1)
  let hit_ref = rt.ref_new("none")

  let update_camera = fn() {
    let yaw = rt.ref_get(yaw_ref)
    let r = 8.4
    let _ = vector3.set(object3d.get_position(cam), r *. sin(yaw), 5.1, Some(r *. cos(yaw)))
    object3d.look_at(cam, vector3.new(Some(0.0), Some(0.0), Some(0.0)))
    object3d.update_matrix_world(cam, Some(True))
    object3d.update_matrix_world(sc, Some(True))
  }

  let render_scene = fn() {
    let sel = rt.ref_get(sel_ref)
    list.index_fold(specs, Nil, fn(_, s, i) {
      let col_hex = case i == sel { True -> hex(0x111111) False -> hex(0xd4d4d4) }
      let _ = color.set_hex(mesh_standard_material.get_color(s.mat), col_hex, None)
      Nil
    })
    case sel >= 0 && sel < 9 {
      True -> {
        let target_sp = nth_specimen(specs, sel)
        let _ = box3.set_from_object(bx, target_sp.obj, Some(True))
        object3d.set_visible(helper, True)
      }
      False -> object3d.set_visible(helper, False)
    }
    web_gl_renderer.render(renderer, sc, cam)
    dom_node.set_text_content(meta_el, Some("picked: " <> rt.ref_get(hit_ref) <> " · 9 specimens · click canvas or buttons"))
  }

  let cast_at_ndc = fn(nx: Float, ny: Float) {
    raycaster.set_from_camera(caster, vector2.new(Some(nx), Some(ny)), cam)
    case raycaster.intersect_objects(caster, mesh_objs, Some(False), None) {
      [first, ..] -> {
        let hit_obj = intersection.get_object(first)
        case find_by_id(specs, object3d.get_id(hit_obj), 0) {
          Some(#(idx, sp)) -> {
            rt.ref_set(sel_ref, idx)
            let _ = box3.set_from_object(bx, sp.obj, Some(True))
            let sz = box3.get_size(bx, vector3.new(None, None, None))
            let pt = intersection.get_point(first)
            rt.ref_set(
              hit_ref,
              sp.name
              <> " at d="
              <> fmt2(intersection.get_distance(first))
              <> " "
              <> fmt_v3(pt)
              <> " box="
              <> fmt2(vector3.get_x(sz))
              <> "x"
              <> fmt2(vector3.get_y(sz))
              <> "x"
              <> fmt2(vector3.get_z(sz)),
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
        rt.ref_set(hit_ref, "miss at ndc (" <> fmt2(nx) <> ", " <> fmt2(ny) <> ")")
      }
    }
    render_scene()
  }

  let pick_specimen = fn(idx: Int) {
    update_camera()
    let sp = nth_specimen(specs, idx)
    let wp = object3d.get_world_position(sp.obj, vector3.new(None, None, None))
    let proj = vector3.project(wp, cam)
    cast_at_ndc(vector3.get_x(proj), vector3.get_y(proj))
  }

  html_element.add_event_listener(canvas, ev.click,
    fn(e) {
      let rect = element.get_bounding_client_rect(canvas)
      let nx = { { mouse_event.get_client_x(e) -. dom_rect_read_only.get_left(rect) } /. dom_rect_read_only.get_width(rect) } *. 2.0 -. 1.0
      let ny = 0.0 -. { { mouse_event.get_client_y(e) -. dom_rect_read_only.get_top(rect) } /. dom_rect_read_only.get_height(rect) } *. 2.0 +. 1.0
      update_camera()
      cast_at_ndc(nx, ny)
    },
    None,
  )

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() }, None)
  }

  wire_click("btn-p0", fn() { pick_specimen(4) })
  wire_click("btn-p1", fn() { pick_specimen(0) })
  wire_click("btn-p2", fn() { pick_specimen(8) })
  wire_click("btn-orbit", fn() {
    let _ = rt.ref_update(yaw_ref, fn(y) { y +. pi /. 4.0 })
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
