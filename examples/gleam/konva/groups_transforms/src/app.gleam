import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/circle
import konva/circle_config
import konva/container
import konva/group
import konva/group_config
import konva/i_rect
import konva/layer
import konva/line
import konva/line_config
import konva/node
import konva/node_get_absolute_position_result as pos
import konva/rect
import konva/rect_config
import konva/stage
import konva/stage_config
import konva/text
import konva/text_config
import konva/types as t

fn make_grid_line(pts: List(Float)) -> t.Node(Nil) {
  let lc =
    line_config.create()
    |> line_config.with_points(t.LinePointsArray(pts))
    |> line_config.with_stroke(t.StringOrCanvasGradientString("#e5e5e5"))
    |> line_config.with_stroke_width(1.0)
  node.upcast(line.new_with(Some(lc)))
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
  container.add(l, [rect.new_with(Some(bg))])

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
  container.add(l, vlines)
  container.add(l, hlines)

  // Group A.
  let gac =
    group_config.create()
    |> group_config.with_x(40.0)
    |> group_config.with_y(55.0)
  let group_a = group.new_with(Some(gac))
  let gab =
    rect_config.create()
    |> rect_config.with_width(160.0)
    |> rect_config.with_height(120.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#f7f7f7"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(1.5)
  let gad =
    circle_config.create()
    |> circle_config.with_x(80.0)
    |> circle_config.with_y(60.0)
    |> circle_config.with_radius(24.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let gal =
    text_config.create()
    |> text_config.with_x(12.0)
    |> text_config.with_y(10.0)
    |> text_config.with_text("Group A · 0°")
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  container.add(group_a, [
    node.upcast(rect.new_with(Some(gab))),
    node.upcast(circle.new_with(Some(gad))),
    node.upcast(text.new_with(Some(gal))),
  ])

  // Group B.
  let gbc =
    group_config.create()
    |> group_config.with_x(340.0)
    |> group_config.with_y(75.0)
    |> group_config.with_rotation(24.0)
  let group_b = group.new_with(Some(gbc))
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
    |> circle_config.with_x(75.0)
    |> circle_config.with_y(55.0)
    |> circle_config.with_radius(14.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#404040"))
  let gbl =
    text_config.create()
    |> text_config.with_x(10.0)
    |> text_config.with_y(8.0)
    |> text_config.with_text("Group B · 24°")
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  container.add(group_b, [
    node.upcast(rect.new_with(Some(gbo))),
    node.upcast(rect.new_with(Some(gbi))),
    node.upcast(circle.new_with(Some(gbd))),
    node.upcast(text.new_with(Some(gbl))),
  ])

  // Group C + nested inner group.
  let gcc =
    group_config.create()
    |> group_config.with_x(495.0)
    |> group_config.with_y(55.0)
    |> group_config.with_scale_x(1.15)
    |> group_config.with_scale_y(1.15)
  let group_c = group.new_with(Some(gcc))
  let gco =
    rect_config.create()
    |> rect_config.with_width(130.0)
    |> rect_config.with_height(110.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)
  let gcl =
    text_config.create()
    |> text_config.with_x(8.0)
    |> text_config.with_y(8.0)
    |> text_config.with_text("Group C · 1.15x")
    |> text_config.with_font_size(10.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  let icc =
    group_config.create()
    |> group_config.with_x(25.0)
    |> group_config.with_y(25.0)
    |> group_config.with_rotation(15.0)
  let inner_group = group.new_with(Some(icc))
  let mc =
    circle_config.create()
    |> circle_config.with_x(40.0)
    |> circle_config.with_y(30.0)
    |> circle_config.with_radius(16.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let marker = circle.new_with(Some(mc))
  container.add(inner_group, [marker])
  container.add(group_c, [
    node.upcast(rect.new_with(Some(gco))),
    node.upcast(text.new_with(Some(gcl))),
    node.upcast(inner_group),
  ])

  container.add(l, [group_a, group_b, group_c])

  let abs = node.get_absolute_position(marker)
  let bbox = node.get_client_rect(group_b)
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
    |> text_config.with_x(44.0)
    |> text_config.with_y(331.0)
    |> text_config.with_text(strip_txt)
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  container.add(l, [
    node.upcast(rect.new_with(Some(strip))),
    node.upcast(text.new_with(Some(sum))),
  ])

  node.draw(l)

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
