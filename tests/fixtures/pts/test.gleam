// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import gleam/io
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

fn pt(x: Float, y: Float) -> b.Pt {
  b.pt_new([b.dynamic(x), b.dynamic(y)])
}

pub fn main() {
  // A Pt is an array of coordinates, with names for the first four.
  let p = pt(3.0, 4.0)
  check(b.pt_get_x(p) == 3.0 && b.pt_get_y(p) == 4.0, "coordinates")
  check(b.pt_magnitude(p) == 5.0, "magnitude")
  check(b.pt_to_string(p) == "Pt(3, 4)", "toString")
  b.pt_set_id(p, "corner")
  check(b.pt_get_id(p) == "corner", "property")

  // A method that changes a Pt returns it; its `$` twin returns another.
  let moved = b.pt_add_new(p, [b.dynamic(1.0), b.dynamic(1.0)])
  check(b.pt_get_x(moved) == 4.0 && b.pt_get_x(p) == 3.0, "$add returns another")
  let _ = b.pt_add(p, [b.dynamic(1.0), b.dynamic(1.0)])
  check(b.pt_get_x(p) == 4.0, "add changes it")
  check(b.pt_equals(p, b.PtLikePt(moved), None), "union argument")
  check(b.pt_equals(b.pt_clone(p), b.PtLikePt(p), Some(0.001)), "optional argument")

  // Statics.
  check(b.num_lerp(0.0, 10.0, 0.5) == 5.0 && b.num_clamp(12.0, 0.0, 10.0) == 10.0, "statics")
  check(b.geom_to_degree(3.141592653589793) == 180.0, "static")
  check(b.pt_to_string(b.pt_make(3.0, Some(2.0), None)) == "Pt(2, 2, 2)", "optional arguments")

  // A Group is an array of Pt: a list, and indexed.
  let square = b.group_new([pt(0.0, 0.0), pt(4.0, 0.0), pt(4.0, 4.0), pt(0.0, 4.0)])
  check(list_length(b.group_to_list(square)) == 4, "a list")
  let assert Some(third) = b.group_get_at(square, 2.0)
  check(b.pt_get_y(third) == 4.0, "indexed")
  check(b.pt_get_x(b.group_get_p1(square)) == 0.0, "getter")
  check(b.pt_to_string(b.group_centroid(square)) == "Pt(2, 2)", "centroid")
  check(b.pt_get_x(b.group_get_p2(b.group_bounding_box(square))) == 4.0, "bounding box")

  // A union parameter takes any of its members: a Group where points are wanted.
  check(b.pt_get_x(b.geom_centroid(b.PtLikeIterableGroup(square))) == 2.0, "a Group as points")
  check(b.line_magnitude(b.PtIterableGroup(b.group_new([pt(0.0, 0.0), pt(3.0, 4.0)]))) == 5.0, "line")
  let ray = b.line_from_angle(b.PtLikePt(pt(1.0, 1.0)), 0.0, 5.0)
  check(b.pt_get_x(b.group_get_p2(ray)) == 6.0 && b.pt_get_y(b.group_get_p2(ray)) == 1.0, "from angle")

  io.println("checks passed")
}

fn list_length(l: List(a)) -> Int {
  case l {
    [] -> 0
    [_, ..rest] -> 1 + list_length(rest)
  }
}
