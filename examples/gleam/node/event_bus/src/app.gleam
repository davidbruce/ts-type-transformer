import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import node/areas/events/event_emitter as events
import node/namespaces/node_js/event_emitter as emitter
import node/runtime

type Cell(a)

@external(javascript, "./app_ffi.mjs", "newCell")
fn new_cell(value: a) -> Cell(a)

@external(javascript, "./app_ffi.mjs", "getCell")
fn get_cell(cell: Cell(a)) -> a

@external(javascript, "./app_ffi.mjs", "setCell")
fn set_cell(cell: Cell(a), value: a) -> Nil

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() -> Nil {
  let bus = events.new(None)
  let _ = emitter.set_max_listeners(bus, 12.0)
  let passed = check(emitter.get_max_listeners(bus) == 12.0, "maxListeners")

  let metric_calls = new_cell(0)
  let metric_sum = new_cell(0.0)
  let once_calls = new_cell(0)
  let shutdown_calls = new_cell(0)

  let ev_metric = runtime.dynamic("metric")
  let ev_shutdown = runtime.dynamic("shutdown")
  let ev_idle = runtime.dynamic("idle")

  let on_metric = fn(args) {
    set_cell(metric_calls, get_cell(metric_calls) + 1)
    case args {
      [first, ..] ->
        case decode.run(first, decode.float) {
          Ok(n) -> set_cell(metric_sum, get_cell(metric_sum) +. n)
          Error(_) -> Nil
        }
      [] -> Nil
    }
  }
  let on_once = fn(_) { set_cell(once_calls, get_cell(once_calls) + 1) }
  let on_shutdown = fn(_) {
    set_cell(shutdown_calls, get_cell(shutdown_calls) + 1)
  }

  let _ = emitter.on(bus, ev_metric, on_metric)
  let _ = emitter.once(bus, ev_metric, on_once)
  let _ = emitter.on(bus, ev_shutdown, on_shutdown)

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
      emitted1 && get_cell(metric_calls) == 1 && get_cell(once_calls) == 1,
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
      && get_cell(metric_calls) == 2
      && get_cell(metric_sum) == 303.0
      && get_cell(once_calls) == 1,
      "second metric emit",
    )
    + check(!emitter.emit(bus, ev_idle, []), "idle emit false")

  let _ = emitter.remove_listener(bus, ev_metric, on_metric)
  let passed =
    passed
    + check(
      emitter.listener_count(bus, ev_metric, None) == 0.0
      && !emitter.emit(bus, ev_metric, [runtime.dynamic(1.0)]),
      "removeListener",
    )
    + check(get_cell(metric_calls) == 2, "metric count unchanged")

  let _ = emitter.remove_all_listeners(bus, Some(ev_shutdown))
  let passed =
    passed
    + check(
      emitter.event_names(bus) == []
      && get_cell(shutdown_calls) == 0,
      "removeAllListeners",
    )

  io.println(
    "maxListeners="
    <> int.to_string(float.round(emitter.get_max_listeners(bus)))
    <> " metricCalls="
    <> int.to_string(get_cell(metric_calls))
    <> " metricSum="
    <> int.to_string(float.round(get_cell(metric_sum))),
  )
  io.println(
    "onceCalls="
    <> int.to_string(get_cell(once_calls))
    <> " shutdownCalls="
    <> int.to_string(get_cell(shutdown_calls)),
  )
  io.println("PASS: event_bus (" <> int.to_string(passed) <> " checks)")
}
