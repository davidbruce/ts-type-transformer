import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pixi_js/areas/main/application
import pixi_js/areas/main/application_init_options
import pixi_js/areas/main/container
import pixi_js/areas/main/graphics
import pixi_js/runtime
import pixi_js/types as t

fn range(from: Int, to: Int) -> List(Int) {
  case from > to {
    True -> []
    False -> [from, ..range(from + 1, to)]
  }
}

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

    let grid = graphics.new(None)
    list.each(range(1, 16), fn(i) {
      let x = int.to_float(i * 40)
      let _ = graphics.move_to(grid, x, 0.0)
      let _ = graphics.line_to(grid, x, 400.0)
      Nil
    })
    list.each(range(1, 9), fn(j) {
      let y = int.to_float(j * 40)
      let _ = graphics.move_to(grid, 0.0, y)
      let _ = graphics.line_to(grid, 680.0, y)
      Nil
    })
    let _ = stroke_g(grid, 1.0, 0xe5e5e5)
    let _ = container.add_child(stage, [grid], runtime.codec_list(t.graphics_codec()))

    let rect_g = graphics.new(None)
    let _ = graphics.rect(rect_g, 40.0, 46.0, 140.0, 88.0)
    let _ = graphics.fill(rect_g, Some(runtime.dynamic(int.to_float(0x111111))))
    let _ = container.add_child(stage, [rect_g], runtime.codec_list(t.graphics_codec()))

    let round_g = graphics.new(None)
    let _ = graphics.round_rect(round_g, 216.0, 46.0, 156.0, 88.0, Some(16.0))
    let _ = graphics.fill(round_g, Some(runtime.dynamic(int.to_float(0xf7f7f7))))
    let _ = stroke_g(round_g, 2.0, 0x111111)
    let _ = container.add_child(stage, [round_g], runtime.codec_list(t.graphics_codec()))

    let circle_g = graphics.new(None)
    let _ = graphics.circle(circle_g, 458.0, 90.0, 44.0)
    let _ = graphics.fill(circle_g, Some(runtime.dynamic(int.to_float(0x404040))))
    let _ = graphics.circle(circle_g, 458.0, 90.0, 22.0)
    let _ = stroke_g(circle_g, 2.0, 0xffffff)
    let _ = graphics.circle(circle_g, 458.0, 90.0, 52.0)
    let _ = stroke_g(circle_g, 1.5, 0x111111)
    let _ = container.add_child(stage, [circle_g], runtime.codec_list(t.graphics_codec()))

    let ellipse_g = graphics.new(None)
    let _ = graphics.ellipse(ellipse_g, 588.0, 90.0, 56.0, 36.0)
    let _ = graphics.fill(ellipse_g, Some(runtime.dynamic(int.to_float(0xd4d4d4))))
    let _ = stroke_g(ellipse_g, 1.5, 0x111111)
    let _ = container.add_child(stage, [ellipse_g], runtime.codec_list(t.graphics_codec()))

    let poly_g = graphics.new(None)
    let _ = graphics.regular_poly(poly_g, 130.0, 274.0, 58.0, 6.0, Some(0.25), None)
    let _ = graphics.fill(poly_g, Some(runtime.dynamic(int.to_float(0x737373))))
    let _ = stroke_g(poly_g, 2.0, 0x111111)
    let _ = container.add_child(stage, [poly_g], runtime.codec_list(t.graphics_codec()))

    let star_g = graphics.new(None)
    let _ = graphics.star(star_g, 340.0, 274.0, 5.0, 62.0, Some(26.0), Some(-0.314))
    let _ = graphics.fill(star_g, Some(runtime.dynamic(int.to_float(0x111111))))
    let _ = container.add_child(stage, [star_g], runtime.codec_list(t.graphics_codec()))

    let diamond_g = graphics.new(None)
    let _ = graphics.move_to(diamond_g, 548.0, 212.0)
    let _ = graphics.line_to(diamond_g, 618.0, 274.0)
    let _ = graphics.line_to(diamond_g, 548.0, 336.0)
    let _ = graphics.line_to(diamond_g, 478.0, 274.0)
    let _ = graphics.close_path(diamond_g)
    let _ = graphics.fill(diamond_g, Some(runtime.dynamic(int.to_float(0xb5b5b5))))
    let _ = stroke_g(diamond_g, 1.5, 0x111111)
    let _ = container.add_child(stage, [diamond_g], runtime.codec_list(t.graphics_codec()))

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
  })
}
