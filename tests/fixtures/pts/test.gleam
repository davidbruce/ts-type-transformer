// Calls the generated Gleam bindings; every check must hold.
import bindings/runtime as r
import bindings/types as t
import gleam/io
import gleam/option.{None, Some}
import bindings/areas/main/geom
import bindings/areas/main/group
import bindings/areas/main/line
import bindings/areas/main/num
import bindings/areas/main/pt

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

fn pt(x: Float, y: Float) -> t.Pt(Nil) {
  pt.new([r.dynamic(x), r.dynamic(y)])
}

pub fn main() {
  // A Pt is an array of coordinates, with names for the first four.
  let p = pt(3.0, 4.0)
  check(pt.get_x(p) == 3.0 && pt.get_y(p) == 4.0, "coordinates")
  check(pt.magnitude(p) == 5.0, "magnitude")
  check(pt.to_string(p) == "Pt(3, 4)", "toString")
  pt.set_id(p, "corner")
  check(pt.get_id(p) == "corner", "property")

  // A method that changes a Pt returns it; its `$` twin returns another.
  let moved = pt.add_new(p, [r.dynamic(1.0), r.dynamic(1.0)])
  check(pt.get_x(moved) == 4.0 && pt.get_x(p) == 3.0, "$add returns another")
  let _ = pt.add(p, [r.dynamic(1.0), r.dynamic(1.0)])
  check(pt.get_x(p) == 4.0, "add changes it")
  check(pt.equals(p, t.PtLikePt(moved), None), "union argument")
  check(pt.equals(pt.clone(p), t.PtLikePt(p), Some(0.001)), "optional argument")

  // Statics.
  check(num.lerp(0.0, 10.0, 0.5) == 5.0 && num.clamp(12.0, 0.0, 10.0) == 10.0, "statics")
  check(geom.to_degree(3.141592653589793) == 180.0, "static")
  check(pt.to_string(pt.make(3.0, Some(2.0), None)) == "Pt(2, 2, 2)", "optional arguments")

  // A Group is an array of Pt: a list, and indexed.
  let square = group.new([pt(0.0, 0.0), pt(4.0, 0.0), pt(4.0, 4.0), pt(0.0, 4.0)])
  check(list_length(group.to_list(square)) == 4, "a list")
  let assert Some(third) = group.get_at(square, 2)
  check(pt.get_y(third) == 4.0, "indexed")
  check(pt.get_x(group.get_p1(square)) == 0.0, "getter")
  check(pt.to_string(group.centroid(square)) == "Pt(2, 2)", "centroid")
  check(pt.get_x(group.get_p2(group.bounding_box(square))) == 4.0, "bounding box")

  // A union parameter takes any of its members: a Group where points are wanted.
  check(pt.get_x(geom.centroid(t.PtLikeIterableGroup(square))) == 2.0, "a Group as points")
  check(line.magnitude(t.PtIterableGroup(group.new([pt(0.0, 0.0), pt(3.0, 4.0)]))) == 5.0, "line")
  let ray = line.from_angle(t.PtLikePt(pt(1.0, 1.0)), 0.0, 5.0)
  check(pt.get_x(group.get_p2(ray)) == 6.0 && pt.get_y(group.get_p2(ray)) == 1.0, "from angle")

  io.println("checks passed")
}

fn list_length(l: List(a)) -> Int {
  case l {
    [] -> 0
    [_, ..rest] -> 1 + list_length(rest)
  }
}
