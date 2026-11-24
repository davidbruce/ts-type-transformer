// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import bindings/types as t
import gleam/io
import bindings/badge
import bindings/figure
import bindings/point
import bindings/namespaces/util


fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // A default export, imported as `default`.
  check(point.get_x(b.origin()) == 0.0, "default export")

  // Exported under other names: called and constructed by them.
  check(b.area(3.0, 4.0) == 12.0, "renamed function")
  let f = figure.new(2.0)
  check(figure.grow(f) == 3.0 && figure.get_size(f) == 3.0, "renamed class")
  check(badge.get_label(badge.new("x")) == "x", "re-exported default class")

  // Constants are read, never assigned.
  check(b.get_limit() == 10.0, "constant")
  check(b.get_counter() == 0.0 && b.bump() == 1.0 && b.get_counter() == 1.0, "a variable read live")

  // A union told apart by the classes as exported.
  let assert t.FigureOrBadgeFigure(fig) = b.pick(0.0)
  check(figure.get_size(fig) == 2.0, "instanceof Figure")
  let assert t.FigureOrBadgeBadge(_) = b.pick(1.0)

  // A tuple is told apart as an array, not by `toString`, which every object has.
  let assert t.PairOrPointArray(_) = b.locate(0.0)
  let assert t.PairOrPointPoint(_) = b.locate(1.0)

  // Rollup's namespace, by the name it is exported as: `util.half`.
  check(util.half(4.0) == 2.0 && util.get_twice()(4.0) == 8.0, "a namespace's re-exports")

  io.println("checks passed")
}
