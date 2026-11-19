import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pixi_js/areas/main/application
import pixi_js/areas/main/application_init_options
import pixi_js/areas/main/container
import pixi_js/areas/main/graphics
import pixi_js/runtime
import pixi_js/types as t

const two_pi_3 = 2.0943951023931953

fn stroke_g(g: t.Graphics(Nil), width: Float, color_hex: Int) -> t.Graphics(Nil) {
  graphics.stroke(g, [
    runtime.js_object([
      #("width", runtime.dynamic(width)),
      #("color", runtime.dynamic(int.to_float(color_hex))),
    ]),
  ])
}

pub fn main() {
  let app = application.new()
  let opts = application_init_options.create()
  application_init_options.set_width(opts, Some(680.0))
  application_init_options.set_height(opts, Some(400.0))
  application_init_options.set_background_color(
    opts,
    Some(runtime.dynamic(int.to_float(0xffffff))),
  )
  application_init_options.set_preference(
    opts,
    Some(t.StringOrRendererPreferenceArrayString("webgl")),
  )
  application_init_options.set_preserve_drawing_buffer(opts, Some(True))
  application_init_options.set_antialias(opts, Some(True))
  application_init_options.set_auto_start(opts, Some(False))

  runtime.promise_then(application.init(app, Some(opts)), fn(ready) {
    let assert Ok(_) = ready
    let doc = core.get_document()
    case document.get_element_by_id(doc, "stage") {
      Some(stage_el) -> {
        let _ = dom_node.append_child(stage_el, application.get_canvas(app))
        Nil
      }
      None -> Nil
    }

    let stage = application.get_stage(app)

    let guide = graphics.new(None)
    let _ = graphics.move_to(guide, 40.0, 200.0)
    let _ = graphics.line_to(guide, 640.0, 200.0)
    let _ = graphics.move_to(guide, 340.0, 24.0)
    let _ = graphics.line_to(guide, 340.0, 376.0)
    let _ = stroke_g(guide, 1.0, 0xe5e5e5)
    let _ = graphics.circle(guide, 340.0, 200.0, 110.0)
    let _ = stroke_g(guide, 1.0, 0xd4d4d4)
    let _ = container.add_child(stage, [guide], runtime.codec_list(t.graphics_codec()))

    let root = container.new(None)
    container.set_x(root, 340.0)
    container.set_y(root, 200.0)

    list.each([0, 1, 2], fn(i) {
      let arm = container.new(None)
      container.set_rotation(arm, int.to_float(i) *. two_pi_3)

      let arm_bar = graphics.new(None)
      let _ = graphics.rect(arm_bar, 0.0, -4.0, 110.0, 8.0)
      let _ = graphics.fill(arm_bar, Some(runtime.dynamic(int.to_float(0x737373))))
      let _ = graphics.circle(arm_bar, 110.0, 0.0, 10.0)
      let _ = graphics.fill(arm_bar, Some(runtime.dynamic(int.to_float(0x111111))))
      let _ = container.add_child(arm, [arm_bar], runtime.codec_list(t.graphics_codec()))

      let sub = container.new(None)
      container.set_x(sub, 110.0)
      container.set_rotation(sub, -0.45)

      let sub_bar = graphics.new(None)
      let _ = graphics.move_to(sub_bar, -32.0, 0.0)
      let _ = graphics.line_to(sub_bar, 48.0, 0.0)
      let _ = stroke_g(sub_bar, 2.0, 0x404040)

      let sat_box = graphics.new(None)
      let _ = graphics.rect(sat_box, -12.0, -12.0, 24.0, 24.0)
      let _ = graphics.fill(sat_box, Some(runtime.dynamic(int.to_float(0x262626))))
      container.set_x(sat_box, 48.0)

      let sat_orb = graphics.new(None)
      let _ = graphics.circle(sat_orb, 0.0, 0.0, 14.0)
      let _ = graphics.fill(sat_orb, Some(runtime.dynamic(int.to_float(0xd4d4d4))))
      let _ = stroke_g(sat_orb, 2.0, 0x111111)
      container.set_x(sat_orb, -32.0)

      let _ =
        container.add_child(
          sub,
          [sub_bar, sat_box, sat_orb],
          runtime.codec_list(t.graphics_codec()),
        )
      let _ = container.add_child(arm, [sub], runtime.codec_list(t.container_codec()))
      let _ = container.add_child(root, [arm], runtime.codec_list(t.container_codec()))
      Nil
    })

    let hub = graphics.new(None)
    let _ = graphics.circle(hub, 0.0, 0.0, 18.0)
    let _ = graphics.fill(hub, Some(runtime.dynamic(int.to_float(0x111111))))
    let _ = graphics.circle(hub, 0.0, 0.0, 7.0)
    let _ = graphics.fill(hub, Some(runtime.dynamic(int.to_float(0xffffff))))
    let _ = container.add_child(root, [hub], runtime.codec_list(t.graphics_codec()))

    let _ = container.add_child(stage, [root], runtime.codec_list(t.container_codec()))
    application.render(app)

    let w_str = float.to_string(int.to_float(float.round(container.get_width(root))))
    let h_str = float.to_string(int.to_float(float.round(container.get_height(root))))
    case document.get_element_by_id(doc, "meta") {
      Some(meta) ->
        dom_node.set_text_content(
          meta,
          Some(
            "3 primary arms · 6 nested satellites · bounds "
            <> w_str
            <> "x"
            <> h_str
            <> " · 680x400 WebGL",
          ),
        )
      None -> Nil
    }
  })
}
