import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import dom/areas/html/html_element
import dom/areas/html/html_element_event_map as ev
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import gleam/result
import three/ambient_light
import three/box_geometry
import three/buffer_attribute
import three/color
import three/directional_light
import three/euler
import three/grid_helper
import three/instanced_mesh
import three/matrix4
import three/mesh
import three/mesh_standard_material
import three/mesh_standard_material_parameters
import three/object3d
import three/orthographic_camera
import three/perspective_camera
import three/plane_geometry
import three/quaternion
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

const pi = 3.141592653589793

fn hex(value: Int) -> Float {
  int.to_float(value)
}

fn tower_height(mode: Int, gx: Int, gz: Int) -> Float {
  let fx = int.to_float(gx) -. 4.5
  let fz = int.to_float(gz) -. 4.5
  case mode {
    1 -> {
      let r = float.square_root(fx *. fx +. fz *. fz) |> result.unwrap(0.0)
      0.35 +. 0.42 *. r +. 0.25 *. cos(fx *. 0.8)
    }
    2 -> {
      let diag = float.absolute_value(fx -. fz)
      0.3
      +. 2.8
      *. float.exponential(0.0 -. diag *. diag /. 5.5)
      +. 0.25
      *. sin(fz *. 0.9)
    }
    _ -> {
      let r2 = fx *. fx +. fz *. fz
      0.35
      +. 3.1
      *. float.exponential(0.0 -. r2 /. 9.5)
      +. 0.2
      *. cos(fx *. 1.4)
      *. cos(fz *. 1.4)
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
  inst: t.InstancedMesh(
    t.BoxGeometry(Nil),
    t.MeshStandardMaterial(Nil),
    t.Object3DEventMap(Nil),
    Nil,
  ),
  mode: Int,
  pos: t.Vector3(Nil),
  rot_q: t.Quaternion(Nil),
  scale: t.Vector3(Nil),
  mtx: t.Matrix4(Nil),
  axis_y: t.Vector3(Nil),
  col: t.Color(Nil),
) -> Float {
  int.range(from: 0, to: 100, with: 0.0, run: fn(max_h, idx) {
    let gz = idx / 10
    let gx = idx % 10
    let x = { int.to_float(gx) -. 4.5 } *. 0.78
    let z = { int.to_float(gz) -. 4.5 } *. 0.78
    let h = tower_height(mode, gx, gz)
    vector3.set_with(pos, x, h *. 0.5, Some(z))
    quaternion.set_from_axis_angle(
      rot_q,
      axis_y,
      0.08 *. int.to_float({ gx + gz } % 4),
    )
    vector3.set_with(scale, 1.0, h, Some(1.0))
    matrix4.compose(mtx, pos, rot_q, scale)
    instanced_mesh.set_matrix_at(inst, int.to_float(idx), mtx)

    let clamped_t = float.clamp({ h -. 0.3 } /. 3.2, min: 0.0, max: 1.0)
    let shade = 0.82 -. 0.7 *. clamped_t
    color.set_rgb(col, shade, shade, shade)
    instanced_mesh.set_color_at(inst, int.to_float(idx), col)
    float.max(max_h, h)
  })
}

pub fn main() {
  let doc = core.get_document()
  let width = 680.0
  let height = 400.0
  let aspect = width /. height

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
  let ortho_cam =
    orthographic_camera.new_with(
      Some(-5.4 *. aspect),
      Some(5.4 *. aspect),
      Some(5.4),
      Some(-5.4),
      Some(0.1),
      Some(100.0),
    )
  let persp_cam =
    perspective_camera.new_with(
      Some(40.0),
      Some(aspect),
      Some(0.1),
      Some(100.0),
    )

  object3d.add(sc, [
    ambient_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(1.25),
    ),
  ])
  let key =
    directional_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(2.1),
    )
  object3d.get_position(key)
  |> vector3.set_with(8.0, 12.0, Some(6.0))
  object3d.add(sc, [key])
  let rim =
    directional_light.new_with(
      Some(t.ColorRepresentationNumber(hex(0xffffff))),
      Some(0.9),
    )
  object3d.get_position(rim)
  |> vector3.set_with(-7.0, 6.0, Some(-8.0))
  object3d.add(sc, [rim])

  let pmp =
    mesh_standard_material_parameters.create()
    |> mesh_standard_material_parameters.with_color(
      t.ColorRepresentationNumber(hex(0xe5e5e5)),
    )
    |> mesh_standard_material_parameters.with_roughness(0.85)
  let plinth =
    mesh.new(
      Some(plane_geometry.new_with(Some(8.6), Some(8.6), None, None)),
      Some(mesh_standard_material.new_with(Some(pmp))),
      t.mesh_standard_material_codec(),
    )
  euler.set_x(object3d.get_rotation(plinth), 0.0 -. pi /. 2.0)
  vector3.set_y(object3d.get_position(plinth), -0.01)
  object3d.add(sc, [plinth])

  let grid =
    grid_helper.new_with(
      Some(9.0),
      Some(10.0),
      Some(t.ColorRepresentationNumber(hex(0x999999))),
      Some(t.ColorRepresentationNumber(hex(0xd4d4d4))),
    )
  object3d.add(sc, [grid])

  let tgeo =
    box_geometry.new_with(Some(0.54), Some(1.0), Some(0.54), None, None, None)
  let tmp =
    mesh_standard_material_parameters.create()
    |> mesh_standard_material_parameters.with_color(
      t.ColorRepresentationNumber(hex(0xffffff)),
    )
    |> mesh_standard_material_parameters.with_roughness(0.65)
    |> mesh_standard_material_parameters.with_metalness(0.08)
  let tmat = mesh_standard_material.new_with(Some(tmp))
  let inst =
    instanced_mesh.new(
      Some(tgeo),
      Some(tmat),
      100.0,
      t.mesh_standard_material_codec(),
    )
  object3d.add(sc, [inst])

  let assert Some(cam_btn) = document.get_element_by_id(doc, "btn-cam")
  let assert Some(meta_el) = document.get_element_by_id(doc, "meta")
  let ortho_ref = rt.ref_new(True)
  let mode_ref = rt.ref_new(0)
  let yaw_ref = rt.ref_new(0.785398)

  let pos = vector3.new()
  let rot_q = quaternion.new()
  let scale = vector3.new()
  let mtx = matrix4.new()
  let axis_y = vector3.new_with(Some(0.0), Some(1.0), Some(0.0))
  let col = color.new()

  let render_city = fn() {
    let mode = rt.ref_get(mode_ref)
    let use_ortho = rt.ref_get(ortho_ref)
    let yaw = rt.ref_get(yaw_ref)
    let max_h = update_towers(inst, mode, pos, rot_q, scale, mtx, axis_y, col)
    buffer_attribute.set_needs_update(
      instanced_mesh.get_instance_matrix(inst),
      True,
    )
    case instanced_mesh.get_instance_color(inst) {
      Some(ic) -> buffer_attribute.set_needs_update(ic, True)
      None -> Nil
    }

    let radius = 11.5
    let cx = radius *. cos(yaw)
    let cz = radius *. sin(yaw)
    let target = vector3.new_with(Some(0.0), Some(0.9), Some(0.0))
    object3d.get_position(ortho_cam)
    |> vector3.set_with(cx, 8.2, Some(cz))
    object3d.look_at(ortho_cam, target)
    object3d.get_position(persp_cam)
    |> vector3.set_with(cx, 7.6, Some(cz))
    object3d.look_at(persp_cam, target)

    case use_ortho {
      True -> web_gl_renderer.render(renderer, sc, ortho_cam)
      False -> web_gl_renderer.render(renderer, sc, persp_cam)
    }

    let cname = case use_ortho {
      True -> "Orthographic"
      False -> "Perspective"
    }
    dom_node.set_text_content(cam_btn, Some("Camera: " <> cname))
    dom_node.set_text_content(
      meta_el,
      Some(
        "100 InstancedMesh towers · layout: "
        <> layout_name(mode)
        <> " · peak height: "
        <> rt.to_fixed(max_h, 2)
        <> " · camera: "
        <> cname,
      ),
    )
  }

  let wire_click = fn(id: String, cb: fn() -> Nil) {
    let assert Some(b) = document.get_element_by_id(doc, id)
    html_element.add_event_listener(b, ev.click, fn(_) { cb() })
  }

  wire_click("btn-cam", fn() {
    rt.ref_update(ortho_ref, fn(o) { !o })
    render_city()
  })
  wire_click("btn-lay0", fn() {
    rt.ref_set(mode_ref, 0)
    render_city()
  })
  wire_click("btn-lay1", fn() {
    rt.ref_set(mode_ref, 1)
    render_city()
  })
  wire_click("btn-lay2", fn() {
    rt.ref_set(mode_ref, 2)
    render_city()
  })
  wire_click("btn-rot", fn() {
    rt.ref_update(yaw_ref, fn(y) { y +. pi /. 4.0 })
    render_city()
  })

  render_city()
}
