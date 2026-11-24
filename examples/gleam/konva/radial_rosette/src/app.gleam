import dom/areas/core
import dom/areas/core/document
import dom/areas/core/node as dom_node
import gleam/int
import gleam/list
import gleam/option.{type Option, None, Some}
import konva/circle
import konva/circle_config
import konva/layer
import konva/line
import konva/line_config
import konva/node
import konva/rect
import konva/rect_config
import konva/regular_polygon
import konva/regular_polygon_config
import konva/shape
import konva/stage
import konva/stage_config
import konva/text
import konva/text_config
import konva/types as t

const pi = 3.141592653589793

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
  let cc =
    circle_config.create()
    |> circle_config.with_x(x)
    |> circle_config.with_y(y)
    |> circle_config.with_radius(r)
    |> circle_config.with_stroke(t.StringOrCanvasGradientString(stroke_col))
    |> circle_config.with_stroke_width(sw)
  let cc = case fill_opt {
    Some(f) -> circle_config.with_fill(cc, t.StringOrCanvasGradientString(f))
    None -> cc
  }
  t.GroupOrShapeShape(shape.widen(circle.new(Some(cc))))
}

fn make_line(pts: List(Float), col: String, sw: Float) -> t.GroupOrShape {
  let lc =
    line_config.create()
    |> line_config.with_points(t.LinePointsArray(pts))
    |> line_config.with_stroke(t.StringOrCanvasGradientString(col))
    |> line_config.with_stroke_width(sw)
  t.GroupOrShapeShape(shape.widen(line.new(Some(lc))))
}

pub fn main() -> Nil {
  let cx = 340.0
  let cy = 200.0

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
  layer.add(l, [t.GroupOrShapeShape(shape.widen(rect.new(Some(bg))))])

  let rings = [
    make_circle(cx, cy, 160.0, "#d4d4d4", 1.0, None),
    make_circle(cx, cy, 125.0, "#a3a3a3", 1.0, None),
    make_circle(cx, cy, 90.0, "#555555", 1.0, None),
    make_circle(cx, cy, 52.0, "#111111", 1.5, Some("#f7f7f7")),
  ]
  let spokes =
    int.range(from: 0, to: 12, with: [], run: fn(acc, i) {
      let a = int.to_float(i) *. pi /. 6.0
      [
        make_line(
          [
            cx +. 36.0 *. approx_cos(a),
            cy +. 36.0 *. approx_sin(a),
            cx +. 160.0 *. approx_cos(a),
            cy +. 160.0 *. approx_sin(a),
          ],
          "#d4d4d4",
          1.0,
        ),
        ..acc
      ]
    })
    |> list.reverse
  let ticks =
    int.range(from: 0, to: 24, with: [], run: fn(acc, i) {
      let a = int.to_float(i) *. pi /. 12.0
      let r0 = case i % 2 == 0 {
        True -> 148.0
        False -> 154.0
      }
      let sw = case i % 2 == 0 {
        True -> 2.0
        False -> 1.0
      }
      [
        make_line(
          [
            cx +. r0 *. approx_cos(a),
            cy +. r0 *. approx_sin(a),
            cx +. 166.0 *. approx_cos(a),
            cy +. 166.0 *. approx_sin(a),
          ],
          "#111111",
          sw,
        ),
        ..acc
      ]
    })
    |> list.reverse
  let petals =
    int.range(from: 0, to: 8, with: [], run: fn(acc, i) {
      let fill_col = case i % 2 == 0 {
        True -> "#111111"
        False -> "#efefef"
      }
      let pc =
        rect_config.create()
        |> rect_config.with_x(cx)
        |> rect_config.with_y(cy)
        |> rect_config.with_width(18.0)
        |> rect_config.with_height(52.0)
        |> rect_config.with_offset_x(9.0)
        |> rect_config.with_offset_y(108.0)
        |> rect_config.with_rotation(int.to_float(i) *. 45.0)
        |> rect_config.with_fill(t.StringOrCanvasGradientString(fill_col))
        |> rect_config.with_stroke(t.StringOrCanvasGradientString("#111111"))
        |> rect_config.with_stroke_width(1.5)
      [t.GroupOrShapeShape(shape.widen(rect.new(Some(pc)))), ..acc]
    })
    |> list.reverse
  let nodes =
    int.range(from: 0, to: 12, with: [], run: fn(acc, i) {
      let a = int.to_float(i) *. pi /. 6.0
      let fill_col = case i % 2 == 0 {
        True -> "#111111"
        False -> "#ffffff"
      }
      [
        make_circle(
          cx +. 125.0 *. approx_cos(a),
          cy +. 125.0 *. approx_sin(a),
          5.0,
          "#111111",
          1.5,
          Some(fill_col),
        ),
        ..acc
      ]
    })
    |> list.reverse

  let cp =
    regular_polygon_config.create()
    |> regular_polygon_config.with_x(cx)
    |> regular_polygon_config.with_y(cy)
    |> regular_polygon_config.with_sides(8.0)
    |> regular_polygon_config.with_radius(26.0)
    |> regular_polygon_config.with_fill(t.StringOrCanvasGradientString(
      "#111111",
    ))

  let cd =
    circle_config.create()
    |> circle_config.with_x(cx)
    |> circle_config.with_y(cy)
    |> circle_config.with_radius(9.0)
    |> circle_config.with_fill(t.StringOrCanvasGradientString("#ffffff"))

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
        let tc =
          text_config.create()
          |> text_config.with_x(x)
          |> text_config.with_y(y)
          |> text_config.with_text(label)
          |> text_config.with_font_size(11.0)
          |> text_config.with_font_family("monospace")
          |> text_config.with_fill(t.StringOrCanvasGradientString("#555555"))
        t.GroupOrShapeShape(shape.widen(text.new(Some(tc))))
      },
    )

  layer.add(l, rings)
  layer.add(l, spokes)
  layer.add(l, ticks)
  layer.add(l, petals)
  layer.add(l, nodes)
  layer.add(l, [
    t.GroupOrShapeShape(shape.widen(regular_polygon.new(Some(cp)))),
    t.GroupOrShapeShape(shape.widen(circle.new(Some(cd)))),
  ])
  layer.add(l, compass)

  node.draw(l)

  let count = int.to_string(list.length(layer.get_children(l, None)))
  let doc = core.get_document()
  case document.get_element_by_id(doc, "meta") {
    None -> Nil
    Some(el) ->
      dom_node.set_text_content(
        el,
        Some(
          "Stage: 680x400 · Procedural rosette · "
          <> count
          <> " shapes on layer",
        ),
      )
  }
}
