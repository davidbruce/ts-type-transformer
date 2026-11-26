// Konva from Gleam, in a page: a stage on the page's element, a layer, shapes,
// groups, finding, events, JSON, destroying. Every check must hold.
//
import bindings/areas/konva/any_string
import bindings/areas/konva/circle
import bindings/areas/konva/circle_config
import bindings/areas/konva/container
import bindings/areas/konva/get_set
import bindings/areas/konva/group
import bindings/areas/konva/group_config
import bindings/areas/konva/i_rect
import bindings/areas/konva/konva_event_object
import bindings/areas/konva/layer
import bindings/areas/konva/node
import bindings/areas/konva/node_config
import bindings/areas/konva/rect
import bindings/areas/konva/rect_config
import bindings/areas/konva/shape
import bindings/areas/konva/stage
import bindings/areas/konva/stage_config
import bindings/areas/konva/text
import bindings/areas/konva/text_config
import bindings/areas/konva/vector2d
import bindings/string_or_canvas_gradient
import bindings/node_get_absolute_position_result as pos
import bindings/runtime
import cell
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn near(a: Float, b: Float) -> Bool {
  float.absolute_value(a -. b) <. 0.001
}

fn attr_string(d, name: String) -> String {
  let assert Ok(s) = decode.run(d, decode.string)
  let _ = name
  s
}

pub fn main() {
  // A stage on the page's #stage, 400 by 300, with a layer.
  let cfg = stage_config.create()
  stage_config.set_container(cfg, "stage")
  node_config.set_width(cfg, Some(400.0))
  node_config.set_height(cfg, Some(300.0))
  let s = stage.new(cfg)
  let l = layer.new()
  let _ = stage.add(s, l, [])
  let passed =
    check(list.length(stage.get_layers(s)) == 1, "the stage has a layer")
    + check(option.is_some(node.get_stage(l)), "the layer knows its stage")
    + check(
      node.get_class_name(s) == "Stage" && node.get_class_name(l) == "Layer",
      "class names",
    )

  // A rectangle from a config, with an id and a name.
  let rc = rect_config.create()
  rect_config.set_x(rc, Some(20.0))
  rect_config.set_y(rc, Some(30.0))
  rect_config.set_width(rc, Some(100.0))
  rect_config.set_height(rc, Some(50.0))
  rect_config.set_fill(rc, "red")
  rect_config.set_id(rc, Some("box"))
  rect_config.set_name(rc, Some("shape"))
  let r = rect.new_with(rc)
  let _ = container.add(l, [r])
  let at = node.get_position(r)
  let client = node.get_client_rect(r)
  let passed =
    passed
    + check(pos.get_x(at) == 20.0 && pos.get_y(at) == 30.0, "position")
    + check(
      i_rect.get_x(client) == 20.0 && i_rect.get_y(client) == 30.0,
      "client rect position",
    )
    + check(
      i_rect.get_width(client) == 100.0 && i_rect.get_height(client) == 50.0,
      "client rect size",
    )
    + check(node.get_class_name(r) == "Rect", "class name of a rect")
    + check(
      get_set.invoke(node.get_x(r), runtime.codec_float()) == 20.0,
      "the x property",
    )
    + check(
      get_set.invoke(node.get_y(r), runtime.codec_float()) == 30.0,
      "the y property",
    )
    + check(
      get_set.invoke(node.get_width(r), runtime.codec_float()) == 100.0,
      "the width property",
    )
    + check(
      get_set.invoke(shape.get_fill(r), string_or_canvas_gradient.codec())
        == string_or_canvas_gradient.String("red"),
      "the fill property",
    )
    + check(node.get_parent(r) |> option.is_some, "the rect has a parent")
    + check(option.is_some(node.get_layer(r)), "the rect knows its layer")
    + check(
      decode.run(node_config.get_at(node.get_attrs(r), "fill"), decode.string)
        == Ok("red"),
      "fill in the attrs",
    )
    + check(
      node_config.get_id(node.get_attrs(r)) == Some("box"),
      "id in the attrs",
    )

  // A circle and a text, and a group holding both.
  let cc = circle_config.create()
  node_config.set_x(cc, Some(10.0))
  node_config.set_y(cc, Some(10.0))
  circle_config.set_radius(cc, Some(25.0))
  node_config.set_name(cc, Some("shape"))
  let c = circle.new_with(cc)
  let tc = text_config.create()
  text_config.set_text(tc, Some("Hello Konva"))
  text_config.set_font_size(tc, Some(20.0))
  node_config.set_y(tc, Some(100.0))
  let tx = text.new_with(tc)
  let gc = group_config.create()
  node_config.set_x(gc, Some(200.0))
  node_config.set_y(gc, Some(100.0))
  node_config.set_name(gc, Some("cluster"))
  let g = group.new_with(gc)
  let _ = container.add(g, [node.upcast(c), node.upcast(tx)])
  let _ = container.add(l, [g])
  let circle_at = node.get_absolute_position(c)
  let text_rect = node.get_client_rect(tx)
  let passed =
    passed
    + check(
      list.length(container.get_children(g)) == 2,
      "the group has two children",
    )
    + check(
      list.length(container.get_children(l)) == 2,
      "the layer has two children",
    )
    + check(
      pos.get_x(circle_at) == 210.0 && pos.get_y(circle_at) == 110.0,
      "absolute position adds the group's",
    )
    + check(pos.get_x(node.get_position(c)) == 10.0, "own position does not")
    + check(
      i_rect.get_width(text_rect) >. 20.0
        && i_rect.get_height(text_rect) >. 10.0,
      "a text has a size",
    )
    + check(
      attr_string(node.get_attr(tx, any_string.String("text")), "text")
        == "Hello Konva",
      "the text's attr",
    )

  // Finding by selector: id, name, type.
  let by_id = container.find_one(l, "#box")
  let by_name = container.find(l, runtime.dynamic(".shape"))
  let rects = container.find(l, runtime.dynamic("Rect"))
  let passed =
    passed
    + check(option.is_some(by_id), "find_one by id")
    + check(list.length(by_name) == 2, "find by name reaches into the group")
    + check(list.length(rects) == 1, "find by type")
    + check(
      container.find(l, runtime.dynamic("#nothing")) == [],
      "nothing found",
    )
    + check(
      option.is_some(container.find_one(l, ".cluster")),
      "find a group by name",
    )

  // Moving and rotating.
  let by = vector2d.create()
  vector2d.set_x(by, 5.0)
  vector2d.set_y(by, -10.0)
  let _ = node.move(r, by)
  let _ = node.rotate(r, 90.0)
  let passed =
    passed
    + check(
      pos.get_x(node.get_position(r)) == 25.0
        && pos.get_y(node.get_position(r)) == 20.0,
      "move",
    )
    + check(near(node.get_absolute_rotation(r), 90.0), "rotate")
  let to = vector2d.create()
  vector2d.set_x(to, 1.0)
  vector2d.set_y(to, 2.0)
  let _ = node.set_position(r, to)
  let passed =
    passed
    + check(
      pos.get_x(node.get_position(r)) == 1.0
        && pos.get_y(node.get_position(r)) == 2.0,
      "set_position",
    )

  // Events: a click fired on the rect, with its target.
  let clicks = cell.new(0)
  let _ = node.on_click(r, fn(_) { cell.set(clicks, cell.get(clicks) + 1) })
  let _ = node.fire(r, "click")
  let _ = node.fire(r, "click")
  let passed =
    passed + check(cell.get(clicks) == 2, "the click listener ran twice")
  let seen = cell.new("")
  let _ =
    node.on_mouseover(c, fn(e) { cell.set(seen, "over " <> konva_type(e)) })
  let _ = node.fire(c, "mouseover")
  let passed =
    passed
    + check(cell.get(seen) == "over mouseover", "the event object's type")

  // Visibility.
  let _ = node.hide(r)
  let hidden = !node.is_visible_2(r)
  let _ = node.show(r)
  let passed = passed + check(hidden && node.is_visible_2(r), "hide and show")

  // Drawing, then the hit canvas finds the shape under a point.
  let _ = node.draw(l)
  let over_circle = vector2d.create()
  vector2d.set_x(over_circle, 210.0)
  vector2d.set_y(over_circle, 110.0)
  let hit = stage.get_intersection(s, over_circle)
  let passed =
    passed
    + check(option.is_some(hit), "a shape is under the point")
    + check(
      case hit {
        Some(shape) -> node.get_class_name(shape) == "Circle"
        None -> False
      },
      "and it is the circle",
    )

  // JSON and clone.
  let json = node.to_json(r)
  let passed =
    passed
    + check(
      string.contains(json, "\"className\":\"Rect\""),
      "toJSON names the class",
    )
    + check(string.contains(json, "\"id\":\"box\""), "toJSON holds the id")
  let layer_json = node.to_json(l)
  let passed =
    passed
    + check(
      string.contains(layer_json, "\"className\":\"Layer\"")
        && string.contains(layer_json, "Circle"),
      "a layer's JSON holds its children",
    )

  // Destroying removes the node from its parent.
  let _ = node.destroy(c)
  let passed =
    passed
    + check(
      container.find(l, runtime.dynamic("Circle")) == [],
      "a destroyed circle is gone",
    )
    + check(
      list.length(container.get_children(g)) == 1,
      "the group has one child left",
    )
  let _ = container.destroy_children(l)
  let passed =
    passed
    + check(!container.has_children(l), "no children after destroy_children")
  let _ = stage.destroy(s)

  io.println(int.to_string(passed) <> " checks passed")
}

fn konva_type(e) -> String {
  konva_event_object.get_type(e)
}
