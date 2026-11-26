// Calls the generated Gleam bindings; every check must hold.
import bindings as b
import bindings/loose
import bindings/loose_a
import bindings/number_or_widget
import bindings/options
import bindings/shape
import bindings/square
import bindings/value
import bindings/widget
import gleam/io

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

  // A union going in: its first member plain, each other one by name.
  check(b.describe("text") == "string text", "string case")
  check(b.describe_number(42.0) == "number 42", "number case")
  check(b.describe_widget(widget) == "widget w", "class case")
  check(
    b.describe_options(options) == "options verbose=true",
    "option bag case",
  )

  // What comes back says which member it is.
  check(b.pick(0.0) == value.String("text"), "string out")
  check(b.pick(1.0) == value.Number(42.0), "number out")
  let assert value.Widget(w) = b.pick(2.0)
  check(widget.get_name(w) == "w", "instanceof out")

  // A tagged union, told apart by its `kind`; one given back goes in by `_of`.
  let s = b.make_shape("square", 3.0)
  let assert shape.Square(sq) = s
  check(square.get_side(sq) == 3.0 && b.area_of(s) == 9.0, "tagged union")

  // An inline union.
  check(b.setup(True) == "bool true", "inline union")
  check(
    b.setup_options(options) == "options verbose=true",
    "inline union object",
  )
  check(b.lookup("abc") == number_or_widget.Number(3.0), "inline union out")

  // Members no test tells apart: each goes in by name; the union, made of
  // any subtype by a function, by `_of`.
  let a = loose_a.create() |> loose_a.with_a(1.0)
  check(b.loose_a(a) == "loose {\"a\":1}", "a loose union's member")
  check(
    b.loose_of(loose.a(a)) == "loose {\"a\":1}",
    "a loose union made of a member",
  )
  check(b.loose_of(loose.A(a)) == "loose {\"a\":1}", "and by its variant")

  io.println("checks passed")
}
