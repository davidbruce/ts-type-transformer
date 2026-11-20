import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/areas/main/circle
import konva/areas/main/circle_config
import konva/areas/main/layer
import konva/areas/main/node
import konva/areas/main/node_config
import konva/areas/main/rect
import konva/areas/main/rect_config
import konva/areas/main/regular_polygon
import konva/areas/main/regular_polygon_config
import konva/areas/main/ring
import konva/areas/main/ring_config
import konva/areas/main/shape
import konva/areas/main/shape_config
import konva/areas/main/stage
import konva/areas/main/stage_config
import konva/areas/main/star
import konva/areas/main/star_config
import konva/areas/main/text
import konva/areas/main/text_config
import konva/areas/main/wedge
import konva/areas/main/wedge_config
import konva/types as t

fn add_caption(l: t.Layer(Nil), x: Float, y: Float, label: String) -> Nil {
  let tc =
    text_config.create()
    |> node_config.with_x(x)
    |> node_config.with_y(y)
    |> text_config.with_text(label)
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#555555"))
  let _ = layer.add(l, [t.GroupOrShapeShape(shape.widen(text.new(Some(tc))))])
  Nil
}

pub fn main() -> Nil {
  let sc =
    stage_config.create()
    |> stage_config.with_container(t.HTMLDivElementOrStringString("stage"))
    |> node_config.with_width(680.0)
    |> node_config.with_height(400.0)
  let stg = stage.new(sc)

  let l = layer.new(None)
  let _ = stage.add(stg, l, [])

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
    |> node_config.with_x(415.0)
    |> node_config.with_y(93.0)
    |> circle_config.with_radius(40.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#404040"))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> shape_config.with_stroke_width(2.0)

  let pc =
    regular_polygon_config.create()
    |> node_config.with_x(575.0)
    |> node_config.with_y(93.0)
    |> regular_polygon_config.with_sides(6.0)
    |> regular_polygon_config.with_radius(42.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#efefef"))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> shape_config.with_stroke_width(2.0)

  // Row 2: Star, Ring, Wedge, Framed Text.
  let st =
    star_config.create()
    |> node_config.with_x(95.0)
    |> node_config.with_y(268.0)
    |> star_config.with_num_points(5.0)
    |> star_config.with_inner_radius(19.0)
    |> star_config.with_outer_radius(44.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#737373"))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> shape_config.with_stroke_width(2.0)

  let rc =
    ring_config.create()
    |> node_config.with_x(250.0)
    |> node_config.with_y(268.0)
    |> ring_config.with_inner_radius(22.0)
    |> ring_config.with_outer_radius(42.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#d4d4d4"))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> shape_config.with_stroke_width(2.0)

  let wc =
    wedge_config.create()
    |> node_config.with_x(415.0)
    |> node_config.with_y(268.0)
    |> wedge_config.with_radius(44.0)
    |> wedge_config.with_angle(270.0)
    |> node_config.with_rotation(-90.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#555555"))
    |> shape_config.with_stroke_width(1.0)

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
    |> node_config.with_x(514.0)
    |> node_config.with_y(240.0)
    |> text_config.with_text("KONVA 2D\nMonochrome\nScene Graph")
    |> text_config.with_font_size(13.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  let _ =
    layer.add(l, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(bg)))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(r1)))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(r2)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(cc)))),
      t.GroupOrShapeShape(shape.widen(regular_polygon.new(Some(pc)))),
      t.GroupOrShapeShape(shape.widen(star.new(Some(st)))),
      t.GroupOrShapeShape(shape.widen(ring.new(Some(rc)))),
      t.GroupOrShapeShape(shape.widen(wedge.new(Some(wc)))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(tf)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(txt)))),
    ])

  add_caption(l, 40.0, 142.0, "Rect (filled)")
  add_caption(l, 195.0, 142.0, "Rect (dashed)")
  add_caption(l, 395.0, 142.0, "Circle")
  add_caption(l, 548.0, 142.0, "Hexagon")
  add_caption(l, 62.0, 325.0, "Star (5pt)")
  add_caption(l, 236.0, 325.0, "Ring")
  add_caption(l, 382.0, 325.0, "Wedge 270°")
  add_caption(l, 558.0, 325.0, "Text")

  let _ = node.draw(l)

  let count = int.to_string(list.length(layer.get_children(l)))
  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some("Stage: 680x400 · 7 primitive types · " <> count <> " layer children"),
      )
  }
}
