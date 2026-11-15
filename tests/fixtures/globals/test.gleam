// Calls the generated Gleam bindings; every check must hold.
import bindings/areas/main as b
import bindings/namespaces/geometry
import bindings/types as t
import gleam/io


fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Global values.
  let s = b.get_settings()
  check(b.settings_get_level(s) == 2.0 && !b.settings_get_debug(s), "global")
  b.settings_set_debug(s, True)
  check(b.settings_get_debug(b.get_settings()), "global object")
  check(b.get_version() == "1.2.3", "global value")

  // A global with members of its own.
  check(b.clock_now() == 1000.0 && b.clock_zero == 0.0, "statics")

  // Namespaces, nested ones included.
  let p = geometry.geometry_point(0.0, 0.0)
  let q = geometry.geometry_point(3.0, 4.0)
  check(geometry.geometry_distance(p, q) == 5.0 && geometry.geometry_point_get_x(q) == 3.0, "namespace")
  check(geometry.get_geometry_unit() == 1.0 && geometry.geometry_deep_twice(4.0) == 8.0, "nested namespace")

  // Rest parameters take a list.
  check(b.join("-", ["a", "b", "c"]) == "a-b-c" && b.join("-", []) == "", "rest")

  // Numeric enums.
  check(b.bump(t.LevelLow) == t.LevelHigh && b.bump(t.LevelHigh) == t.LevelHigh, "numeric enum")

  io.println("checks passed")
}
