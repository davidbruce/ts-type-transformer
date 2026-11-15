// Calls the generated Gleam bindings; every check must hold.
import bindings/areas/main as b
import bindings/runtime as r
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
  // Option objects: built here, read by JavaScript.
  let o = b.options_create()
  b.options_set_width(o, 3.0)
  b.options_set_mode(o, t.OptionsModeFill)
  check(b.describe(o) == "3x? fill", "option object")
  b.options_set_height(o, Some(2.0))
  b.options_set_title(o, Some("hi"))
  check(b.describe(o) == "3x2 fill \"hi\"", "optional properties")

  // Built by JavaScript, read here. A missing optional property is None.
  let d = b.defaults()
  check(b.options_get_width(d) == 4.0 && b.options_get_mode(d) == t.OptionsModeFit, "read")
  check(b.options_get_height(d) == None && b.options_get_title(d) == None, "missing")

  // Partial<Options>, Pick<Options, ...>, keyof Options.
  let changes = b.patch_changes_create()
  b.patch_changes_set_width(changes, Some(9.0))
  let patched = b.patch(d, changes)
  check(b.options_get_width(patched) == 9.0, "Partial")
  let size = b.area_size_create()
  b.area_size_set_width(size, 3.0)
  check(b.area(size) == 9.0, "Pick")
  check(b.field() == t.FieldResultTitle, "keyof")

  // Record<string, number>: an index.
  let table = b.lookup_table_create()
  b.lookup_table_set_at(table, "a", 7.0)
  check(b.lookup(table, "a") == 7.0 && b.lookup(table, "b") == -1.0, "Record")
  check(b.lookup_table_get_at(table, "a") == Some(7.0) && b.lookup_table_get_at(table, "b") == None, "index")

  // An intersection, and an inline object type.
  let tagged = b.tagged_create()
  b.tagged_set_name(tagged, "x")
  b.tagged_set_id(tagged, 5.0)
  check(b.show(tagged) == "x#5", "intersection")
  check(b.norm_point_get_x(b.origin()) == 0.0, "inline object")

  // Arrays are lists, tuples tuples.
  check(b.range(4.0) == [0.0, 1.0, 2.0, 3.0], "array")
  check(b.sum([10.0, 5.0]) == 15.0, "array argument")
  check(b.words("a bc") == ["a", "bc"], "array of strings")
  check(b.pair(1.0, "one") == #(1.0, "one"), "tuple")
  check(b.swap(#(2.0, "two")) == #("two", 2.0), "tuple argument")
  // Generic aliases are the types they stand for.
  check(b.total([1.0, 2.0]) == 3.0, "List(Float)")
  check(b.flip(#("x", 7.0)) == #(7.0, "x"), "Pair(String, Float)")
  let assert [first, ..] = b.items()
  check(b.item_get_id(first) == 1.0 && b.item_get_tags(first) == ["a", "b"], "array of objects")

  // Generics are Gleam's. A value of T crosses by the codec of T: a handle
  // (`Box(t)`) as it is, so it needs none.
  let float = r.codec_float()
  let bx = b.wrap(21.0, float)
  check(b.box_get_value(bx, float) == 21.0, "generic")
  check(b.box_get_value(b.box_map(bx, fn(v) { v *. 2.0 }, float, float), float) == 42.0, "generic callback")
  check(b.first(b.range(4.0), float) == Some(0.0) && b.first([], float) == None, "generic function")
  // What a Box holds is converted as T is: an Id, a variant, not JavaScript's string.
  let assert t.IdString(three) = b.box_get_value(b.boxed_id(), t.id_codec())
  check(three == "three", "Box(Id)")
  let assert [t.IdNumber(1.0), ..] = b.box_get_value(b.wrap([t.IdNumber(1.0)], r.codec_list(t.id_codec())), r.codec_list(t.id_codec()))
  check(b.box_get_value(b.wrap(Some("x"), r.codec_option(r.codec_string())), r.codec_option(r.codec_string())) == Some("x"), "Box(Option(String))")

  // A generic union, and a union in an array.
  check(b.unbox(t.EitherT(5.0), float) == 5.0, "union of T")
  check(b.unbox(t.EitherBox(bx), float) == 21.0, "union of Box(T)")
  check(b.ids() == [t.IdNumber(1.0), t.IdString("two")], "array of unions")

  // Pipe extends Source(Float) and Sink(Float). It is a Source, and has
  // Sink's members as its own, Float for T.
  let pp = b.pipe()
  check(b.pipe_get_label(pp) == "pipe", "a member of both bases")
  b.pipe_put(pp, 4.0)
  check(b.source_take(pp, r.codec_float()) == 4.0, "the base's, on the subtype")

  io.println("checks passed")
}
