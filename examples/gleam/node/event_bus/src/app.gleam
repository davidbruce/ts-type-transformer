import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import node/areas/events/event_emitter as events
import node/namespaces/node_js/event_emitter as emitter
import node/runtime

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() -> Nil {
  let bus =
    events.new(None)
    |> emitter.set_max_listeners(12.0)
  let passed = check(emitter.get_max_listeners(bus) == 12.0, "maxListeners")

  let metric_calls = runtime.ref_new(0)
  let metric_sum = runtime.ref_new(0.0)
  let once_calls = runtime.ref_new(0)
  let shutdown_calls = runtime.ref_new(0)

  let ev_metric = runtime.dynamic("metric")
  let ev_shutdown = runtime.dynamic("shutdown")
  let ev_idle = runtime.dynamic("idle")

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
  let on_once = fn(_) {
    runtime.ref_update(once_calls, fn(n) { n + 1 })
    Nil
  }
  let on_shutdown = fn(_) {
    runtime.ref_update(shutdown_calls, fn(n) { n + 1 })
    Nil
  }

  emitter.on(bus, ev_metric, on_metric)
  emitter.once(bus, ev_metric, on_once)
  emitter.on(bus, ev_shutdown, on_shutdown)

  let passed =
    passed
    + check(
      emitter.listener_count(bus, ev_metric, None) == 2.0,
      "initial listenerCount",
    )
    + check(list.length(emitter.event_names(bus)) == 2, "eventNames")

  let emitted1 =
    emitter.emit(bus, ev_metric, [
      runtime.dynamic(101.0),
      runtime.dynamic("ok"),
    ])
  let passed =
    passed
    + check(
      emitted1
      && runtime.ref_get(metric_calls) == 1
      && runtime.ref_get(once_calls) == 1,
      "first metric emit",
    )
    + check(
      emitter.listener_count(bus, ev_metric, None) == 1.0,
      "once removes itself",
    )

  let emitted2 =
    emitter.emit(bus, ev_metric, [
      runtime.dynamic(202.0),
      runtime.dynamic("ok"),
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
    + check(!emitter.emit(bus, ev_idle, []), "idle emit false")

  emitter.remove_listener(bus, ev_metric, on_metric)
  let passed =
    passed
    + check(
      emitter.listener_count(bus, ev_metric, None) == 0.0
      && !emitter.emit(bus, ev_metric, [runtime.dynamic(1.0)]),
      "removeListener",
    )
    + check(runtime.ref_get(metric_calls) == 2, "metric count unchanged")

  emitter.remove_all_listeners(bus, Some(ev_shutdown))
  let passed =
    passed
    + check(
      emitter.event_names(bus) == []
      && runtime.ref_get(shutdown_calls) == 0,
      "removeAllListeners",
    )

  io.println(
    "maxListeners="
    <> int.to_string(float.round(emitter.get_max_listeners(bus)))
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
