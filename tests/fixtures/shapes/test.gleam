// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import bindings/named
import bindings/shape
import bindings/types as t
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Class pairing: a constructor, a static function, a constant.
  let s = shape.new(t.ShapeKindSquare, 3.0)
  check(shape.area(s) == 9.0, "constructor")
  check(shape.area(shape.unit()) == 1.0, "static")
  check(shape.sides == 4.0, "constant")

  // A string enum is a custom type, in both directions.
  check(shape.get_kind(s) == t.ShapeKindSquare, "enum out")
  check(
    shape.get_kind(shape.new(t.ShapeKindCircle, 1.0)) == t.ShapeKindCircle,
    "enum in",
  )

  // Properties.
  shape.set_size(s, 4.0)
  check(shape.get_size(s) == 4.0, "property")

  // A Shape is a Named: the Named's functions take it as it is, no upcast.
  check(named.get_name(s) == "square-3", "inherited")
  check(named.describe(s) == "square-3 (16)", "inherited method")
  check(b.name_of(s) == "square-3", "a Shape where a Named is wanted")
  // Closing the type at Named, for a list of several kinds of Named.
  let named = [named.upcast(s), named.upcast(shape.unit())]
  check(list.length(named) == 2, "upcast")

  // Overloads are renamed.
  let big = shape.scale(s, 2.0)
  check(shape.get_size(big) == 8.0, "overload")
  check(
    named.get_name(shape.scale_with_name(s, 2.0, "big")) == "big",
    "renamed overload",
  )

  // An optional argument is an Option.
  shape.resize(s, 2.6)
  check(shape.get_size(s) == 2.6, "absent argument")
  shape.resize_with(s, 2.6, Some(True))
  check(shape.get_size(s) == 3.0, "given argument")

  // A union is a custom type.
  check(
    shape.label(s, t.StringOrNumberString("a")) == "string:a",
    "union string",
  )
  check(
    shape.label(s, t.StringOrNumberNumber(7.0)) == "number:7",
    "union number",
  )

  // A nullable value is an Option.
  check(shape.get_parent(s) == None, "null")
  check(shape.get_parent(big) == Some(s), "not null")
  shape.set_parent(big, None)
  check(shape.get_parent(big) == None, "set null")

  // Functions.
  check(b.total_area(s, shape.unit()) == 10.0, "function")
  check(b.largest(s, None) == Some(s), "null argument")
  check(b.largest(shape.unit(), Some(s)) == Some(s), "nullable result")
  check(shape.area(b.or_unit()) == 1.0, "undefined argument")
  check(b.or_unit_with(Some(s)) == s, "defined argument")
  check(b.find("nothing") == None, "undefined result")
  check(option.map(b.find("unit"), shape.area) == Some(1.0), "defined result")

  // `adopt<T extends Named>(child: T): T` takes any Named and gives back its
  // type: a Shape stays a Shape.
  check(shape.area(b.adopt(s)) == shape.area(s), "bound type parameter")
  // Narrowing, checked: a Shape is a class at runtime; a plain Named is not one.
  let assert Ok(back) = shape.from(named.upcast(s))
  check(shape.area(back) == shape.area(s), "checked downcast")
  check(shape.from(b.plain()) == Error(Nil), "not a Shape")

  io.println("checks passed")
}
