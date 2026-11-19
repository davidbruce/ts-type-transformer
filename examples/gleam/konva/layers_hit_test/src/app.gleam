import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/dynamic
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/list
import gleam/option.{None, Some}
import konva/areas/main/circle
import konva/areas/main/circle_config
import konva/areas/main/container
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
import konva/areas/main/vector2d
import konva/runtime
import konva/types as t

fn range(from: Int, to: Int) -> List(Int) {
  case from > to {
    True -> []
    False -> [from, ..range(from + 1, to)]
  }
}

fn make_line(pts: List(Float), col: String, sw: Float) -> t.GroupOrShape {
  let lc = line_config.create()
  line_config.set_points(lc, Some(t.LinePointsArray(pts)))
  shape_config.set_stroke(lc, Some(t.StringOrCanvasGradientString(col)))
  shape_config.set_stroke_width(lc, Some(sw))
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc), t.line_config_codec())))
}

fn make_card(id: String, x: Float, y: Float, fill_hex: String) -> t.Rect(Nil) {
  let rc = rect_config.create()
  node_config.set_id(rc, Some(id))
  node_config.set_name(rc, Some("card"))
  node_config.set_x(rc, Some(x))
  node_config.set_y(rc, Some(y))
  node_config.set_width(rc, Some(200.0))
  node_config.set_height(rc, Some(130.0))
  shape_config.set_fill(rc, Some(t.StringOrCanvasGradientString(fill_hex)))
  shape_config.set_stroke(rc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(rc, Some(2.0))
  rect.new(Some(rc))
}

fn read_clicks(counter: dynamic.Dynamic) -> Int {
  case runtime.js_get(counter, "n") {
    Some(d) ->
      case decode.run(d, decode.int) {
        Ok(v) -> v
        Error(_) -> 0
      }
    None -> 0
  }
}

pub fn main() -> Nil {
  let sc = stage_config.create()
  stage_config.set_container(sc, Some(t.HTMLDivElementOrStringString("stage")))
  node_config.set_width(sc, Some(680.0))
  node_config.set_height(sc, Some(400.0))
  let stg = stage.new(sc)

  let bg_layer = layer.new(None)
  let fg_layer = layer.new(None)
  let _ = stage.add(stg, bg_layer, [])
  let _ = stage.add(stg, fg_layer, [])

  // Layer 1: Blueprint Grid.
  let bgr = rect_config.create()
  node_config.set_x(bgr, Some(0.0))
  node_config.set_y(bgr, Some(0.0))
  node_config.set_width(bgr, Some(680.0))
  node_config.set_height(bgr, Some(400.0))
  shape_config.set_fill(bgr, Some(t.StringOrCanvasGradientString("#ffffff")))
  let _ = container.add(
    bg_layer,
    [t.GroupOrShapeShape(shape.widen(rect.new(Some(bgr))))],
    t.group_or_shape_codec(),
  )

  let vlines =
    list.map(range(1, 16), fn(i) {
      let x = int.to_float(i * 40)
      make_line([x, 20.0, x, 380.0], "#e5e5e5", 1.0)
    })
  let hlines =
    list.map(range(1, 9), fn(i) {
      let y = int.to_float(i * 40)
      make_line([20.0, y, 660.0, y], "#e5e5e5", 1.0)
    })
  let frame = rect_config.create()
  node_config.set_x(frame, Some(20.0))
  node_config.set_y(frame, Some(20.0))
  node_config.set_width(frame, Some(640.0))
  node_config.set_height(frame, Some(360.0))
  shape_config.set_stroke(frame, Some(t.StringOrCanvasGradientString("#b5b5b5")))
  shape_config.set_stroke_width(frame, Some(1.5))
  shape_config.set_dash(frame, Some([6.0, 4.0]))

  let l1 = text_config.create()
  node_config.set_x(l1, Some(32.0))
  node_config.set_y(l1, Some(32.0))
  text_config.set_text(l1, Some("Layer 1: Blueprint Grid (z=0)"))
  text_config.set_font_size(l1, Some(12.0))
  text_config.set_font_family(l1, Some("monospace"))
  shape_config.set_fill(l1, Some(t.StringOrCanvasGradientString("#737373")))

  let _ = container.add(bg_layer, vlines, t.group_or_shape_codec())
  let _ = container.add(bg_layer, hlines, t.group_or_shape_codec())
  let _ = container.add(
    bg_layer,
    [
      t.GroupOrShapeShape(shape.widen(rect.new(Some(frame)))),
      t.GroupOrShapeShape(shape.widen(text.new(Some(l1)))),
    ],
    t.group_or_shape_codec(),
  )

  // Layer 2: Stacked cards & badge.
  let card_a = make_card("card-a", 110.0, 80.0, "#efefef")
  let card_b = make_card("card-b", 210.0, 125.0, "#404040")
  let card_c = make_card("card-c", 310.0, 170.0, "#ffffff")
  let bc = circle_config.create()
  node_config.set_id(bc, Some("badge"))
  node_config.set_x(bc, Some(555.0))
  node_config.set_y(bc, Some(135.0))
  circle_config.set_radius(bc, Some(38.0))
  shape_config.set_fill(bc, Some(t.StringOrCanvasGradientString("#111111")))
  let _ = container.add(
    fg_layer,
    [
      t.GroupOrShapeShape(shape.widen(card_a)),
      t.GroupOrShapeShape(shape.widen(card_b)),
      t.GroupOrShapeShape(shape.widen(card_c)),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(bc)))),
    ],
    t.group_or_shape_codec(),
  )

  let _ = node.move_to_top(card_a)
  let card_a_z = int.to_string(float.round(node.get_z_index(card_a)))

  let counter = runtime.js_object([#("n", dynamic.int(0))])
  let _ = node.on_click(
    card_a,
    fn(_) { runtime.js_set(counter, "n", dynamic.int(read_clicks(counter) + 1)) },
    t.rect_config_codec(),
  )
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

  let dc = circle_config.create()
  node_config.set_x(dc, Some(250.0))
  node_config.set_y(dc, Some(160.0))
  circle_config.set_radius(dc, Some(7.0))
  shape_config.set_fill(dc, Some(t.StringOrCanvasGradientString("#ffffff")))
  shape_config.set_stroke(dc, Some(t.StringOrCanvasGradientString("#111111")))
  shape_config.set_stroke_width(dc, Some(2.0))

  let l2 = text_config.create()
  node_config.set_x(l2, Some(32.0))
  node_config.set_y(l2, Some(354.0))
  text_config.set_text(l2, Some("Layer 2: Elevated Stack (z=1) · Probe (250,160)"))
  text_config.set_font_size(l2, Some(12.0))
  text_config.set_font_family(l2, Some("monospace"))
  shape_config.set_fill(l2, Some(t.StringOrCanvasGradientString("#111111")))

  let _ = container.add(
    fg_layer,
    [
      t.GroupOrShapeShape(shape.widen(circle.new(Some(dc)))),
      make_line([238.0, 160.0, 262.0, 160.0], "#111111", 1.5),
      make_line([250.0, 148.0, 250.0, 172.0], "#111111", 1.5),
      t.GroupOrShapeShape(shape.widen(text.new(Some(l2)))),
    ],
    t.group_or_shape_codec(),
  )
  let _ = node.draw(fg_layer)

  let layers = int.to_string(list.length(stage.get_layers(stg)))
  let clicks = int.to_string(read_clicks(counter))
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
