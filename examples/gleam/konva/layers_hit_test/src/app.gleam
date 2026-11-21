import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/circle
import konva/circle_config
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
import konva/vector2d
import konva/runtime
import konva/types as t

fn make_line(pts: List(Float), col: String, sw: Float) -> t.GroupOrShape {
  let lc =
    line_config.create()
    |> line_config.with_points(t.LinePointsArray(pts))
    |> line_config.with_stroke(t.StringOrCanvasGradientString(col))
    |> line_config.with_stroke_width(sw)
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc))))
}

fn make_card(id: String, x: Float, y: Float, fill_hex: String) -> t.Rect(Nil) {
  let rc =
    rect_config.create()
    |> rect_config.with_id(id)
    |> rect_config.with_name("card")
    |> rect_config.with_x(x)
    |> rect_config.with_y(y)
    |> rect_config.with_width(200.0)
    |> rect_config.with_height(130.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString(fill_hex))
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> rect_config.with_stroke_width(2.0)
  rect.new(Some(rc))
}

pub fn main() -> Nil {
  let sc =
    stage_config.create()
    |> stage_config.with_container(t.HTMLDivElementOrStringString("stage"))
    |> stage_config.with_width(680.0)
    |> stage_config.with_height(400.0)
  let stg = stage.new(sc)

  let bg_layer = layer.new(None)
  let fg_layer = layer.new(None)
  let _ = stage.add(stg, bg_layer, [])
  let _ = stage.add(stg, fg_layer, [])

  // Layer 1: Blueprint Grid.
  let bgr =
    rect_config.create()
    |> rect_config.with_x(0.0)
    |> rect_config.with_y(0.0)
    |> rect_config.with_width(680.0)
    |> rect_config.with_height(400.0)
    |> rect_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
  let _ = layer.add(bg_layer, [t.GroupOrShapeShape(shape.widen(rect.new(Some(bgr))))])

  let vlines =
    int.range(from: 1, to: 17, with: [], run: fn(acc, i) {
      let x = int.to_float(i * 40)
      [make_line([x, 20.0, x, 380.0], "#e5e5e5", 1.0), ..acc]
    })
    |> list.reverse
  let hlines =
    int.range(from: 1, to: 10, with: [], run: fn(acc, i) {
      let y = int.to_float(i * 40)
      [make_line([20.0, y, 660.0, y], "#e5e5e5", 1.0), ..acc]
    })
    |> list.reverse
  let frame =
    rect_config.create()
    |> rect_config.with_x(20.0)
    |> rect_config.with_y(20.0)
    |> rect_config.with_width(640.0)
    |> rect_config.with_height(360.0)
    |> rect_config.with_stroke(t.StringOrCanvasGradientString("#b5b5b5"))
    |> rect_config.with_stroke_width(1.5)
    |> rect_config.with_dash([6.0, 4.0])

  let l1 =
    text_config.create()
    |> text_config.with_x(32.0)
    |> text_config.with_y(32.0)
    |> text_config.with_text("Layer 1: Blueprint Grid (z=0)")
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#737373"))

  let _ = layer.add(bg_layer, vlines)
  let _ = layer.add(bg_layer, hlines)
  let _ =
    layer.add(bg_layer, [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(frame)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(l1)))),
    ])

  // Layer 2: Stacked cards & badge.
  let card_a = make_card("card-a", 110.0, 80.0, "#efefef")
  let card_b = make_card("card-b", 210.0, 125.0, "#404040")
  let card_c = make_card("card-c", 310.0, 170.0, "#ffffff")
  let bc =
    circle_config.create()
    |> circle_config.with_id("badge")
    |> circle_config.with_x(555.0)
    |> circle_config.with_y(135.0)
    |> circle_config.with_radius(38.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#111111"))
  let _ =
    layer.add(fg_layer, [
      t.GroupOrShapeShape(shape.widen(card_a)),
      t.GroupOrShapeShape(shape.widen(card_b)),
      t.GroupOrShapeShape(shape.widen(card_c)),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(bc)))),
    ])

  let _ = node.move_to_top(card_a)
  let card_a_z = int.to_string(float.round(node.get_z_index(card_a)))

  let counter = runtime.ref_new(0)
  let _ =
    node.on_click(card_a, fn(_) {
      let _ = runtime.ref_update(counter, fn(n) { n + 1 })
      Nil
    })
  let _ = node.fire(card_a, "click", None, None)
  let _ = node.fire(card_a, "click", None, None)

  let _ = node.draw(bg_layer)
  let _ = node.draw(fg_layer)

  let pt = vector2d.create()
  vector2d.set_x(pt, 250.0)
  vector2d.set_y(pt, 160.0)
  let hit_id = case stage.get_intersection(stg, pt) {
    None -> "none"
    Some(s) ->
      case decode.run(node.get_attr(s, "id", runtime.codec_string()), decode.string) {
        Ok(id_str) -> id_str
        Error(_) -> "unknown"
      }
  }

  let dc =
    circle_config.create()
    |> circle_config.with_x(250.0)
    |> circle_config.with_y(160.0)
    |> circle_config.with_radius(7.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))
    |> circle_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
    |> circle_config.with_stroke_width(2.0)

  let l2 =
    text_config.create()
    |> text_config.with_x(32.0)
    |> text_config.with_y(354.0)
    |> text_config.with_text("Layer 2: Elevated Stack (z=1) · Probe (250,160)")
    |> text_config.with_font_size(12.0)
    |> text_config.with_font_family("monospace")
    |> text_config.with_fill(t.StringOrCanvasGradientString("#111111"))

  let _ =
    layer.add(fg_layer, [
      t.GroupOrShapeShape(shape.widen(circle.new(Some(dc)))),
      make_line([238.0, 160.0, 262.0, 160.0], "#111111", 1.5),
      make_line([250.0, 148.0, 250.0, 172.0], "#111111", 1.5),
      t.GroupOrShapeShape(shape.widen(text.new(Some(l2)))),
    ])
  let _ = node.draw(fg_layer)

  let layers = int.to_string(list.length(stage.get_layers(stg)))
  let clicks = int.to_string(runtime.ref_get(counter))
  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some(
          "Layers: "
          <> layers
          <> " · card-a zIndex="
          <> card_a_z
          <> " · hit@(250,160)="
          <> hit_id
          <> " · clicks="
          <> clicks,
        ),
      )
  }
}
