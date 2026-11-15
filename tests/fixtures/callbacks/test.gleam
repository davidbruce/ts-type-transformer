// Calls the generated Gleam bindings; every check must hold.
import bindings/areas/main as b
import bindings/runtime as r
import bindings/types as t
import cell
import gleam/int
import gleam/io
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // An object that is also a function, and can be constructed.
  let s = b.scaler(2.0)
  check(b.scaler_invoke(s, 5.0) == 10.0, "invoke")
  check(b.scaler_get_factor(s) == 2.0, "property of a function")
  check(b.scaler_invoke(b.scaler_construct(s, 3.0), 5.0) == 15.0, "construct")

  // A call that throws: `attempt` gives the exception.
  check(r.attempt(fn() { b.explode("boom") }) |> result_message == Error("boom"), "exception")
  check(r.attempt(fn() { b.flush() }) == Ok(0.0), "no exception")

  let c = b.counter_new()
  let last = cell.new(0.0)
  let calls = cell.new(0)

  // A callback is a Gleam function; the same one is the same JavaScript function.
  let on_value = fn(value: Float) {
    cell.set(last, value)
    cell.set(calls, cell.get(calls) + 1)
  }
  b.counter_subscribe(c, on_value)
  check(b.counter_increment(c, None) == 1.0, "increment")
  check(cell.get(last) == 1.0, "listener called")
  let _ = b.counter_increment(c, Some(4.0))
  check(cell.get(last) == 5.0 && cell.get(calls) == 2, "listener called again")

  // A callback that takes an object and returns a value.
  check(b.counter_visit(c, fn(counter) { b.counter_get_count(counter) == 5.0 }), "callback result")

  // Removed by identity.
  b.counter_unsubscribe(c, on_value)
  let _ = b.counter_increment(c, None)
  check(cell.get(last) == 5.0 && cell.get(calls) == 2, "listener removed")

  // A nullable callback property.
  check(option.is_none(b.counter_get_onchange(c)), "no callback")
  let prop = cell.new(0.0)
  b.counter_set_onchange(c, Some(fn(value) { cell.set(prop, value) }))
  check(option.is_some(b.counter_get_onchange(c)), "a callback")
  let _ = b.counter_increment(c, None)
  check(cell.get(prop) == 7.0, "callback property called")
  b.counter_set_onchange(c, None)
  let _ = b.counter_increment(c, None)
  check(cell.get(prop) == 7.0, "callback property removed")

  // Several arguments, a string among them, and a result.
  check(b.counter_fold(c, fn(total, label) { total *. 2.0 +. int.to_float(string.length(label)) }) == 8.0 *. 2.0 +. 5.0, "several arguments")

  // A callback kept and called later.
  let ran = cell.new(0)
  b.later(fn() { cell.set(ran, cell.get(ran) + 1) })
  b.later(fn() { cell.set(ran, cell.get(ran) + 1) })
  check(cell.get(ran) == 0, "not yet")
  check(b.flush() == 2.0, "flushed")
  check(cell.get(ran) == 2, "called later")

  // A union argument arrives as the custom type.
  let seen = cell.new([])
  check(b.each_id(fn(id) { cell.set(seen, [id, ..cell.get(seen)]) }) == 2.0, "union callback")
  check(cell.get(seen) == [t.IdString("two"), t.IdNumber(1.0)], "union argument")

  // Promises: a function is given the result once one settles.
  r.promise_then(b.delayed(21.0), fn(r) {
    check(r == Ok(42.0), "resolved")
    r.promise_then(b.rejected("nope"), fn(r) {
      check(result_message(r) == Error("nope"), "rejected")
      r.promise_then(b.settled(), fn(r) {
        check(r == Ok(Nil), "settled")
        io.println("checks passed")
      })
    })
  })
}

fn result_message(r: Result(a, r.JsError)) -> Result(a, String) {
  case r {
    Ok(v) -> Ok(v)
    Error(e) -> Error(r.js_error_message(e))
  }
}
