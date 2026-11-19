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
import pixi_js/areas/main/matrix
import pixi_js/areas/main/point
import pixi_js/areas/main/point_data
import pixi_js/runtime
import pixi_js/types as t

const two_pi = 6.283185307179586

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

fn ring_spec(r_idx: Int) -> #(Float, Int) {
  case r_idx {
    0 -> #(42.0, 0x111111)
    1 -> #(82.0, 0x555555)
    2 -> #(122.0, 0xa3a3a3)
    _ -> #(162.0, 0xe5e5e5)
  }
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

    let guides = graphics.new(None)
    list.each([42.0, 82.0, 122.0, 162.0], fn(r) {
      let _ = graphics.circle(guides, 340.0, 200.0, r)
      let _ = stroke_g(guides, 1.0, 0xe5e5e5)
      Nil
    })

    let unit_x = point.new(Some(1.0), Some(0.0))
    list.each(range(0, 11), fn(step) {
      let theta = int.to_float(step) *. { two_pi /. 12.0 }
      let spoke = matrix.rotate(matrix.new(None, None, None, None, None, None), theta)
      let dir = matrix.apply(spoke, unit_x, None)
      let _ = graphics.move_to(guides, 340.0, 200.0)
      let _ =
        graphics.line_to(
          guides,
          340.0 +. point_data.get_x(dir) *. 182.0,
          200.0 +. point_data.get_y(dir) *. 182.0,
        )
      Nil
    })
    let _ = stroke_g(guides, 1.0, 0xd4d4d4)
    let _ = container.add_child(stage, [guides], runtime.codec_list(t.graphics_codec()))

    let tiles = graphics.new(None)
    list.each([0, 1, 2, 3], fn(r_idx) {
      let #(radius, fill_hex) = ring_spec(r_idx)
      let s = 8.0 +. int.to_float(r_idx) *. 2.5
      let hw = s *. 0.72
      list.each(range(0, 11), fn(step) {
        let theta =
          int.to_float(step) *. { two_pi /. 12.0 } +. int.to_float(r_idx) *. 0.13
        let dir_m = matrix.rotate(matrix.new(None, None, None, None, None, None), theta)
        let dir = matrix.apply(dir_m, unit_x, None)

        let tile_m =
          matrix.new(None, None, None, None, None, None)
          |> matrix.rotate(theta +. int.to_float(r_idx) *. 0.2)
          |> matrix.translate(
            340.0 +. radius *. point_data.get_x(dir),
            200.0 +. radius *. point_data.get_y(dir),
          )

        let p0 = matrix.apply(tile_m, point.new(Some(0.0), Some(0.0 -. s)), None)
        let p1 = matrix.apply(tile_m, point.new(Some(hw), Some(0.0)), None)
        let p2 = matrix.apply(tile_m, point.new(Some(0.0), Some(s)), None)
        let p3 = matrix.apply(tile_m, point.new(Some(0.0 -. hw), Some(0.0)), None)

        let _ = graphics.move_to(tiles, point_data.get_x(p0), point_data.get_y(p0))
        let _ = graphics.line_to(tiles, point_data.get_x(p1), point_data.get_y(p1))
        let _ = graphics.line_to(tiles, point_data.get_x(p2), point_data.get_y(p2))
        let _ = graphics.line_to(tiles, point_data.get_x(p3), point_data.get_y(p3))
        let _ = graphics.close_path(tiles)
        let _ = graphics.fill(tiles, Some(runtime.dynamic(int.to_float(fill_hex))))
        let _ = stroke_g(tiles, 1.2, 0x111111)
        Nil
      })
    })
    let _ = container.add_child(stage, [tiles], runtime.codec_list(t.graphics_codec()))

    let center_dot = graphics.new(None)
    let _ = graphics.circle(center_dot, 340.0, 200.0, 8.0)
    let _ = graphics.fill(center_dot, Some(runtime.dynamic(int.to_float(0x111111))))
    let _ = container.add_child(stage, [center_dot], runtime.codec_list(t.graphics_codec()))

    application.render(app)

    case document.get_element_by_id(doc, "meta") {
      Some(meta) ->
        dom_node.set_text_content(
          meta,
          Some(
            "48 matrix-transformed diamond tiles · 4 rings · 12 radial spokes · 680x400 WebGL",
          ),
        )
      None -> Nil
    }
  })
}
