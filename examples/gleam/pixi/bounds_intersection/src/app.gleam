import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import pixi_js/application
import pixi_js/application_init_options
import pixi_js/container
import pixi_js/graphics
import pixi_js/rectangle
import pixi_js/runtime
import pixi_js/types as t

fn mk_rect(x: Float, y: Float, w: Float, h: Float) -> t.Rectangle(Nil) {
  let n = fn(v) { Some(t.NumberOrStringNumber(v)) }
  rectangle.new(n(x), n(y), n(w), n(h))
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

  let query = mk_rect(190.0, 105.0, 260.0, 170.0)
  let padded = rectangle.pad(rectangle.clone(query), Some(10.0), Some(10.0))
  let hull = rectangle.clone(query)

  let candidates = [
    mk_rect(210.0, 125.0, 90.0, 60.0),
    mk_rect(330.0, 165.0, 95.0, 80.0),
    mk_rect(110.0, 80.0, 130.0, 85.0),
    mk_rect(390.0, 75.0, 130.0, 90.0),
    mk_rect(135.0, 225.0, 110.0, 90.0),
    mk_rect(40.0, 40.0, 100.0, 110.0),
    mk_rect(510.0, 48.0, 125.0, 115.0),
    mk_rect(505.0, 245.0, 130.0, 110.0),
  ]

  let g = graphics.new(None)
  let #(contained, intersecting, disjoint) =
    list.fold(candidates, #(0, 0, 0), fn(acc, c) {
      let #(cnt_c, cnt_i, cnt_d) = acc
      rectangle.enlarge(hull, c)
      let g =
        graphics.rect(
          g,
          rectangle.get_x(c),
          rectangle.get_y(c),
          rectangle.get_width(c),
          rectangle.get_height(c),
        )
      case rectangle.contains_rect(query, c) {
        True -> {
          graphics.fill(g, Some(runtime.dynamic(int.to_float(0x111111))))
          #(cnt_c + 1, cnt_i, cnt_d)
        }
        False ->
          case rectangle.intersects(query, c, None) {
            True -> {
              g
              |> graphics.fill(Some(runtime.dynamic(int.to_float(0x737373))))
              |> stroke_g(2.0, 0x111111)
              #(cnt_c, cnt_i + 1, cnt_d)
            }
            False -> {
              g
              |> graphics.fill(Some(runtime.dynamic(int.to_float(0xe5e5e5))))
              |> stroke_g(1.0, 0xa3a3a3)
              #(cnt_c, cnt_i, cnt_d + 1)
            }
          }
      }
    })

  g
  |> graphics.rect(
    rectangle.get_x(hull),
    rectangle.get_y(hull),
    rectangle.get_width(hull),
    rectangle.get_height(hull),
  )
  |> stroke_g(1.5, 0xa3a3a3)
  |> graphics.rect(
    rectangle.get_x(padded),
    rectangle.get_y(padded),
    rectangle.get_width(padded),
    rectangle.get_height(padded),
  )
  |> stroke_g(1.0, 0x737373)
  |> graphics.rect(
    rectangle.get_x(query),
    rectangle.get_y(query),
    rectangle.get_width(query),
    rectangle.get_height(query),
  )
  |> stroke_g(2.5, 0x111111)

  container.add_child(stage, [g])
  application.render(app)

  let hull_w = int.to_string(float.round(rectangle.get_width(hull)))
  let hull_h = int.to_string(float.round(rectangle.get_height(hull)))
  case document.get_element_by_id(doc, "meta") {
    Some(meta) ->
      dom_node.set_text_content(
        meta,
        Some(
          "query 260x170 · "
          <> int.to_string(contained)
          <> " contained · "
          <> int.to_string(intersecting)
          <> " intersecting · "
          <> int.to_string(disjoint)
          <> " disjoint · enlarged hull "
          <> hull_w
          <> "x"
          <> hull_h,
        ),
      )
    None -> Nil
  }
}
