import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/dynamic
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import konva/arrow
import konva/arrow_config
import konva/container
import konva/group
import konva/group_config
import konva/layer
import konva/line
import konva/line_config
import konva/node
import konva/rect
import konva/rect_config
import konva/shape
import konva/stage
import konva/stage_config
import konva/text
import konva/text_config
import konva/types as t

fn stage_box(
  x: Float,
  title: String,
  sub: String,
  fill_hex: String,
  ink_hex: String,
) -> t.GroupOrShape {
  let gc =
    group_config.create()
    |> group_config.with_x(x)
    |> group_config.with_y(95.0)
  let g = group.new(Some(gc))

  let rc =
    rect_config.create()
    |> rect_config.with_width(126.0)
    |> rect_config.with_height(68.0)
    |> rect_config.with_corner_radius(t.NumberOrNumberArrayNumber(3.0))
    |> rect_config.with_fill(t.StringOrCanvasGradientString(fill_hex))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)

  let t1 =
    text_config.create()
    |> text_config.with_x(12.0)
    |> text_config.with_y(16.0)
    |> text_config.with_text(title)
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_style("bold")
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString(ink_hex))

  let t2 =
    text_config.create()
    |> text_config.with_x(12.0)
    |> text_config.with_y(38.0)
    |> text_config.with_text(sub)
    |> text_config.with_font_size(11.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString(ink_hex))

  group.add(g, [
    t.GroupOrShapeShape(shape.widen(rect.new(Some(rc)))),
    t.GroupOrShapeShape(shape.widen(text.new(Some(t1)))),
    t.GroupOrShapeShape(shape.widen(text.new(Some(t2)))),
  ])
  t.GroupOrShapeGroup(g)
}

fn make_arrow(pts: List(Float)) -> t.GroupOrShape {
  let ac =
    arrow_config.create()
    |> line_config.with_points(t.LinePointsArray(pts))
    |> arrow_config.with_pointer_length(8.0)
    |> arrow_config.with_pointer_width(8.0)
    |> arrow_config.with_fill(t.StringOrCanvasGradientString("#111111"))
    |> arrow_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> arrow_config.with_stroke_width(2.0)
  t.GroupOrShapeShape(shape.widen(arrow.new(Some(ac))))
}

pub fn main() -> Nil {
  let sc =
    stage_config.create()
    |> stage_config.with_container(t.HTMLDivElementOrStringString("stage"))
    |> stage_config.with_width(680.0)
    |> stage_config.with_height(400.0)
  let stg = stage.new(sc)

  let l = layer.new(None)
  stage.add(stg, l, [])

  let bg =
    rect_config.create()
    |> rect_config.with_x(0.0)
    |> rect_config.with_y(0.0)
    |> rect_config.with_width(680.0)
    |> rect_config.with_height(400.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))

  let fbc =
    line_config.create()
    |> line_config.with_points(
      t.LinePointsArray([589.0, 163.0, 589.0, 212.0, 91.0, 212.0, 91.0, 168.0]),
    )
    |> line_config.with_stroke(t.StringOrCanvasGradientString("#737373"))
    |> line_config.with_stroke_width(1.5)
    |> line_config.with_dash([6.0, 4.0])

  let bar =
    rect_config.create()
    |> rect_config.with_x(28.0)
    |> rect_config.with_y(255.0)
    |> rect_config.with_width(624.0)
    |> rect_config.with_height(58.0)
    |> rect_config.with_corner_radius(t.NumberOrNumberArrayNumber(3.0))
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#f7f7f7"))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(1.5)

  let btxt =
    text_config.create()
    |> text_config.with_x(180.0)
    |> text_config.with_y(278.0)
    |> text_config.with_text("Shared DOM & Package Handle Runtime")
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  layer.add(l, [
    t.GroupOrShapeShape(shape.widen(rect.new(Some(bg)))),
    stage_box(28.0, "01 PARSE", ".d.ts AST", "#ffffff", "#111111"),
    stage_box(194.0, "02 LOWER", "Typed IR", "#f7f7f7", "#111111"),
    stage_box(360.0, "03 EMIT", "6 Targets", "#efefef", "#111111"),
    stage_box(526.0, "04 RUN", "WASM / JS", "#111111", "#ffffff"),
    make_arrow([154.0, 129.0, 190.0, 129.0]),
    make_arrow([320.0, 129.0, 356.0, 129.0]),
    make_arrow([486.0, 129.0, 522.0, 129.0]),
    make_arrow([257.0, 163.0, 257.0, 251.0]),
    make_arrow([423.0, 163.0, 423.0, 251.0]),
    t.GroupOrShapeShape(shape.widen(line.new(Some(fbc)))),
    t.GroupOrShapeShape(shape.widen(rect.new(Some(bar)))),
    t.GroupOrShapeShape(shape.widen(text.new(Some(btxt)))),
  ])

  node.draw(l)

  let rects = int.to_string(list.length(container.find(l, dynamic.string("Rect"))))
  let arrows = int.to_string(list.length(container.find(l, dynamic.string("Arrow"))))
  let json_len = int.to_string(string.length(node.to_json(l)))

  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some(
          "Pipeline diagram · "
          <> rects
          <> " rects · "
          <> arrows
          <> " arrows · JSON "
          <> json_len
          <> " bytes",
        ),
      )
  }
}
