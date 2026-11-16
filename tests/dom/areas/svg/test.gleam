// SVG from Gleam: elements made in the SVG namespace, their attributes,
// animated lengths, geometry (getBBox, path length) and the viewBox.
import bindings/areas/canvas/dom_matrix_read_only
import bindings/areas/core as dom
import bindings/areas/core/document
import bindings/areas/core/dom_point_init
import bindings/areas/core/dom_point_read_only
import bindings/areas/core/dom_rect_read_only
import bindings/areas/core/dom_token_list
import bindings/areas/core/element
import bindings/areas/core/node
import bindings/areas/css/css_style_properties as props
import bindings/areas/svg/svg_animated_length as animated
import bindings/areas/svg/svg_animated_rect
import bindings/areas/svg/svg_circle_element
import bindings/areas/svg/svg_geometry_element
import bindings/areas/svg/svg_graphics_element
import bindings/areas/svg/svg_length
import bindings/areas/svg/svg_rect_element
import bindings/areas/svg/svgsvg_element
import bindings/areas/html/html_collection_base as collection
import cell
import gleam/float
import gleam/int
import gleam/io
import gleam/option.{None, Some}
import gleam/string

const svg_ns = "http://www.w3.org/2000/svg"

fn near(a: Float, b: Float) -> Bool {
  float.absolute_value(a -. b) <. 0.01
}

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)

  // The <svg> root, in the page.
  let svg = document.create_element_ns_svg_svg(doc)
  element.set_attribute(svg, "width", "200")
  element.set_attribute(svg, "height", "100")
  element.set_attribute(svg, "viewBox", "0 0 400 200")
  node.append_child(body, svg)
  check(element.get_namespace_uri(svg) == Some(svg_ns), "svg namespace")
  check(element.get_local_name(svg) == "svg" && element.get_tag_name(svg) == "svg", "svg names")
  check(node.get_is_connected(svg), "svg is in the page")
  check(animated.get_base_val(svgsvg_element.get_width(svg)) |> svg_length.get_value == 200.0, "svg width")
  check(animated.get_base_val(svgsvg_element.get_height(svg)) |> svg_length.get_value == 100.0, "svg height")

  // The viewBox.
  let view = svg_animated_rect.get_base_val(svgsvg_element.get_view_box(svg))
  check(dom_rect_read_only.get_x(view) == 0.0 && dom_rect_read_only.get_y(view) == 0.0, "viewBox origin")
  check(dom_rect_read_only.get_width(view) == 400.0 && dom_rect_read_only.get_height(view) == 200.0, "viewBox size")
  check(dom_rect_read_only.get_width(svg_animated_rect.get_anim_val(svgsvg_element.get_view_box(svg))) == 400.0, "animVal viewBox")

  // A rect with attributes, and its animated lengths.
  let rect = document.create_element_ns_svg_rect(doc)
  element.set_attribute(rect, "x", "10")
  element.set_attribute(rect, "y", "20")
  element.set_attribute(rect, "width", "100")
  element.set_attribute(rect, "height", "50")
  element.set_attribute(rect, "fill", "red")
  node.append_child(svg, rect)
  let width = animated.get_base_val(svg_rect_element.get_width(rect))
  check(svg_length.get_value(width) == 100.0, "rect width value")
  check(svg_length.get_unit_type(width) == svg_length.svg_lengthtype_number, "unitless is a number")
  check(svg_length.get_value_as_string(width) == "100", "value as string")
  check(svg_length.get_value(animated.get_anim_val(svg_rect_element.get_width(rect))) == 100.0, "animVal length")
  check(svg_length.get_value(animated.get_base_val(svg_rect_element.get_x(rect))) == 10.0, "rect x")
  svg_length.set_value(width, 120.0)
  check(element.get_attribute(rect, "width") == Some("120"), "setting a length writes the attribute")
  element.set_attribute(rect, "height", "40")
  check(svg_length.get_value(animated.get_base_val(svg_rect_element.get_height(rect))) == 40.0, "an attribute change shows in the length")
  svg_length.set_value_as_string(width, "2em")
  check(svg_length.get_unit_type(width) == svg_length.svg_lengthtype_ems, "em unit")
  check(svg_length.get_value_in_specified_units(width) == 2.0, "specified units")
  check(element.get_attribute(rect, "width") == Some("2em"), "unit written back")
  svg_length.set_value_as_string(width, "10%")
  check(svg_length.get_unit_type(width) == svg_length.svg_lengthtype_percentage, "percentage unit")
  svg_length.set_value_as_string(width, "1in")
  check(near(svg_length.get_value(width), 96.0), "an inch is 96 user units")
  svg_length.convert_to_specified_units(width, svg_length.svg_lengthtype_px)
  check(svg_length.get_unit_type(width) == svg_length.svg_lengthtype_px && near(svg_length.get_value_in_specified_units(width), 96.0), "converted to px")
  svg_length.new_value_specified_units(width, svg_length.svg_lengthtype_number, 100.0)
  check(element.get_attribute(rect, "width") == Some("100"), "back to 100")
  element.set_attribute(rect, "height", "50")

  // getBBox of a shape, and of a group.
  let box = svg_graphics_element.get_b_box(rect, None)
  check(
    dom_rect_read_only.get_x(box) == 10.0 && dom_rect_read_only.get_y(box) == 20.0
      && dom_rect_read_only.get_width(box) == 100.0 && dom_rect_read_only.get_height(box) == 50.0,
    "rect bbox",
  )
  let group = document.create_element_ns_svg_g(doc)
  element.set_attribute(group, "transform", "translate(20 30)")
  node.append_child(svg, group)
  let circle = document.create_element_ns_svg_circle(doc)
  element.set_attribute(circle, "cx", "150")
  element.set_attribute(circle, "cy", "60")
  element.set_attribute(circle, "r", "30")
  node.append_child(group, circle)
  check(svg_length.get_value(animated.get_base_val(svg_circle_element.get_r(circle))) == 30.0, "circle radius")
  let circle_box = svg_graphics_element.get_b_box(circle, None)
  check(dom_rect_read_only.get_x(circle_box) == 120.0 && dom_rect_read_only.get_width(circle_box) == 60.0, "circle bbox")
  node.append_child(group, node.clone_node(rect, Some(True)))
  let group_box = svg_graphics_element.get_b_box(group, None)
  check(
    dom_rect_read_only.get_x(group_box) == 10.0 && dom_rect_read_only.get_y(group_box) == 20.0
      && dom_rect_read_only.get_width(group_box) == 170.0 && dom_rect_read_only.get_height(group_box) == 70.0,
    "group bbox is the union (in its own coordinates)",
  )
  // The browser approximates a circle by curves, so its length is near, not exact.
  let circumference = svg_geometry_element.get_total_length(circle)
  check(float.absolute_value(circumference -. 188.4956) <. 0.5, "circle circumference: " <> float.to_string(circumference))

  // Transforms: the viewBox halves, the group moves.
  let assert Some(ctm) = svg_graphics_element.get_ctm(circle)
  check(near(dom_matrix_read_only.get_a(ctm), 0.5), "scale from the viewBox")
  check(near(dom_matrix_read_only.get_e(ctm), 10.0) && near(dom_matrix_read_only.get_f(ctm), 15.0), "translation of the group, scaled")
  let assert Some(screen) = svg_graphics_element.get_screen_ctm(rect)
  check(near(dom_matrix_read_only.get_a(screen), 0.5), "screen CTM scale")
  let on_screen = element.get_bounding_client_rect(rect)
  check(near(dom_rect_read_only.get_width(on_screen), 50.0) && near(dom_rect_read_only.get_height(on_screen), 25.0), "client rect is in screen pixels")

  // A path: length, points along it, hit tests.
  let path = document.create_element_ns_svg_path(doc)
  element.set_attribute(path, "d", "M0 0 L100 0 L100 100 Z")
  element.set_attribute(path, "fill", "blue")
  element.set_attribute(path, "stroke", "black")
  node.append_child(svg, path)
  let expected = 200.0 +. float_sqrt(20000.0)
  check(near(svg_geometry_element.get_total_length(path), expected), "path length is the perimeter")
  let halfway = svg_geometry_element.get_point_at_length(path, 50.0)
  check(near(dom_point_read_only.get_x(halfway), 50.0) && near(dom_point_read_only.get_y(halfway), 0.0), "point at length 50")
  let corner = svg_geometry_element.get_point_at_length(path, 150.0)
  check(near(dom_point_read_only.get_x(corner), 100.0) && near(dom_point_read_only.get_y(corner), 50.0), "point on the second side")
  let inside = dom_point_init.create()
  dom_point_init.set_x(inside, Some(90.0))
  dom_point_init.set_y(inside, Some(10.0))
  check(svg_geometry_element.is_point_in_fill(path, Some(inside)), "a point inside the triangle")
  let outside = dom_point_init.create()
  dom_point_init.set_x(outside, Some(10.0))
  dom_point_init.set_y(outside, Some(90.0))
  check(!svg_geometry_element.is_point_in_fill(path, Some(outside)), "a point outside the triangle")
  let on_edge = dom_point_init.create()
  dom_point_init.set_x(on_edge, Some(50.0))
  dom_point_init.set_y(on_edge, Some(0.0))
  check(svg_geometry_element.is_point_in_stroke(path, Some(on_edge)), "a point on the stroke")
  check(!svg_geometry_element.is_point_in_stroke(path, Some(inside)), "a point off the stroke")
  element.set_attribute(path, "d", "M0 0 H50")
  check(near(svg_geometry_element.get_total_length(path), 50.0), "a new d changes the length")

  // The generic constructor, and downcasts from Element.
  let made = document.create_element_ns(doc, Some(svg_ns), "circle", None)
  check(element.get_namespace_uri(made) == Some(svg_ns), "createElementNS namespace")
  check(svg_circle_element.from(made) |> is_ok, "an Element made as a circle is an SVGCircleElement")
  check(svg_rect_element.from(made) |> is_error, "but not an SVGRectElement")
  check(svg_rect_element.from(document.create_element_div(doc, None)) |> is_error, "an HTML div is no SVGRectElement")
  let html_circle = document.create_element_ns(doc, Some("http://www.w3.org/1999/xhtml"), "circle", None)
  check(svg_circle_element.from(html_circle) |> is_error, "the same name in the HTML namespace is not SVG")
  check(svgsvg_element.from(svg) |> is_ok, "the root downcasts")

  // Tree, classes and styles apply to SVG as to HTML.
  check(collection.get_length(element.get_children(svg)) == 3.0, "svg has three children")
  check(element.matches(rect, "svg > rect[fill=red]"), "selector matches")
  let assert Some(found) = element.query_selector(svg, "g > circle")
  check(element.get_attribute(found, "r") == Some("30"), "querySelector finds the circle")
  dom_token_list.add(element.get_class_list(rect), ["hot", "big"])
  check(element.get_attribute(rect, "class") == Some("hot big"), "classList on SVG")
  check(string.contains(element.get_outer_html(rect), "<rect "), "outerHTML")
  check(props.get_fill(dom.get_computed_style(rect, None)) == "rgb(255, 0, 0)", "computed fill")
  element.set_attribute(rect, "style", "fill: green")
  check(props.get_fill(dom.get_computed_style(rect, None)) == "rgb(0, 128, 0)", "style beats the attribute")

  // Plain SVG factory results.
  let length = svgsvg_element.create_svg_length(svg)
  svg_length.set_value(length, 5.0)
  check(svg_length.get_value(length) == 5.0, "createSVGLength")
  let rect2 = svgsvg_element.create_svg_rect(svg)
  check(dom_rect_read_only.get_width(rect2) == 0.0, "createSVGRect is empty")
  element.set_attribute(svg, "viewBox", "0 0 100 50")
  check(dom_rect_read_only.get_width(svg_animated_rect.get_base_val(svgsvg_element.get_view_box(svg))) == 100.0, "viewBox follows its attribute")
  node.remove_child(svg, rect)
  check(collection.get_length(element.get_children(svg)) == 2.0, "removed from the svg")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}

fn float_sqrt(x: Float) -> Float {
  let assert Ok(r) = float.square_root(x)
  r
}

fn is_ok(r: Result(a, b)) -> Bool {
  case r {
    Ok(_) -> True
    Error(_) -> False
  }
}

fn is_error(r: Result(a, b)) -> Bool {
  !is_ok(r)
}
