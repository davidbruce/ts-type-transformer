import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/dynamic
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import konva/areas/main/arrow
import konva/areas/main/arrow_config
import konva/areas/main/container
import konva/areas/main/group
import konva/areas/main/group_config
import konva/areas/main/layer
import konva/areas/main/line
import konva/areas/main/line_config
import konva/areas/main/node
import konva/areas/main/node_config
import konva/areas/main/rect
import konva/areas/main/rect_config
import konva/areas/main/shape
import konva/areas/main/shape_config
import konva/areas/main/stage
import konva/areas/main/stage_config
import konva/areas/main/text
import konva/areas/main/text_config
import konva/types as t

fn stage_box(
  x: Float,
  title: String,
  sub: String,
  fill_hex: String,
  ink_hex: String,
) -> t.GroupOrShape {
  let gc = group_config.create()
  node_config.set_x(gc, Some(x))
  node_config.set_y(gc, Some(95.0))
  let g = group.new(Some(gc))

  let rc = rect_config.create()
  node_config.set_width(rc, Some(126.0))
  node_config.set_height(rc, Some(68.0))
  rect_config.set_corner_radius(rc, Some(t.NumberOrNumberArrayNumber(3.0)))
  shape_config.set_fill(rc, Some(t.StringOrCanvasGradientString(fill_hex)))
  shape_config.set_stroke(rc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(rc, Some(2.0))

  let t1 = text_config.create()
  node_config.set_x(t1, Some(12.0))
  node_config.set_y(t1, Some(16.0))
  text_config.set_text(t1, Some(title))
  text_config.set_font_size(t1, Some(12.0))
  text_config.set_font_style(t1, Some("bold"))
  text_config.set_font_family(t1, Some("monospace"))
  shape_config.set_fill(t1, Some(t.StringOrCanvasGradientString(ink_hex)))

  let t2 = text_config.create()
  node_config.set_x(t2, Some(12.0))
  node_config.set_y(t2, Some(38.0))
  text_config.set_text(t2, Some(sub))
  text_config.set_font_size(t2, Some(11.0))
  text_config.set_font_family(t2, Some("monospace"))
  shape_config.set_fill(t2, Some(t.StringOrCanvasGradientString(ink_hex)))

  let _ = container.add(
    g,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(rc)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(t1)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(t2)))),
    ],
    t.group_or_shape_codec(),
  )
  t.GroupOrShapeGroup(g)
}

fn make_arrow(pts: List(Float)) -> t.GroupOrShape {
  let ac = arrow_config.create()
  line_config.set_points(ac, Some(t.LinePointsArray(pts)))
  arrow_config.set_pointer_length(ac, Some(8.0))
  arrow_config.set_pointer_width(ac, Some(8.0))
  shape_config.set_fill(ac, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke(ac, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(ac, Some(2.0))
  t.GroupOrShapeShape(shape.widen(arrow.new(Some(ac))))
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

  let fbc = line_config.create()
  line_config.set_points(
    fbc,
    Some(t.LinePointsArray([589.0, 163.0, 589.0, 212.0, 91.0, 212.0, 91.0, 168.0])),
  )
  shape_config.set_stroke(fbc, Some(t.StringOrCanvasGradientString("#737373")))
  shape_config.set_stroke_width(fbc, Some(1.5))
  shape_config.set_dash(fbc, Some([6.0, 4.0]))

  let bar = rect_config.create()
  node_config.set_x(bar, Some(28.0))
  node_config.set_y(bar, Some(255.0))
  node_config.set_width(bar, Some(624.0))
  node_config.set_height(bar, Some(58.0))
  rect_config.set_corner_radius(bar, Some(t.NumberOrNumberArrayNumber(3.0)))
  shape_config.set_fill(bar, Some(t.StringOrCanvasGradientString("#f7f7f7")))
  shape_config.set_stroke(bar, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(bar, Some(1.5))

  let btxt = text_config.create()
  node_config.set_x(btxt, Some(180.0))
  node_config.set_y(btxt, Some(278.0))
  text_config.set_text(btxt, Some("Shared DOM & Package Handle Runtime"))
  text_config.set_font_size(btxt, Some(12.0))
  text_config.set_font_family(btxt, Some("monospace"))
  shape_config.set_fill(btxt, Some(t.StringOrCanvasGradientString("#111111")))

  let _ = container.add(
    l,
    [
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
      t.GroupOrShapeShape(shape.widen(line.new(Some(fbc), t.line_config_codec()))),
      t.GroupOrShapeShape(shape.widen(rect.new(Some(bar)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(btxt)))),
    ],
    t.group_or_shape_codec(),
  )

  let _ = node.draw(l)

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
