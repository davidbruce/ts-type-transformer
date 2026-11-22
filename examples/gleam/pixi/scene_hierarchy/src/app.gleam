import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/option.{None, Some}
import pixi_js/application
import pixi_js/application_init_options
import pixi_js/container
import pixi_js/graphics
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
  let opts =
    application_init_options.create()
    |> application_init_options.with_width(680.0)
    |> application_init_options.with_height(400.0)
    |> application_init_options.with_background_color(
      runtime.dynamic(int.to_float(0xffffff)),
    )
    |> application_init_options.with_preference(
      t.StringOrRendererPreferenceArrayString("webgl"),
    )
    |> application_init_options.with_preserve_drawing_buffer(True)
    |> application_init_options.with_antialias(True)
    |> application_init_options.with_auto_start(False)

  use ready <- runtime.promise_then(application.init(app, Some(opts)))
  let assert Ok(_) = ready
  let doc = core.get_document()
  case document.get_element_by_id(doc, "stage") {
    Some(stage_el) -> {
      dom_node.append_child(stage_el, application.get_canvas(app))
      Nil
    }
    None -> Nil
  }

  let stage = application.get_stage(app)

  let guide =
    graphics.new(None)
    |> graphics.move_to(40.0, 200.0)
    |> graphics.line_to(640.0, 200.0)
    |> graphics.move_to(340.0, 24.0)
    |> graphics.line_to(340.0, 376.0)
    |> stroke_g(1.0, 0xe5e5e5)
    |> graphics.circle(340.0, 200.0, 110.0)
    |> stroke_g(1.0, 0xd4d4d4)
  container.add_child(stage, [guide])

  let root = container.new(None)
  container.set_x(root, 340.0)
  container.set_y(root, 200.0)

  int.range(from: 0, to: 3, with: Nil, run: fn(_, i) {
    let arm = container.new(None)
    container.set_rotation(arm, int.to_float(i) *. two_pi_3)

    let arm_bar =
      graphics.new(None)
      |> graphics.rect(0.0, -4.0, 110.0, 8.0)
      |> graphics.fill(Some(runtime.dynamic(int.to_float(0x737373))))
      |> graphics.circle(110.0, 0.0, 10.0)
      |> graphics.fill(Some(runtime.dynamic(int.to_float(0x111111))))
    container.add_child(arm, [arm_bar])

    let sub = container.new(None)
    container.set_x(sub, 110.0)
    container.set_rotation(sub, -0.45)

    let sub_bar =
      graphics.new(None)
      |> graphics.move_to(-32.0, 0.0)
      |> graphics.line_to(48.0, 0.0)
      |> stroke_g(2.0, 0x404040)

    let sat_box =
      graphics.new(None)
      |> graphics.rect(-12.0, -12.0, 24.0, 24.0)
      |> graphics.fill(Some(runtime.dynamic(int.to_float(0x262626))))
    container.set_x(sat_box, 48.0)

    let sat_orb =
      graphics.new(None)
      |> graphics.circle(0.0, 0.0, 14.0)
      |> graphics.fill(Some(runtime.dynamic(int.to_float(0xd4d4d4))))
      |> stroke_g(2.0, 0x111111)
    container.set_x(sat_orb, -32.0)

    container.add_child(sub, [sub_bar, sat_box, sat_orb])
    container.add_child(arm, [sub])
    container.add_child(root, [arm])
    Nil
  })

  let hub =
    graphics.new(None)
    |> graphics.circle(0.0, 0.0, 18.0)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0x111111))))
    |> graphics.circle(0.0, 0.0, 7.0)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0xffffff))))
  container.add_child(root, [hub])

  container.add_child(stage, [root])
  application.render(app)

  let w_str = int.to_string(float.round(container.get_width(root)))
  let h_str = int.to_string(float.round(container.get_height(root)))
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
}
