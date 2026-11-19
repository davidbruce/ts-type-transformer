import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/areas/main/circle
import konva/areas/main/circle_config
import konva/areas/main/container
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
  let tc = text_config.create()
  node_config.set_x(tc, Some(x))
  node_config.set_y(tc, Some(y))
  text_config.set_text(tc, Some(label))
  text_config.set_font_size(tc, Some(11.0))
  text_config.set_font_family(tc, Some("monospace"))
  shape_config.set_fill(tc, Some(t.StringOrCanvasGradientString("#555555")))
  let _ = container.add(
    l,
    [t.GroupOrShapeShape(shape.widen(text.new(Some(tc))))],
    t.group_or_shape_codec(),
  )
  Nil
}

pub fn main() -> Nil {
  let sc = stage_config.create()
  stage_config.set_container(sc, Some(t.HTMLDivElementOrStringString("stage")))
  node_config.set_width(sc, Some(680.0))
  node_config.set_height(sc, Some(400.0))
  let stg = stage.new(sc)

  let l = layer.new(None)
  let _ = stage.add(stg, l, [])

  let bg = rect_config.create()
  node_config.set_x(bg, Some(0.0))
  node_config.set_y(bg, Some(0.0))
  node_config.set_width(bg, Some(680.0))
  node_config.set_height(bg, Some(400.0))
  shape_config.set_fill(bg, Some(t.StringOrCanvasGradientString("#ffffff")))

  // Row 1: Rounded filled Rect, dashed Rect, Circle, Hexagon.
  let r1 = rect_config.create()
  node_config.set_x(r1, Some(40.0))
  node_config.set_y(r1, Some(55.0))
  node_config.set_width(r1, Some(110.0))
  node_config.set_height(r1, Some(76.0))
  rect_config.set_corner_radius(r1, Some(t.NumberOrNumberArrayNumber(8.0)))
  shape_config.set_fill(r1, Some(t.StringOrCanvasGradientString("#111111")))

  let r2 = rect_config.create()
  node_config.set_x(r2, Some(195.0))
  node_config.set_y(r2, Some(55.0))
  node_config.set_width(r2, Some(110.0))
  node_config.set_height(r2, Some(76.0))
  shape_config.set_fill(r2, Some(t.StringOrCanvasGradientString("#ffffff")))
  shape_config.set_stroke(r2, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(r2, Some(2.0))
  shape_config.set_dash(r2, Some([6.0, 4.0]))

  let cc = circle_config.create()
  node_config.set_x(cc, Some(415.0))
  node_config.set_y(cc, Some(93.0))
  circle_config.set_radius(cc, Some(40.0))
  shape_config.set_fill(cc, Some(t.StringOrCanvasGradientString("#404040")))
  shape_config.set_stroke(cc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(cc, Some(2.0))

  let pc = regular_polygon_config.create()
  node_config.set_x(pc, Some(575.0))
  node_config.set_y(pc, Some(93.0))
  regular_polygon_config.set_sides(pc, 6.0)
  regular_polygon_config.set_radius(pc, 42.0)
  shape_config.set_fill(pc, Some(t.StringOrCanvasGradientString("#efefef")))
  shape_config.set_stroke(pc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(pc, Some(2.0))

  // Row 2: Star, Ring, Wedge, Framed Text.
  let st = star_config.create()
  node_config.set_x(st, Some(95.0))
  node_config.set_y(st, Some(268.0))
  star_config.set_num_points(st, 5.0)
  star_config.set_inner_radius(st, 19.0)
  star_config.set_outer_radius(st, 44.0)
  shape_config.set_fill(st, Some(t.StringOrCanvasGradientString("#737373")))
  shape_config.set_stroke(st, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(st, Some(2.0))

  let rc = ring_config.create()
  node_config.set_x(rc, Some(250.0))
  node_config.set_y(rc, Some(268.0))
  ring_config.set_inner_radius(rc, 22.0)
  ring_config.set_outer_radius(rc, 42.0)
  shape_config.set_fill(rc, Some(t.StringOrCanvasGradientString("#d4d4d4")))
  shape_config.set_stroke(rc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(rc, Some(2.0))

  let wc = wedge_config.create()
  node_config.set_x(wc, Some(415.0))
  node_config.set_y(wc, Some(268.0))
  wedge_config.set_radius(wc, 44.0)
  wedge_config.set_angle(wc, 270.0)
  node_config.set_rotation(wc, Some(-90.0))
  shape_config.set_fill(wc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke(wc, Some(t.StringOrCanvasGradientString("#555555")))
  shape_config.set_stroke_width(wc, Some(1.0))

  let tf = rect_config.create()
  node_config.set_x(tf, Some(500.0))
  node_config.set_y(tf, Some(224.0))
  node_config.set_width(tf, Some(150.0))
  node_config.set_height(tf, Some(88.0))
  shape_config.set_fill(tf, Some(t.StringOrCanvasGradientString("#f7f7f7")))
  shape_config.set_stroke(tf, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(tf, Some(1.5))

  let txt = text_config.create()
  node_config.set_x(txt, Some(514.0))
  node_config.set_y(txt, Some(240.0))
  text_config.set_text(txt, Some("KONVA 2D\nMonochrome\nScene Graph"))
  text_config.set_font_size(txt, Some(13.0))
  text_config.set_font_family(txt, Some("monospace"))
  shape_config.set_fill(txt, Some(t.StringOrCanvasGradientString("#111111")))

  let _ = container.add(
    l,
    [
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
    ],
    t.group_or_shape_codec(),
  )

  add_caption(l, 40.0, 142.0, "Rect (filled)")
  add_caption(l, 195.0, 142.0, "Rect (dashed)")
  add_caption(l, 395.0, 142.0, "Circle")
  add_caption(l, 548.0, 142.0, "Hexagon")
  add_caption(l, 62.0, 325.0, "Star (5pt)")
  add_caption(l, 236.0, 325.0, "Ring")
  add_caption(l, 382.0, 325.0, "Wedge 270°")
  add_caption(l, 558.0, 325.0, "Text")

  let _ = node.draw(l)

  let count = int.to_string(list.length(container.get_children(l, t.group_or_shape_codec())))
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
