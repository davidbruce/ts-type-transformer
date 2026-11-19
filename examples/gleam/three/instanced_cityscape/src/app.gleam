import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import gleam/int
import gleam/option.{None, Some}
import three/areas/main/ambient_light
import three/areas/main/box_geometry
import three/areas/main/buffer_attribute
import three/areas/main/color
import three/areas/main/directional_light
import three/areas/main/euler
import three/areas/main/grid_helper
import three/areas/main/instanced_mesh
import three/areas/main/matrix4
import three/areas/main/mesh
import three/areas/main/mesh_standard_material
import three/areas/main/mesh_standard_material_parameters
import three/areas/main/object3d
import three/areas/main/orthographic_camera
import three/areas/main/perspective_camera
import three/areas/main/plane_geometry
import three/areas/main/quaternion
import three/areas/main/scene
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

@external(javascript, "./app_ffi.mjs", "sqrt")
fn sqrt(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "exp")
fn exp(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "abs")
fn abs(x: Float) -> Float

@external(javascript, "./app_ffi.mjs", "fmt2")
fn fmt2(x: Float) -> String

fn tower_height(mode: Int, gx: Int, gz: Int) -> Float {
  let fx = int.to_float(gx) -. 4.5
  let fz = int.to_float(gz) -. 4.5
  case mode {
    1 -> {
      let r = sqrt(fx *. fx +. fz *. fz)
      0.35 +. 0.42 *. r +. 0.25 *. cos(fx *. 0.8)
    }
    2 -> {
      let diag = abs(fx -. fz)
      0.3 +. 2.8 *. exp(0.0 -. diag *. diag /. 5.5) +. 0.25 *. sin(fz *. 0.9)
    }
    _ -> {
      let r2 = fx *. fx +. fz *. fz
      0.35 +. 3.1 *. exp(0.0 -. r2 /. 9.5) +. 0.2 *. cos(fx *. 1.4) *. cos(fz *. 1.4)
    }
  }
}

fn layout_name(mode: Int) -> String {
  case mode {
    1 -> "Amphitheater"
    2 -> "Diagonal"
    _ -> "Plaza"
  }
}

fn update_towers(
  inst: t.InstancedMesh(t.BoxGeometry(Nil), t.MeshStandardMaterial(Nil), t.Object3DEventMap(Nil), Nil),
  mode: Int,
  gz: Int,
  gx: Int,
  idx: Int,
  max_h: Float,
  pos: t.Vector3(Nil),
  rot_q: t.Quaternion(Nil),
  scale: t.Vector3(Nil),
  mtx: t.Matrix4(Nil),
  axis_y: t.Vector3(Nil),
  col: t.Color(Nil),
) -> Float {
  case gz >= 10 {
    True -> max_h
    False ->
      case gx >= 10 {
        True -> update_towers(inst, mode, gz + 1, 0, idx, max_h, pos, rot_q, scale, mtx, axis_y, col)
        False -> {
          let x = { int.to_float(gx) -. 4.5 } *. 0.78
          let z = { int.to_float(gz) -. 4.5 } *. 0.78
          let h = tower_height(mode, gx, gz)
          let next_max = case h >. max_h {
            True -> h
            False -> max_h
          }
          let _ = vector3.set(pos, x, h *. 0.5, Some(z))
          let _ = quaternion.set_from_axis_angle(rot_q, axis_y, 0.08 *. int.to_float({ gx + gz } % 4))
          let _ = vector3.set(scale, 1.0, h, Some(1.0))
          let _ = matrix4.compose(mtx, pos, rot_q, scale)
          let _ = instanced_mesh.set_matrix_at(inst, int.to_float(idx), mtx)

          let raw_t = { h -. 0.3 } /. 3.2
          let clamped_t = case raw_t <. 0.0 {
            True -> 0.0
            False -> case raw_t >. 1.0 { True -> 1.0 False -> raw_t }
          }
          let shade = 0.82 -. 0.70 *. clamped_t
          let _ = color.set_rgb(col, shade, shade, shade, None)
          let _ = instanced_mesh.set_color_at(inst, int.to_float(idx), col)
          update_towers(inst, mode, gz, gx + 1, idx + 1, next_max, pos, rot_q, scale, mtx, axis_y, col)
        }
      }
  }
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0
  let aspect = width /. height
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
  let ortho_cam = orthographic_camera.new(Some(-5.4 *. aspect), Some(5.4 *. aspect), Some(5.4), Some(-5.4), Some(0.1), Some(100.0))
  let persp_cam = perspective_camera.new(Some(40.0), Some(aspect), Some(0.1), Some(100.0))

  let _ = object3d.add(sc, [ambient_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(1.25))])
  let key = directional_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(2.1))
  let _ = vector3.set(object3d.get_position(key), 8.0, 12.0, Some(6.0))
  let _ = object3d.add(sc, [key])
  let rim = directional_light.new(Some(t.ColorRepresentationNumber(16_777_215.0)), Some(0.9))
  let _ = vector3.set(object3d.get_position(rim), -7.0, 6.0, Some(-8.0))
  let _ = object3d.add(sc, [rim])

  let pmp = mesh_standard_material_parameters.create()
  mesh_standard_material_parameters.set_color(pmp, Some(t.ColorRepresentationNumber(15_066_597.0)))
  mesh_standard_material_parameters.set_roughness(pmp, Some(0.85))
  let plinth =
    mesh.new(
      Some(plane_geometry.new(Some(8.6), Some(8.6), None, None)),
      Some(mesh_standard_material.new(Some(pmp))),
      t.plane_geometry_codec(),
      t.mesh_standard_material_codec(),
    )
  euler.set_x(object3d.get_rotation(plinth), 0.0 -. pi /. 2.0)
  vector3.set_y(object3d.get_position(plinth), -0.01)
  let _ = object3d.add(sc, [plinth])

  let grid =
    grid_helper.new(
      Some(9.0),
      Some(10.0),
      Some(t.ColorRepresentationNumber(10_066_329.0)),
      Some(t.ColorRepresentationNumber(13_948_116.0)),
    )
  let _ = object3d.add(sc, [grid])

  let tgeo = box_geometry.new(Some(0.54), Some(1.0), Some(0.54), None, None, None)
  let tmp = mesh_standard_material_parameters.create()
  mesh_standard_material_parameters.set_color(tmp, Some(t.ColorRepresentationNumber(16_777_215.0)))
  mesh_standard_material_parameters.set_roughness(tmp, Some(0.65))
  mesh_standard_material_parameters.set_metalness(tmp, Some(0.08))
  let tmat = mesh_standard_material.new(Some(tmp))
  let inst = instanced_mesh.new(Some(tgeo), Some(tmat), 100.0, t.box_geometry_codec(), t.mesh_standard_material_codec())
  let _ = object3d.add(sc, [inst])

  let assert Some(cam_btn) = document.get_element_by_id(doc, "btn-cam")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let ortho_cell = new_cell(True)
  let mode_cell = new_cell(0)
  let yaw_cell = new_cell(0.785398)

  let pos = vector3.new(None, None, None)
  let rot_q = quaternion.new(None, None, None, None)
  let scale = vector3.new(None, None, None)
  let mtx = matrix4.new()
  let axis_y = vector3.new(Some(0.0), Some(1.0), Some(0.0))
  let col = color.new(None)

  let render_city = fn() {
    let mode = get_cell(mode_cell)
    let use_ortho = get_cell(ortho_cell)
    let yaw = get_cell(yaw_cell)
    let max_h = update_towers(inst, mode, 0, 0, 0, 0.0, pos, rot_q, scale, mtx, axis_y, col)
    buffer_attribute.set_needs_update(instanced_mesh.get_instance_matrix(inst), True)
    case instanced_mesh.get_instance_color(inst) {
      Some(ic) -> buffer_attribute.set_needs_update(ic, True)
      None -> Nil
    }

    let radius = 11.5
    let cx = radius *. cos(yaw)
    let cz = radius *. sin(yaw)
    let target = vector3.new(Some(0.0), Some(0.9), Some(0.0))
    let _ = vector3.set(object3d.get_position(ortho_cam), cx, 8.2, Some(cz))
    object3d.look_at(ortho_cam, target)
    let _ = vector3.set(object3d.get_position(persp_cam), cx, 7.6, Some(cz))
    object3d.look_at(persp_cam, target)

    case use_ortho {
      True -> web_gl_renderer.render(renderer, sc, ortho_cam)
      False -> web_gl_renderer.render(renderer, sc, persp_cam)
    }

    let cname = case use_ortho { True -> "Orthographic" False -> "Perspective" }
    dom_node.set_text_content(cam_btn, Some("Camera: " <> cname))
    dom_node.set_text_content(
      meta_el,
      Some("100 InstancedMesh towers · layout: " <> layout_name(mode) <> " · peak height: " <> fmt2(max_h) <> " · camera: " <> cname),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener_click(coerce(b), fn(_) { cb() }, None)
  }

  wire_click("btn-cam", fn() {
    set_cell(ortho_cell, !get_cell(ortho_cell))
    render_city()
  })
  wire_click("btn-lay0", fn() {
    set_cell(mode_cell, 0)
    render_city()
  })
  wire_click("btn-lay1", fn() {
    set_cell(mode_cell, 1)
    render_city()
  })
  wire_click("btn-lay2", fn() {
    set_cell(mode_cell, 2)
    render_city()
  })
  wire_click("btn-rot", fn() {
    set_cell(yaw_cell, get_cell(yaw_cell) +. pi /. 4.0)
    render_city()
  })

  render_city()
}
