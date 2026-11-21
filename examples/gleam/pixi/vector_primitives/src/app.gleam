import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/option.{None, Some}
import pixi_js/application
import pixi_js/application_init_options
import pixi_js/container
import pixi_js/graphics
import pixi_js/runtime
import pixi_js/types as t

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
      let _ = dom_node.append_child(stage_el, application.get_canvas(app))
      Nil
    }
    None -> Nil
  }

  let stage = application.get_stage(app)

  let grid = graphics.new(None)
  int.range(from: 1, to: 17, with: Nil, run: fn(_, i) {
    let x = int.to_float(i * 40)
    let _ = graphics.move_to(grid, x, 0.0) |> graphics.line_to(x, 400.0)
    Nil
  })
  int.range(from: 1, to: 10, with: Nil, run: fn(_, j) {
    let y = int.to_float(j * 40)
    let _ = graphics.move_to(grid, 0.0, y) |> graphics.line_to(680.0, y)
    Nil
  })
  let _ = stroke_g(grid, 1.0, 0xe5e5e5)
  let _ = container.add_child(stage, [grid])

  let rect_g =
    graphics.new(None)
    |> graphics.rect(40.0, 46.0, 140.0, 88.0)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0x111111))))
  let _ = container.add_child(stage, [rect_g])

  let round_g =
    graphics.new(None)
    |> graphics.round_rect(216.0, 46.0, 156.0, 88.0, Some(16.0))
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0xf7f7f7))))
    |> stroke_g(2.0, 0x111111)
  let _ = container.add_child(stage, [round_g])

  let circle_g =
    graphics.new(None)
    |> graphics.circle(458.0, 90.0, 44.0)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0x404040))))
    |> graphics.circle(458.0, 90.0, 22.0)
    |> stroke_g(2.0, 0xffffff)
    |> graphics.circle(458.0, 90.0, 52.0)
    |> stroke_g(1.5, 0x111111)
  let _ = container.add_child(stage, [circle_g])

  let ellipse_g =
    graphics.new(None)
    |> graphics.ellipse(588.0, 90.0, 56.0, 36.0)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0xd4d4d4))))
    |> stroke_g(1.5, 0x111111)
  let _ = container.add_child(stage, [ellipse_g])

  let poly_g =
    graphics.new(None)
    |> graphics.regular_poly(130.0, 274.0, 58.0, 6.0, Some(0.25), None)
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0x737373))))
    |> stroke_g(2.0, 0x111111)
  let _ = container.add_child(stage, [poly_g])

  let star_g =
    graphics.new(None)
    |> graphics.star(340.0, 274.0, 5.0, 62.0, Some(26.0), Some(-0.314))
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0x111111))))
  let _ = container.add_child(stage, [star_g])

  let diamond_g =
    graphics.new(None)
    |> graphics.move_to(548.0, 212.0)
    |> graphics.line_to(618.0, 274.0)
    |> graphics.line_to(548.0, 336.0)
    |> graphics.line_to(478.0, 274.0)
    |> graphics.close_path()
    |> graphics.fill(Some(runtime.dynamic(int.to_float(0xb5b5b5))))
    |> stroke_g(1.5, 0x111111)
  let _ = container.add_child(stage, [diamond_g])

  application.render(app)

  case document.get_element_by_id(doc, "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "7 vector primitives · rect · roundRect · circle · ellipse · regularPoly · star · path · 680x400 WebGL",
        ),
      )
    None -> Nil
  }
}
