import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/areas/main/circle
import konva/areas/main/circle_config
import konva/areas/main/group
import konva/areas/main/group_config
import konva/areas/main/i_rect
import konva/areas/main/layer
import konva/areas/main/line
import konva/areas/main/line_config
import konva/areas/main/node
import konva/areas/main/node_config
import konva/areas/main/node_get_absolute_position_result as pos
import konva/areas/main/rect
import konva/areas/main/rect_config
import konva/areas/main/shape
import konva/areas/main/shape_config
import konva/areas/main/stage
import konva/areas/main/stage_config
import konva/areas/main/text
import konva/areas/main/text_config
import konva/types as t

fn make_grid_line(pts: List(Float)) -> t.GroupOrShape {
  let lc =
    line_config.create()
    |> line_config.with_points(t.LinePointsArray(pts))
    |> shape_config.with_stroke(t.StringOrCanvasGradientString("#e5e5e5"))
    |> shape_config.with_stroke_width(1.0)
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc))))
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
  let _ = layer.add(l, [t.GroupOrShapeShape(shape.widen(rect.new(Some(bg))))])

  let vlines =
    int.range(from: 1, to: 17, with: [], run: fn(acc, i) {
      let x = int.to_float(i * 40)
      [make_grid_line([x, 0.0, x, 400.0]), ..acc]
    })
    |> list.reverse
  let hlines =
    int.range(from: 1, to: 10, with: [], run: fn(acc, i) {
      let y = int.to_float(i * 40)
      [make_grid_line([0.0, y, 680.0, y]), ..acc]
    })
    |> list.reverse
  let _ = layer.add(l, vlines)
  let _ = layer.add(l, hlines)

  // Group A.
  let gac =
    group_config.create()
    |> node_config.with_x(40.0)
    |> node_config.with_y(55.0)
  let group_a = group.new(Some(gac))
  let gab =
    rect_config.create()
    |> rect_config.with_width(160.0)
    |> rect_config.with_height(120.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#f7f7f7"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(1.5)
  let gad =
    circle_config.create()
    |> node_config.with_x(80.0)
    |> node_config.with_y(60.0)
    |> circle_config.with_radius(24.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let gal =
    text_config.create()
    |> node_config.with_x(12.0)
    |> node_config.with_y(10.0)
    |> text_config.with_text("Group A · 0°")
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let _ =
    group.add(group_a, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gab)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(gad)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gal)))),
    ])

  // Group B.
  let gbc =
    group_config.create()
    |> node_config.with_x(340.0)
    |> node_config.with_y(75.0)
    |> node_config.with_rotation(24.0)
  let group_b = group.new(Some(gbc))
  let gbo =
    rect_config.create()
    |> rect_config.with_width(150.0)
    |> rect_config.with_height(110.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#efefef"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)
  let gbi =
    rect_config.create()
    |> rect_config.with_x(22.0)
    |> rect_config.with_y(22.0)
    |> rect_config.with_width(106.0)
    |> rect_config.with_height(66.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#555555"))
    |> rect_config.with_stroke_width(1.5)
    |> rect_config.with_dash([4.0, 3.0])
  let gbd =
    circle_config.create()
    |> node_config.with_x(75.0)
    |> node_config.with_y(55.0)
    |> circle_config.with_radius(14.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#404040"))
  let gbl =
    text_config.create()
    |> node_config.with_x(10.0)
    |> node_config.with_y(8.0)
    |> text_config.with_text("Group B · 24°")
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let _ =
    group.add(group_b, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gbo)))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gbi)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(gbd)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gbl)))),
    ])

  // Group C + nested inner group.
  let gcc =
    group_config.create()
    |> node_config.with_x(495.0)
    |> node_config.with_y(55.0)
    |> node_config.with_scale_x(1.15)
    |> node_config.with_scale_y(1.15)
  let group_c = group.new(Some(gcc))
  let gco =
    rect_config.create()
    |> rect_config.with_width(130.0)
    |> rect_config.with_height(110.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)
  let gcl =
    text_config.create()
    |> node_config.with_x(8.0)
    |> node_config.with_y(8.0)
    |> text_config.with_text("Group C · 1.15x")
    |> text_config.with_font_size(10.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  let icc =
    group_config.create()
    |> node_config.with_x(25.0)
    |> node_config.with_y(25.0)
    |> node_config.with_rotation(15.0)
  let inner_group = group.new(Some(icc))
  let mc =
    circle_config.create()
    |> node_config.with_x(40.0)
    |> node_config.with_y(30.0)
    |> circle_config.with_radius(16.0)
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let marker = circle.new(Some(mc))
  let _ = group.add(inner_group, [t.GroupOrShapeShape(shape.widen(marker))])
  let _ =
    group.add(group_c, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gco)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gcl)))),
      t.GroupOrShapeGroup(inner_group),
    ])

  let _ =
    layer.add(l, [
      t.GroupOrShapeGroup(group_a),
      t.GroupOrShapeGroup(group_b),
      t.GroupOrShapeGroup(group_c),
    ])

  let abs = node.get_absolute_position(marker, None)
  let bbox = node.get_client_rect(group_b, None)
  let ax = int.to_string(float.round(pos.get_x(abs)))
  let ay = int.to_string(float.round(pos.get_y(abs)))
  let bw = int.to_string(float.round(i_rect.get_width(bbox)))
  let bh = int.to_string(float.round(i_rect.get_height(bbox)))
  let strip_txt =
    "Marker abs=(" <> ax <> ", " <> ay <> ") · Group B bbox=" <> bw <> "x" <> bh

  let strip =
    rect_config.create()
    |> rect_config.with_x(28.0)
    |> rect_config.with_y(308.0)
    |> rect_config.with_width(624.0)
    |> rect_config.with_height(60.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#f7f7f7"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(1.5)
  let sum =
    text_config.create()
    |> node_config.with_x(44.0)
    |> node_config.with_y(331.0)
    |> text_config.with_text(strip_txt)
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_family("monospace")
    |> shape_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let _ =
    layer.add(l, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(strip)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(sum)))),
    ])

  let _ = node.draw(l)

  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some(
          "Stage: 680x400 · Nested marker abs=("
          <> ax
          <> ", "
          <> ay
          <> ") · Rotated Group B rect="
          <> bw
          <> "x"
          <> bh,
        ),
      )
  }
}
