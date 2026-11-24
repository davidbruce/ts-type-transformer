import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/circle
import konva/circle_config
import konva/container
import konva/layer
import konva/node
import konva/rect
import konva/rect_config
import konva/regular_polygon
import konva/regular_polygon_config
import konva/ring
import konva/ring_config
import konva/stage
import konva/stage_config
import konva/star
import konva/star_config
import konva/text
import konva/text_config
import konva/types as t
import konva/wedge
import konva/wedge_config

fn add_caption(l: t.Layer(Nil), x: Float, y: Float, label: String) -> Nil {
  let tc =
    text_config.create()
    |> text_config.with_x(x)
    |> text_config.with_y(y)
    |> text_config.with_text(label)
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#555555"))
  container.add(l, [text.new_with(Some(tc))])
  Nil
}

pub fn main() -> Nil {
  let sc =
    stage_config.create()
    |> stage_config.with_container(t.HTMLDivElementOrStringString("stage"))
    |> stage_config.with_width(680.0)
    |> stage_config.with_height(400.0)
  let stg = stage.new(sc)

  let l = layer.new()
  stage.add(stg, l, [])

  let bg =
    rect_config.create()
    |> rect_config.with_x(0.0)
    |> rect_config.with_y(0.0)
    |> rect_config.with_width(680.0)
    |> rect_config.with_height(400.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))

  // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
  let r1 =
    rect_config.create()
    |> rect_config.with_x(40.0)
    |> rect_config.with_y(55.0)
    |> rect_config.with_width(110.0)
    |> rect_config.with_height(76.0)
    |> rect_config.with_corner_radius(t.NumberOrNumberArrayNumber(8.0))
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  let r2 =
    rect_config.create()
    |> rect_config.with_x(195.0)
    |> rect_config.with_y(55.0)
    |> rect_config.with_width(110.0)
    |> rect_config.with_height(76.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)
    |> rect_config.with_dash([6.0, 4.0])

  let cc =
    circle_config.create()
    |> circle_config.with_x(415.0)
    |> circle_config.with_y(93.0)
    |> circle_config.with_radius(40.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#404040"))
    |> circle_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> circle_config.with_stroke_width(2.0)

  let pc =
    regular_polygon_config.create()
    |> regular_polygon_config.with_x(575.0)
    |> regular_polygon_config.with_y(93.0)
    |> regular_polygon_config.with_sides(6.0)
    |> regular_polygon_config.with_radius(42.0)
    |> regular_polygon_config.with_fill(t.StringOrCanvasGradientString(
      "#efefef",
    ))
    |> regular_polygon_config.with_stroke(t.StringOrCanvasGradientString(
      "#111111",
    ))
    |> regular_polygon_config.with_stroke_width(2.0)

  // Row 2: Star, Ring, Wedge, Framed Text.
  let st =
    star_config.create()
    |> star_config.with_x(95.0)
    |> star_config.with_y(268.0)
    |> star_config.with_num_points(5.0)
    |> star_config.with_inner_radius(19.0)
    |> star_config.with_outer_radius(44.0)
    |> star_config.with_fill(t.StringOrCanvasGradientString("#737373"))
    |> star_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> star_config.with_stroke_width(2.0)

  let rc =
    ring_config.create()
    |> ring_config.with_x(250.0)
    |> ring_config.with_y(268.0)
    |> ring_config.with_inner_radius(22.0)
    |> ring_config.with_outer_radius(42.0)
    |> ring_config.with_fill(t.StringOrCanvasGradientString("#d4d4d4"))
    |> ring_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> ring_config.with_stroke_width(2.0)

  let wc =
    wedge_config.create()
    |> wedge_config.with_x(415.0)
    |> wedge_config.with_y(268.0)
    |> wedge_config.with_radius(44.0)
    |> wedge_config.with_angle(270.0)
    |> wedge_config.with_rotation(-90.0)
    |> wedge_config.with_fill(t.StringOrCanvasGradientString("#111111"))
    |> wedge_config.with_stroke(t.StringOrCanvasGradientString("#555555"))
    |> wedge_config.with_stroke_width(1.0)

  let tf =
    rect_config.create()
    |> rect_config.with_x(500.0)
    |> rect_config.with_y(224.0)
    |> rect_config.with_width(150.0)
    |> rect_config.with_height(88.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#f7f7f7"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(1.5)

  let txt =
    text_config.create()
    |> text_config.with_x(514.0)
    |> text_config.with_y(240.0)
    |> text_config.with_text("KONVA 2D\nMonochrome\nScene Graph")
    |> text_config.with_font_size(13.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  container.add(l, [
    node.upcast(rect.new_with(Some(bg))),
    node.upcast(rect.new_with(Some(r1))),
    node.upcast(rect.new_with(Some(r2))),
    node.upcast(circle.new_with(Some(cc))),
    node.upcast(regular_polygon.new_with(Some(pc))),
    node.upcast(star.new_with(Some(st))),
    node.upcast(ring.new_with(Some(rc))),
    node.upcast(wedge.new_with(Some(wc))),
    node.upcast(rect.new_with(Some(tf))),
    node.upcast(text.new_with(Some(txt))),
  ])

  add_caption(l, 40.0, 142.0, "Rect (filled)")
  add_caption(l, 195.0, 142.0, "Rect (dashed)")
  add_caption(l, 395.0, 142.0, "Circle")
  add_caption(l, 548.0, 142.0, "Hexagon")
  add_caption(l, 62.0, 325.0, "Star (5pt)")
  add_caption(l, 236.0, 325.0, "Ring")
  add_caption(l, 382.0, 325.0, "Wedge 270°")
  add_caption(l, 558.0, 325.0, "Text")

  node.draw(l)

  let count = int.to_string(list.length(layer.get_children(l)))
  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some(
          "Stage: 680x400 · 7 primitive types · " <> count <> " layer children",
        ),
      )
  }
}
