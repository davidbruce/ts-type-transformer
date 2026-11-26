import gleam/dynamic
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import node/areas/events/event_emitter as events
import node/runtime

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() -> Nil {
  let bus =
    events.new()
    |> events.set_max_listeners(12.0)
  let passed = check(events.get_max_listeners(bus) == 12.0, "maxListeners")

  let metric_calls = runtime.ref_new(0)
  let metric_sum = runtime.ref_new(0.0)
  let once_calls = runtime.ref_new(0)
  let shutdown_calls = runtime.ref_new(0)

  // kept in a variable: remove_listener needs the same function
  let on_metric = fn(args) {
    runtime.ref_update(metric_calls, fn(n) { n + 1 })
    case args {
      [first, ..] ->
        case decode.run(first, decode.float) {
          Ok(n) -> {
            runtime.ref_update(metric_sum, fn(s) { s +. n })
            Nil
          }
          Error(_) -> Nil
        }
      [] -> Nil
    }
  }

  events.on(bus, "metric", on_metric)
  events.once(bus, "metric", fn(_) {
    runtime.ref_update(once_calls, fn(n) { n + 1 })
    Nil
  })
  events.on(bus, "shutdown", fn(_) {
    runtime.ref_update(shutdown_calls, fn(n) { n + 1 })
    Nil
  })

  let passed =
    passed
    + check(
      events.listener_count(bus, "metric") == 2.0,
      "initial listenerCount",
    )
    + check(list.length(events.event_names(bus)) == 2, "eventNames")

  let emitted1 =
    events.emit(bus, "metric", [
      dynamic.float(101.0),
      dynamic.string("ok"),
    ])
  let passed =
    passed
    + check(
      emitted1
        && runtime.ref_get(metric_calls) == 1
        && runtime.ref_get(once_calls) == 1,
      "first metric emit",
    )
    + check(events.listener_count(bus, "metric") == 1.0, "once removes itself")

  let emitted2 =
    events.emit(bus, "metric", [
      dynamic.float(202.0),
      dynamic.string("ok"),
    ])
  let passed =
    passed
    + check(
      emitted2
        && runtime.ref_get(metric_calls) == 2
        && runtime.ref_get(metric_sum) == 303.0
        && runtime.ref_get(once_calls) == 1,
      "second metric emit",
    )
    + check(!events.emit(bus, "idle", []), "idle emit false")

  events.remove_listener(bus, "metric", on_metric)
  let passed =
    passed
    + check(
      events.listener_count(bus, "metric") == 0.0
        && !events.emit(bus, "metric", [dynamic.float(1.0)]),
      "removeListener",
    )
    + check(runtime.ref_get(metric_calls) == 2, "metric count unchanged")

  events.remove_all_listeners_with(bus, "shutdown")
  let passed =
    passed
    + check(
      events.event_names(bus) == [] && runtime.ref_get(shutdown_calls) == 0,
      "removeAllListeners",
    )

  io.println(
    "maxListeners="
    <> int.to_string(float.round(events.get_max_listeners(bus)))
    <> " metricCalls="
    <> int.to_string(runtime.ref_get(metric_calls))
    <> " metricSum="
    <> int.to_string(float.round(runtime.ref_get(metric_sum))),
  )
  io.println(
    "onceCalls="
    <> int.to_string(runtime.ref_get(once_calls))
    <> " shutdownCalls="
    <> int.to_string(runtime.ref_get(shutdown_calls)),
  )
  io.println("PASS: event_bus (" <> int.to_string(passed) <> " checks)")
}
