// Timers and scheduling from Gleam: setTimeout, setInterval, microtasks,
// animation frames, and the performance clock. Reports after they fire.
//
// FINDING 6 (can't express): setTimeout and setInterval take a TimerHandler,
// whose function case holds a `Function`, an opaque type nothing can make
// (no constructor, no conversion from a Gleam fn). Only the string case works,
// which runs JavaScript source, so the timers below act through the page
// (attributes of the body) and the program watches them from animation frames.
import bindings/areas/core as dom
import bindings/areas/core/document
import bindings/areas/core/element
import bindings/areas/core/idle_deadline
import bindings/areas/core/performance
import cell
import bindings/types as t
import gleam/float
import gleam/int
import gleam/io
import gleam/option.{None, Some}
import gleam/string

fn code(js: String) {
  t.TimerHandlerString(js)
}

fn bump(attr: String) -> String {
  "document.body.setAttribute('"
  <> attr
  <> "', String(+(document.body.getAttribute('"
  <> attr
  <> "')||0)+1))"
}

// Runs `then` on the first animation frame after `ready()` holds.
fn when(ready: fn() -> Bool, what: String, started: Float, then: fn() -> Nil) -> Nil {
  dom.request_animation_frame(fn(_) {
    case ready() {
      True -> then()
      False ->
        case performance.now(dom.get_performance()) -. started >. 8000.0 {
          True -> panic as { "timed out waiting for " <> what }
          False -> when(ready, what, started, then)
        }
    }
  })
  Nil
}

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }
  let doc = dom.get_document()
  let body = document.get_body(doc)
  let perf = dom.get_performance()
  let attr = fn(name) { element.get_attribute(body, name) }
  let num = fn(name) {
    case option.map(attr(name), int.parse) {
      Some(Ok(n)) -> n
      _ -> 0
    }
  }

  // The clock: monotonic, and after the time origin.
  let t0 = performance.now(perf)
  let t1 = performance.now(perf)
  let t2 = performance.now(perf)
  check(t0 >=. 0.0 && t1 >=. t0 && t2 >=. t1, "performance.now is monotonic")
  check(performance.get_time_origin(perf) >. 1.0e12, "timeOrigin is an epoch time")
  let mark = performance.mark(perf, "m1", None)
  check(float.absolute_value(performance.now(perf) -. t0) <. 5000.0, "the clock is in milliseconds")
  let _ = mark

  // Order: synchronous code, then microtasks, then a zero-delay timer.
  element.set_attribute(body, "data-order", "")
  let add = fn(letter) {
    let old = case attr("data-order") {
      Some(s) -> s
      None -> ""
    }
    element.set_attribute(body, "data-order", old <> letter)
  }
  let timer_a = dom.set_timeout(code("document.body.setAttribute('data-order', document.body.getAttribute('data-order') + 'T')"), Some(0.0), [])
  dom.queue_microtask(fn() { add("1") })
  dom.queue_microtask(fn() {
    add("2")
    dom.queue_microtask(fn() { add("3") })
  })
  add("S")
  check(attr("data-order") == Some("S"), "microtasks wait for the synchronous code")
  check(timer_a >. 0.0, "a timer id")

  // Timeouts: one fires, one is cleared, ids differ.
  let fired = dom.set_timeout(code(bump("data-fired")), Some(10.0), [])
  let cleared = dom.set_timeout(code(bump("data-cleared")), Some(10.0), [])
  check(fired != cleared, "timer ids differ")
  dom.clear_timeout(Some(cleared))
  dom.clear_timeout(None)
  let late = dom.set_timeout(code(bump("data-late")), Some(250.0), [])

  // An interval that is cleared once it has ticked three times.
  let interval = dom.set_interval(code(bump("data-ticks")), Some(15.0), [])
  check(interval != fired, "interval id differs")

  // A frame request that is cancelled, and one that runs.
  let cancelled = dom.request_animation_frame(fn(_) { element.set_attribute(body, "data-cancelled-frame", "ran") })
  dom.cancel_animation_frame(cancelled)
  let frame_time = cell.new(-1.0)
  let frame_started = performance.now(perf)
  let frame_id = dom.request_animation_frame(fn(stamp) { cell.set(frame_time, stamp) })
  check(frame_id != cancelled, "frame ids differ")
  check(cell.get(frame_time) <. 0.0, "a frame does not run at once")

  let started = performance.now(perf)
  when(fn() { cell.get(frame_time) >=. 0.0 }, "the frame", started, fn() {
    check(cell.get(frame_time) >=. frame_started -. 1000.0, "the frame timestamp is a clock reading")
    check(cell.get(frame_time) <=. performance.now(perf), "the frame timestamp is not in the future")
    check(attr("data-cancelled-frame") == None, "a cancelled frame never runs")
    check(string.starts_with(option.unwrap(attr("data-order"), ""), "S123"), "microtasks 1 2 3, nested one last: " <> option.unwrap(attr("data-order"), "?"))
    when(fn() { num("data-ticks") >= 3 }, "three ticks", started, fn() {
      dom.clear_interval(Some(interval))
      let at_clear = num("data-ticks")
      check(at_clear >= 3, "the interval ticked at least three times")
      check(attr("data-order") == Some("S123T"), "the zero timer ran after the microtasks")
      check(num("data-fired") == 1, "the timeout fired once")
      check(attr("data-cleared") == None, "a cleared timeout never fired")
      check(num("data-late") == 0, "the late timeout has not fired yet")
      // Let some intervals' worth of time pass: a cleared interval is quiet.
      when(fn() { num("data-late") == 1 }, "the late timeout", started, fn() {
        check(num("data-late") == 1, "the late timeout fired once")
        check(performance.now(perf) -. started >=. 100.0, "the late timeout waited")
        check(num("data-ticks") == at_clear, "the cleared interval stopped")
        dom.clear_timeout(Some(late))
        idle(check, count, perf)
      })
    })
  })
}

// An idle callback runs with a deadline, then the program reports.
fn idle(check, count, perf) -> Nil {
  let remaining = cell.new(-1.0)
  let options = None
  dom.request_idle_callback(fn(deadline) { cell.set(remaining, idle_deadline.time_remaining(deadline)) }, options)
  let started = performance.now(perf)
  when(fn() { cell.get(remaining) >=. 0.0 }, "the idle callback", started, fn() {
    check(cell.get(remaining) <=. 50.0, "idle time remaining is at most 50 ms")
    io.println(int.to_string(cell.get(count)) <> " checks passed")
  })
}
