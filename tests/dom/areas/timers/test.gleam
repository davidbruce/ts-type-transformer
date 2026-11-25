// Timers and scheduling from Gleam: setTimeout, setInterval, microtasks,
// animation frames, and the performance clock. Reports after they fire.
import bindings/areas/core as dom
import bindings/areas/core/idle_deadline
import bindings/areas/core/performance
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/option.{Some}
import gleam/string

fn callback(f: fn() -> Nil) -> t.Function {
  fn(_) {
    f()
    runtime.dynamic(Nil)
  }
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
  let perf = dom.get_performance()

  // The clock: monotonic, and after the time origin.
  let t0 = performance.now(perf)
  let t1 = performance.now(perf)
  let t2 = performance.now(perf)
  check(t0 >=. 0.0 && t1 >=. t0 && t2 >=. t1, "performance.now is monotonic")
  check(performance.get_time_origin(perf) >. 1.0e12, "timeOrigin is an epoch time")
  let mark = performance.mark(perf, "m1")
  check(float.absolute_value(performance.now(perf) -. t0) <. 5000.0, "the clock is in milliseconds")
  let _ = mark

  // Order: synchronous code, then microtasks, then a zero-delay timer.
  let order = cell.new("")
  let add = fn(letter) { cell.set(order, cell.get(order) <> letter) }
  let timer_a = dom.set_timeout_fn(callback(fn() { add("T") }), Some(0.0), [])
  dom.queue_microtask(fn() { add("1") })
  dom.queue_microtask(fn() {
    add("2")
    dom.queue_microtask(fn() { add("3") })
  })
  add("S")
  check(cell.get(order) == "S", "microtasks wait for the synchronous code")
  check(timer_a >. 0.0, "a timer id")

  // Timeouts: one fires (receiving extra args), one is cleared, ids differ.
  let fired_count = cell.new(0)
  let got_args = cell.new("")
  let cleared_count = cell.new(0)
  let late_count = cell.new(0)
  let ticks = cell.new(0)
  let cancelled_frame = cell.new(False)
  let fired =
    dom.set_timeout_fn(
      fn(args) {
        cell.set(fired_count, cell.get(fired_count) + 1)
        let decoded = case args {
          [a, b] -> {
            let assert Ok(sa) = decode.run(a, decode.string)
            let assert Ok(sb) = decode.run(b, decode.string)
            sa <> "," <> sb
          }
          _ -> ""
        }
        cell.set(got_args, decoded)
        runtime.dynamic(Nil)
      },
      Some(10.0),
      [dynamic.string("x"), dynamic.string("y")],
    )
  let cleared = dom.set_timeout_fn(callback(fn() { cell.set(cleared_count, cell.get(cleared_count) + 1) }), Some(10.0), [])
  check(fired != cleared, "timer ids differ")
  dom.clear_timeout_with(cleared)
  dom.clear_timeout()
  let late = dom.set_timeout_fn(callback(fn() { cell.set(late_count, cell.get(late_count) + 1) }), Some(250.0), [])

  // An interval that is cleared once it has ticked three times.
  let interval = dom.set_interval_fn(callback(fn() { cell.set(ticks, cell.get(ticks) + 1) }), Some(15.0), [])
  check(interval != fired, "interval id differs")

  // A frame request that is cancelled, and one that runs.
  let cancelled = dom.request_animation_frame(fn(_) { cell.set(cancelled_frame, True) })
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
    check(!cell.get(cancelled_frame), "a cancelled frame never runs")
    check(string.starts_with(cell.get(order), "S123"), "microtasks 1 2 3, nested one last: " <> cell.get(order))
    when(fn() { cell.get(ticks) >= 3 }, "three ticks", started, fn() {
      dom.clear_interval_with(interval)
      let at_clear = cell.get(ticks)
      check(at_clear >= 3, "the interval ticked at least three times")
      check(cell.get(order) == "S123T", "the zero timer ran after the microtasks")
      check(cell.get(fired_count) == 1, "the timeout fired once")
      check(cell.get(got_args) == "x,y", "the timeout received its extra arguments")
      check(cell.get(cleared_count) == 0, "a cleared timeout never fired")
      check(cell.get(late_count) == 0, "the late timeout has not fired yet")
      // Let some intervals' worth of time pass: a cleared interval is quiet.
      when(fn() { cell.get(late_count) == 1 }, "the late timeout", started, fn() {
        check(cell.get(late_count) == 1, "the late timeout fired once")
        check(performance.now(perf) -. started >=. 100.0, "the late timeout waited")
        check(cell.get(ticks) == at_clear, "the cleared interval stopped")
        dom.clear_timeout_with(late)
        idle(check, count, perf)
      })
    })
  })
}

// An idle callback runs with a deadline, then the program reports.
fn idle(check, count, perf) -> Nil {
  let remaining = cell.new(-1.0)
  dom.request_idle_callback(fn(deadline) { cell.set(remaining, idle_deadline.time_remaining(deadline)) })
  let started = performance.now(perf)
  when(fn() { cell.get(remaining) >=. 0.0 }, "the idle callback", started, fn() {
    check(cell.get(remaining) <=. 50.0, "idle time remaining is at most 50 ms")
    io.println(int.to_string(cell.get(count)) <> " checks passed")
  })
}
