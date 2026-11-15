// Calls the generated Gleam bindings; every check must hold.
import bindings/areas/main as b
import bindings/types as t
import gleam/io
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Class pairing: a constructor, a static function, a constant.
  let s = b.shape_new(t.ShapeKindSquare, 3.0)
  check(b.shape_area(s) == 9.0, "constructor")
  check(b.shape_area(b.shape_unit()) == 1.0, "static")
  check(b.shape_sides == 4.0, "constant")

  // A string enum is a custom type, in both directions.
  check(b.shape_get_kind(s) == t.ShapeKindSquare, "enum out")
  check(b.shape_get_kind(b.shape_new(t.ShapeKindCircle, 1.0)) == t.ShapeKindCircle, "enum in")

  // Properties.
  b.shape_set_size(s, 4.0)
  check(b.shape_get_size(s) == 4.0, "property")

  // Inherited members are the type's own; an ancestor is reached by `as`.
  check(b.shape_get_name(s) == "square-3", "inherited")
  check(b.shape_describe(s) == "square-3 (16)", "inherited method")
  check(b.name_of(b.shape_as_named(s)) == "square-3", "upcast")

  // Overloads are renamed.
  let big = b.shape_scale(s, 2.0)
  check(b.shape_get_size(big) == 8.0, "overload")
  check(b.shape_get_name(b.shape_scale_with_name(s, 2.0, "big")) == "big", "renamed overload")

  // An optional argument is an Option.
  b.shape_resize(s, 2.6, None)
  check(b.shape_get_size(s) == 2.6, "absent argument")
  b.shape_resize(s, 2.6, Some(True))
  check(b.shape_get_size(s) == 3.0, "given argument")

  // A union is a custom type.
  check(b.shape_label(s, t.StringOrNumberString("a")) == "string:a", "union string")
  check(b.shape_label(s, t.StringOrNumberNumber(7.0)) == "number:7", "union number")

  // A nullable value is an Option.
  check(b.shape_get_parent(s) == None, "null")
  check(b.shape_get_parent(big) == Some(s), "not null")
  b.shape_set_parent(big, None)
  check(b.shape_get_parent(big) == None, "set null")

  // Functions.
  check(b.total_area(s, b.shape_unit()) == 10.0, "function")
  check(b.largest(s, None) == Some(s), "null argument")
  check(b.largest(b.shape_unit(), Some(s)) == Some(s), "nullable result")
  check(b.shape_area(b.or_unit(None)) == 1.0, "undefined argument")
  check(b.or_unit(Some(s)) == s, "defined argument")
  check(b.find("nothing") == None, "undefined result")
  check(option.map(b.find("unit"), b.shape_area) == Some(1.0), "defined result")

  io.println("checks passed")
}
