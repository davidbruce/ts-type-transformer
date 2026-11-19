import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
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
import konva/areas/main/regular_polygon
import konva/areas/main/regular_polygon_config
import konva/areas/main/shape
import konva/areas/main/shape_config
import konva/areas/main/stage
import konva/areas/main/stage_config
import konva/areas/main/text
import konva/areas/main/text_config
import konva/types as t

const pi = 3.141592653589793

fn range(from: Int, to: Int) -> List(Int) {
  case from > to {
    True -> []
    False -> [from, ..range(from + 1, to)]
  }
}

fn wrap_pi(x: Float) -> Float {
  let two_pi = 2.0 *. pi
  case x >. pi {
    True -> wrap_pi(x -. two_pi)
    False ->
      case x <. 0.0 -. pi {
        True -> wrap_pi(x +. two_pi)
        False -> x
      }
  }
}

fn approx_sin(x: Float) -> Float {
  let y = wrap_pi(x)
  let x2 = y *. y
  y *. { 1.0 -. x2 /. 6.0 *. { 1.0 -. x2 /. 20.0 *. { 1.0 -. x2 /. 42.0 } } }
}

fn approx_cos(x: Float) -> Float {
  approx_sin(x +. 0.5 *. pi)
}

fn make_circle(
  x: Float,
  y: Float,
  r: Float,
  stroke_col: String,
  sw: Float,
  fill_opt: Option(String),
) -> t.GroupOrShape {
  let cc = circle_config.create()
  node_config.set_x(cc, Some(x))
  node_config.set_y(cc, Some(y))
  circle_config.set_radius(cc, Some(r))
  shape_config.set_stroke(cc, Some(t.StringOrCanvasGradientString(stroke_col)))
  shape_config.set_stroke_width(cc, Some(sw))
  case fill_opt {
    Some(f) -> shape_config.set_fill(cc, Some(t.StringOrCanvasGradientString(f)))
    None -> Nil
  }
  t.GroupOrShapeShape(shape.widen(circle.new(Some(cc))))
}

fn make_line(pts: List(Float), col: String, sw: Float) -> t.GroupOrShape {
  let lc = line_config.create()
  line_config.set_points(lc, Some(t.LinePointsArray(pts)))
  shape_config.set_stroke(lc, Some(t.StringOrCanvasGradientString(col)))
  shape_config.set_stroke_width(lc, Some(sw))
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc), t.line_config_codec())))
}

pub fn main() -> Nil {
  let cx = 340.0
  let cy = 200.0

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

  let rings = [
    make_circle(cx, cy, 160.0, "#d4d4d4", 1.0, None),
    make_circle(cx, cy, 125.0, "#a3a3a3", 1.0, None),
    make_circle(cx, cy, 90.0, "#555555", 1.0, None),
    make_circle(cx, cy, 52.0, "#111111", 1.5, Some("#f7f7f7")),
  ]
  let spokes =
    list.map(range(0, 11), fn(i) {
      let a = int.to_float(i) *. pi /. 6.0
      make_line(
        [
          cx +. 36.0 *. approx_cos(a),
          cy +. 36.0 *. approx_sin(a),
          cx +. 160.0 *. approx_cos(a),
          cy +. 160.0 *. approx_sin(a),
        ],
        "#d4d4d4",
        1.0,
      )
    })
  let ticks =
    list.map(range(0, 23), fn(i) {
      let a = int.to_float(i) *. pi /. 12.0
      let r0 = case i % 2 == 0 {
        True -> 148.0
        False -> 154.0
      }
      let sw = case i % 2 == 0 {
        True -> 2.0
        False -> 1.0
      }
      make_line(
        [
          cx +. r0 *. approx_cos(a),
          cy +. r0 *. approx_sin(a),
          cx +. 166.0 *. approx_cos(a),
          cy +. 166.0 *. approx_sin(a),
        ],
        "#111111",
        sw,
      )
    })
  let petals =
    list.map(range(0, 7), fn(i) {
      let pc = rect_config.create()
      node_config.set_x(pc, Some(cx))
      node_config.set_y(pc, Some(cy))
      node_config.set_width(pc, Some(18.0))
      node_config.set_height(pc, Some(52.0))
      node_config.set_offset_x(pc, Some(9.0))
      node_config.set_offset_y(pc, Some(108.0))
      node_config.set_rotation(pc, Some(int.to_float(i) *. 45.0))
      let fill_col = case i % 2 == 0 {
        True -> "#111111"
        False -> "#efefef"
      }
      shape_config.set_fill(pc, Some(t.StringOrCanvasGradientString(fill_col)))
      shape_config.set_stroke(pc, Some(t.StringOrCanvasGradientString("#111111")))
      shape_config.set_stroke_width(pc, Some(1.5))
      t.GroupOrShapeShape(shape.widen(rect.new(Some(pc))))
    })
  let nodes =
    list.map(range(0, 11), fn(i) {
      let a = int.to_float(i) *. pi /. 6.0
      let fill_col = case i % 2 == 0 {
        True -> "#111111"
        False -> "#ffffff"
      }
      make_circle(
        cx +. 125.0 *. approx_cos(a),
        cy +. 125.0 *. approx_sin(a),
        5.0,
        "#111111",
        1.5,
        Some(fill_col),
      )
    })

  let cp = regular_polygon_config.create()
  node_config.set_x(cp, Some(cx))
  node_config.set_y(cp, Some(cy))
  regular_polygon_config.set_sides(cp, 8.0)
  regular_polygon_config.set_radius(cp, 26.0)
  shape_config.set_fill(cp, Some(t.StringOrCanvasGradientString("#111111")))

  let cd = circle_config.create()
  node_config.set_x(cd, Some(cx))
  node_config.set_y(cd, Some(cy))
  circle_config.set_radius(cd, Some(9.0))
  shape_config.set_fill(cd, Some(t.StringOrCanvasGradientString("#ffffff")))

  let compass =
    list.map(
      [
        #(516.0, 194.0, "0°"),
        #(332.0, 376.0, "90°"),
        #(140.0, 194.0, "180°"),
        #(328.0, 14.0, "270°"),
      ],
      fn(item) {
        let #(x, y, label) = item
        let tc = text_config.create()
        node_config.set_x(tc, Some(x))
        node_config.set_y(tc, Some(y))
        text_config.set_text(tc, Some(label))
        text_config.set_font_size(tc, Some(11.0))
        text_config.set_font_family(tc, Some("monospace"))
        shape_config.set_fill(tc, Some(t.StringOrCanvasGradientString("#555555")))
        t.GroupOrShapeShape(shape.widen(text.new(Some(tc))))
      },
    )

  let _ = container.add(l, rings, t.group_or_shape_codec())
  let _ = container.add(l, spokes, t.group_or_shape_codec())
  let _ = container.add(l, ticks, t.group_or_shape_codec())
  let _ = container.add(l, petals, t.group_or_shape_codec())
  let _ = container.add(l, nodes, t.group_or_shape_codec())
  let _ = container.add(
    l,
    [
      t.GroupOrShapeShape(shape.widen(regular_polygon.new(Some(cp)))),
      t.GroupOrShapeShape(shape.widen(circle.new(Some(cd)))),
    ],
    t.group_or_shape_codec(),
  )
  let _ = container.add(l, compass, t.group_or_shape_codec())

  let _ = node.draw(l)

  let count = int.to_string(list.length(container.get_children(l, t.group_or_shape_codec())))
  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some("Stage: 680x400 · Procedural rosette · " <> count <> " shapes on layer"),
      )
  }
}
