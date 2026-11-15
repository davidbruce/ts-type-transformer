// Names that would be one Gleam identifier, each given its own. Every check must hold.
import bindings as b
import gleam/io

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  let g = b.gizmo_new()
  check(b.gizmo_is_transforming(g), "method")
  let _ = b.gizmo_get_is_transforming()
  // `fooBar` and `foo_bar` are one `foo_bar`.
  check(b.gizmo_foo_bar(g) == 1.0 && b.gizmo_foo_bar_2(g) == 2.0, "foo_bar")

  // The method `shape_area` keeps its name; the function of that name gives way.
  let s = b.shape()
  check(b.shape_area(s) == 3.0 && b.shape_area_2(s) == 30.0, "shape_area")

  let p = b.point(4.0)
  check(b.point_get_x(p) == 4.0, "point")
  let _ = b.point_get_create(p)

  check(b.ns_node_id(b.ns_node_new()) == 2.0, "class")
  check(b.ns_node2_id(b.ns_node2_new()) == 1.0, "namespace's class")

  io.println("checks passed")
}
