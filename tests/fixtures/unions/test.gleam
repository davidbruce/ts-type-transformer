// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import bindings/types as t
import gleam/io
import bindings/loose_a
import bindings/options
import bindings/square
import bindings/widget


fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  let widget = widget.new("w")
  let options = options.create()
  options.set_verbose(options, True)

  // A union is a custom type: any member goes in.
  check(b.describe(t.ValueString("text")) == "string text", "string case")
  check(b.describe(t.ValueNumber(42.0)) == "number 42", "number case")
  check(b.describe(t.ValueWidget(widget)) == "widget w", "class case")
  check(b.describe(t.ValueOptions(options)) == "options verbose=true", "option bag case")

  // What comes back says which member it is.
  check(b.pick(0.0) == t.ValueString("text"), "string out")
  check(b.pick(1.0) == t.ValueNumber(42.0), "number out")
  let assert t.ValueWidget(w) = b.pick(2.0)
  check(widget.get_name(w) == "w", "instanceof out")

  // A tagged union, told apart by its `kind`.
  let square = b.make_shape("square", 3.0)
  let assert t.ShapeSquare(sq) = square
  check(square.get_side(sq) == 3.0 && b.area(square) == 9.0, "tagged union")

  // An inline union.
  check(b.setup(t.BooleanOrOptionsBoolean(True)) == "bool true", "inline union")
  check(b.setup(t.BooleanOrOptionsOptions(options)) == "options verbose=true", "inline union object")
  check(b.lookup("abc") == t.NumberOrWidgetNumber(3.0), "inline union out")

  // Members no test tells apart: typed going in, made of any subtype by a function.
  let a = loose_a.create() |> loose_a.with_a(1.0)
  check(b.loose(t.loose_loose_a(a)) == "loose {\"a\":1}", "a loose union made of a member")
  check(b.loose(t.LooseLooseA(a)) == "loose {\"a\":1}", "and by its variant")

  io.println("checks passed")
}
