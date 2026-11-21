// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import bindings/namespaces/geometry
import bindings/types as t
import gleam/io
import bindings/clock
import bindings/settings
import bindings/namespaces/geometry/deep
import bindings/namespaces/geometry/point


fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Global values.
  let s = b.get_settings()
  check(settings.get_level(s) == 2.0 && !settings.get_debug(s), "global")
  settings.set_debug(s, True)
  check(settings.get_debug(b.get_settings()), "global object")
  check(b.get_version() == "1.2.3", "global value")

  // A global with members of its own.
  check(clock.now() == 1000.0 && clock.zero == 0.0, "statics")

  // Namespaces, nested ones included.
  let p = geometry.point(0.0, 0.0)
  let q = geometry.point(3.0, 4.0)
  check(geometry.distance(p, q) == 5.0 && point.get_x(q) == 3.0, "namespace")
  check(geometry.get_unit() == 1.0 && deep.twice(4.0) == 8.0, "nested namespace")

  // Rest parameters take a list.
  check(b.join("-", ["a", "b", "c"]) == "a-b-c" && b.join("-", []) == "", "rest")

  // Numeric enums.
  check(b.bump(t.LevelLow) == t.LevelHigh && b.bump(t.LevelHigh) == t.LevelHigh, "numeric enum")

  io.println("checks passed")
}
