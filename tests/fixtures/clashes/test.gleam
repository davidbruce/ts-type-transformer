// Names that would be one Gleam identifier, each given its own. Every check must hold.
import bindings as b
import gleam/io
import bindings/gizmo
import bindings/ns_node
import bindings/point
import bindings/shape
import bindings/namespaces/ns
import bindings/namespaces/ns/node

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  let g = gizmo.new()
  check(!gizmo.is_transforming_2(), "static")
  check(gizmo.is_transforming(g), "method")
  // `fooBar` and `foo_bar` are one `foo_bar`.
  check(gizmo.foo_bar(g) == 1.0 && gizmo.foo_bar_2(g) == 2.0, "foo_bar")

  // The function `shape_area` and the method `Shape.area` are in modules
  // of their own: neither gives way.
  let s = b.shape()
  check(shape.area(s) == 3.0 && b.shape_area(s) == 30.0, "shape_area")

  let p = b.point(4.0)
  check(point.get_x(p) == 4.0, "point")
  check(point.get_x(point.create_2(p, 5.0)) == 5.0, "property create")

  check(ns_node.id(ns_node.new()) == 2.0, "class")
  // The namespace's class: its functions in a module of its own name, its
  // type by its own name in the namespace's.
  let n: ns.Node(Nil) = node.new()
  check(node.id(n) == 1.0, "namespace's class")

  io.println("checks passed")
}
