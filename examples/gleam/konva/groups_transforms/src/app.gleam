import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/areas/main/circle
import konva/areas/main/circle_config
import konva/areas/main/container
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

fn range(from: Int, to: Int) -> List(Int) {
  case from > to {
    True -> []
    False -> [from, ..range(from + 1, to)]
  }
}

fn make_grid_line(pts: List(Float)) -> t.GroupOrShape {
  let lc = line_config.create()
  line_config.set_points(lc, Some(t.LinePointsArray(pts)))
  shape_config.set_stroke(lc, Some(t.StringOrCanvasGradientString("#e5e5e5")))
  shape_config.set_stroke_width(lc, Some(1.0))
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc), t.line_config_codec())))
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
  let _ = container.add(
    l,
    [t.GroupOrShapeShape(shape.widen(rect.new(Some(bg))))],
    t.group_or_shape_codec(),
  )

  let vlines =
    list.map(range(1, 16), fn(i) {
      let x = int.to_float(i * 40)
      make_grid_line([x, 0.0, x, 400.0])
    })
  let hlines =
    list.map(range(1, 9), fn(i) {
      let y = int.to_float(i * 40)
      make_grid_line([0.0, y, 680.0, y])
    })
  let _ = container.add(l, vlines, t.group_or_shape_codec())
  let _ = container.add(l, hlines, t.group_or_shape_codec())

  // Group A.
  let gac = group_config.create()
  node_config.set_x(gac, Some(40.0))
  node_config.set_y(gac, Some(55.0))
  let group_a = group.new(Some(gac))
  let gab = rect_config.create()
  node_config.set_width(gab, Some(160.0))
  node_config.set_height(gab, Some(120.0))
  shape_config.set_fill(gab, Some(t.StringOrCanvasGradientString("#f7f7f7")))
  shape_config.set_stroke(gab, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(gab, Some(1.5))
  let gad = circle_config.create()
  node_config.set_x(gad, Some(80.0))
  node_config.set_y(gad, Some(60.0))
  circle_config.set_radius(gad, Some(24.0))
  shape_config.set_fill(gad, Some(t.StringOrCanvasGradientString("#111111")))
  let gal = text_config.create()
  node_config.set_x(gal, Some(12.0))
  node_config.set_y(gal, Some(10.0))
  text_config.set_text(gal, Some("Group A · 0°"))
  text_config.set_font_size(gal, Some(11.0))
  text_config.set_font_family(gal, Some("monospace"))
  shape_config.set_fill(gal, Some(t.StringOrCanvasGradientString("#111111")))
  let _ = container.add(
    group_a,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gab)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(gad)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gal)))),
    ],
    t.group_or_shape_codec(),
  )

  // Group B.
  let gbc = group_config.create()
  node_config.set_x(gbc, Some(340.0))
  node_config.set_y(gbc, Some(75.0))
  node_config.set_rotation(gbc, Some(24.0))
  let group_b = group.new(Some(gbc))
  let gbo = rect_config.create()
  node_config.set_width(gbo, Some(150.0))
  node_config.set_height(gbo, Some(110.0))
  shape_config.set_fill(gbo, Some(t.StringOrCanvasGradientString("#efefef")))
  shape_config.set_stroke(gbo, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(gbo, Some(2.0))
  let gbi = rect_config.create()
  node_config.set_x(gbi, Some(22.0))
  node_config.set_y(gbi, Some(22.0))
  node_config.set_width(gbi, Some(106.0))
  node_config.set_height(gbi, Some(66.0))
  shape_config.set_fill(gbi, Some(t.StringOrCanvasGradientString("#ffffff")))
  shape_config.set_stroke(gbi, Some(t.StringOrCanvasGradientString("#555555")))
  shape_config.set_stroke_width(gbi, Some(1.5))
  shape_config.set_dash(gbi, Some([4.0, 3.0]))
  let gbd = circle_config.create()
  node_config.set_x(gbd, Some(75.0))
  node_config.set_y(gbd, Some(55.0))
  circle_config.set_radius(gbd, Some(14.0))
  shape_config.set_fill(gbd, Some(t.StringOrCanvasGradientString("#404040")))
  let gbl = text_config.create()
  node_config.set_x(gbl, Some(10.0))
  node_config.set_y(gbl, Some(8.0))
  text_config.set_text(gbl, Some("Group B · 24°"))
  text_config.set_font_size(gbl, Some(11.0))
  text_config.set_font_family(gbl, Some("monospace"))
  shape_config.set_fill(gbl, Some(t.StringOrCanvasGradientString("#111111")))
  let _ = container.add(
    group_b,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gbo)))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gbi)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(gbd)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gbl)))),
    ],
    t.group_or_shape_codec(),
  )

  // Group C + nested inner group.
  let gcc = group_config.create()
  node_config.set_x(gcc, Some(495.0))
  node_config.set_y(gcc, Some(55.0))
  node_config.set_scale_x(gcc, Some(1.15))
  node_config.set_scale_y(gcc, Some(1.15))
  let group_c = group.new(Some(gcc))
  let gco = rect_config.create()
  node_config.set_width(gco, Some(130.0))
  node_config.set_height(gco, Some(110.0))
  shape_config.set_fill(gco, Some(t.StringOrCanvasGradientString("#ffffff")))
  shape_config.set_stroke(gco, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(gco, Some(2.0))
  let gcl = text_config.create()
  node_config.set_x(gcl, Some(8.0))
  node_config.set_y(gcl, Some(8.0))
  text_config.set_text(gcl, Some("Group C · 1.15x"))
  text_config.set_font_size(gcl, Some(10.0))
  text_config.set_font_family(gcl, Some("monospace"))
  shape_config.set_fill(gcl, Some(t.StringOrCanvasGradientString("#111111")))

  let icc = group_config.create()
  node_config.set_x(icc, Some(25.0))
  node_config.set_y(icc, Some(25.0))
  node_config.set_rotation(icc, Some(15.0))
  let inner_group = group.new(Some(icc))
  let mc = circle_config.create()
  node_config.set_x(mc, Some(40.0))
  node_config.set_y(mc, Some(30.0))
  circle_config.set_radius(mc, Some(16.0))
  shape_config.set_fill(mc, Some(t.StringOrCanvasGradientString("#111111")))
  let marker = circle.new(Some(mc))
  let _ = container.add(
    inner_group,
    [t.GroupOrShapeShape(shape.widen(marker))],
    t.group_or_shape_codec(),
  )
  let _ = container.add(
    group_c,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(gco)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(gcl)))),
      t.GroupOrShapeGroup(inner_group),
    ],
    t.group_or_shape_codec(),
  )

  let _ = container.add(
    l,
    [
      t.GroupOrShapeGroup(group_a),
      t.GroupOrShapeGroup(group_b),
      t.GroupOrShapeGroup(group_c),
    ],
    t.group_or_shape_codec(),
  )

  let abs = node.get_absolute_position(marker, None)
  let bbox = node.get_client_rect(group_b, None)
  let ax = int.to_string(float.round(pos.get_x(abs)))
  let ay = int.to_string(float.round(pos.get_y(abs)))
  let bw = int.to_string(float.round(i_rect.get_width(bbox)))
  let bh = int.to_string(float.round(i_rect.get_height(bbox)))
  let strip_txt =
    "Marker abs=(" <> ax <> ", " <> ay <> ") · Group B bbox=" <> bw <> "x" <> bh

  let strip = rect_config.create()
  node_config.set_x(strip, Some(28.0))
  node_config.set_y(strip, Some(308.0))
  node_config.set_width(strip, Some(624.0))
  node_config.set_height(strip, Some(60.0))
  shape_config.set_fill(strip, Some(t.StringOrCanvasGradientString("#f7f7f7")))
  shape_config.set_stroke(strip, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(strip, Some(1.5))
  let sum = text_config.create()
  node_config.set_x(sum, Some(44.0))
  node_config.set_y(sum, Some(331.0))
  text_config.set_text(sum, Some(strip_txt))
  text_config.set_font_size(sum, Some(12.0))
  text_config.set_font_family(sum, Some("monospace"))
  shape_config.set_fill(sum, Some(t.StringOrCanvasGradientString("#111111")))
  let _ = container.add(
    l,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(strip)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(sum)))),
    ],
    t.group_or_shape_codec(),
  )

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
