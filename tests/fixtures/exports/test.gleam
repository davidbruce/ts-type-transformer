// Calls the generated Gleam bindings; every check must hold.
import bindings/areas/main as b
import bindings/types as t
import gleam/io


fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // A default export, imported as `default`.
  check(b.point_get_x(b.origin()) == 0.0, "default export")

  // Exported under other names: called and constructed by them.
  check(b.area(3.0, 4.0) == 12.0, "renamed function")
  let f = b.figure_new(2.0)
  check(b.figure_grow(f) == 3.0 && b.figure_get_size(f) == 3.0, "renamed class")
  check(b.badge_get_label(b.badge_new("x")) == "x", "re-exported default class")

  // Constants are read, never assigned.
  check(b.get_limit() == 10.0, "constant")
  check(b.get_counter() == 0.0 && b.bump() == 1.0 && b.get_counter() == 1.0, "a variable read live")

  // A union told apart by the classes as exported.
  let assert t.FigureOrBadgeFigure(fig) = b.pick(0.0)
  check(b.figure_get_size(fig) == 2.0, "instanceof Figure")
  let assert t.FigureOrBadgeBadge(_) = b.pick(1.0)

  io.println("checks passed")
}
