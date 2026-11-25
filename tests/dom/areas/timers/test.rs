// Timers and the event loop from Rust: timeouts, intervals, microtasks,
// animation frames and the clock. `busy` says how many callbacks are still
// due; `finish` checks the order they came in. Every check! must hold.
#![no_main]
use bindings::*;
use std::cell::{Cell, RefCell};
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

thread_local! {
    static PENDING: Cell<i32> = const { Cell::new(0) };
    static LOG: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static ELAPSED: Cell<f64> = const { Cell::new(-1.0) };
    static TICKS: Cell<i32> = const { Cell::new(0) };
    static FRAMES: RefCell<Vec<f64>> = const { RefCell::new(Vec::new()) };
}

fn log(entry: &str) {
    LOG.with(|l| l.borrow_mut().push(entry.to_string()));
}

fn function(mut f: impl FnMut() + 'static) -> Function {
    Function::new(move |_| {
        f();
        JsValue::from(0.0)
    })
}

// A callback that is due: counted until it has run.
fn due(entry: &'static str) -> Function {
    PENDING.with(|p| p.set(p.get() + 1));
    function(move || {
        log(entry);
        PENDING.with(|p| p.set(p.get() - 1));
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    PENDING.with(Cell::get)
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    let log = LOG.with(|l| l.borrow().clone());
    check!(busy() == 0);
    // Synchronous code first, then microtasks in order, then timers by delay (ties by creation).
    check!(log[..3] == ["sync-end", "micro-1", "micro-2"]);
    let timers: Vec<&str> = log[3..].iter().map(String::as_str).filter(|e| e.starts_with("timeout")).collect();
    check!(timers == ["timeout-0", "timeout-0b", "timeout-20", "timeout-40"]);
    // The microtask queued by a microtask runs before any timer; one in a timer, before the next.
    check!(log.iter().position(|e| e == "micro-nested") < log.iter().position(|e| e == "timeout-0"));
    let i = log.iter().position(|e| e == "timeout-20").unwrap();
    check!(log[i + 1] == "micro-in-timer");
    // The cleared timeout never ran.
    check!(!log.iter().any(|e| e == "cleared"));
    // The delay was kept: at least that long passed.
    check!(ELAPSED.with(Cell::get) >= 39.0);
    // The interval ticked three times and was stopped from inside.
    check!(TICKS.with(Cell::get) == 3);
    check!(log.iter().filter(|e| e.starts_with("tick")).count() == 3);
    // Animation frames: two ran, in order, with increasing timestamps; the canceled one did not.
    let frames = FRAMES.with(|f| f.borrow().clone());
    check!(frames.len() == 2 && frames[0] > 0.0 && frames[1] >= frames[0]);
    check!(log.iter().any(|e| e == "frame-1") && !log.iter().any(|e| e == "frame-canceled"));
    check!(log.iter().position(|e| e == "frame-1") < log.iter().position(|e| e == "frame-2"));
    // Timers and frames were not run in the middle of synchronous code.
    check!(log[0] == "sync-end");
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let perf = performance();

    // The clock: non-decreasing, and the same line as Date.
    let first = perf.now();
    let mut last = first;
    let mut monotonic = true;
    for _ in 0..2000 {
        let now = perf.now();
        monotonic &= now >= last;
        last = now;
    }
    check!(monotonic && first > 0.0 && last >= first);
    check!((perf.time_origin() + perf.now() - js::date_now()).abs() < 50.0);
    // A busy wait of a few milliseconds shows in now().
    let start = perf.now();
    while perf.now() - start < 5.0 {}
    check!(perf.now() - start >= 5.0);

    // Timeouts: scheduled out of order, they run by delay. The id is a positive number.
    let started = perf.now();
    let id40 = set_timeout(due("timeout-40"), Some(40.0), JsArray::new());
    let id20 = set_timeout(function(|| log("plain-20")), Some(20.0), JsArray::new());
    let id0 = set_timeout(due("timeout-0"), Some(0.0), JsArray::new());
    let id0b = set_timeout(due("timeout-0b"), None, JsArray::new());
    check!(id40 > 0.0 && id20 > id40 && id0 > id20 && id0b > id0);
    set_timeout(
        function({
            let p = performance();
            move || ELAPSED.with(|e| e.set(p.now() - started))
        }),
        Some(41.0),
        JsArray::new(),
    );
    PENDING.with(|p| p.set(p.get() + 1));
    set_timeout(function(|| PENDING.with(|p| p.set(p.get() - 1))), Some(45.0), JsArray::new());
    // A microtask queued inside a timer runs right after it, before the next timer. The plain
    // timer made at 20ms is canceled; this one takes its place.
    clear_timeout_with(id20);
    PENDING.with(|p| p.set(p.get() + 2));
    set_timeout(
        function(|| {
            log("timeout-20");
            let m = VoidFunction::new(|| {
                log("micro-in-timer");
                PENDING.with(|p| p.set(p.get() - 1));
            });
            queue_microtask(&m);
            PENDING.with(|p| p.set(p.get() - 1));
        }),
        Some(20.0),
        JsArray::new(),
    );
    // clearTimeout stops one from running.
    let doomed = set_timeout(function(|| log("cleared")), Some(10.0), JsArray::new());
    clear_timeout_with(doomed);
    // Clearing an id that is gone, or none at all, does nothing.
    clear_timeout_with(doomed);
    clear_timeout();
    clear_timeout_with(987654.0);

    // An interval: three ticks, then it stops itself.
    let interval_id = Rc::new(Cell::new(0.0));
    PENDING.with(|p| p.set(p.get() + 1));
    let id = set_interval(
        function({
            let interval_id = interval_id.clone();
            move || {
                let n = TICKS.with(|t| {
                    t.set(t.get() + 1);
                    t.get()
                });
                log(&format!("tick-{n}"));
                if n == 3 {
                    clear_interval_with(interval_id.get());
                    PENDING.with(|p| p.set(p.get() - 1));
                }
            }
        }),
        Some(15.0),
        JsArray::new(),
    );
    interval_id.set(id);
    check!(id > 0.0);
    // An interval cleared before it ticks never does.
    let never = set_interval(function(|| log("tick-never")), Some(5.0), JsArray::new());
    clear_interval_with(never);

    // Microtasks run after this code, in order, ahead of every timer; a nested one ahead of timers too.
    let first_micro = VoidFunction::new(|| {
        log("micro-1");
        let nested = VoidFunction::new(|| log("micro-nested"));
        queue_microtask(&nested);
    });
    let second_micro = VoidFunction::new(|| log("micro-2"));
    queue_microtask(&first_micro);
    queue_microtask(&second_micro);
    check!(LOG.with(|l| l.borrow().is_empty()));

    // Animation frames: the first runs, the canceled does not, the second chains from the first.
    PENDING.with(|p| p.set(p.get() + 2));
    let second = FrameRequestCallback::new(|t| {
        FRAMES.with(|f| f.borrow_mut().push(t));
        log("frame-2");
        PENDING.with(|p| p.set(p.get() - 1));
    });
    let first_frame = FrameRequestCallback::new(move |t| {
        FRAMES.with(|f| f.borrow_mut().push(t));
        log("frame-1");
        request_animation_frame(&second);
        PENDING.with(|p| p.set(p.get() - 1));
    });
    let canceled = FrameRequestCallback::new(|_| log("frame-canceled"));
    let frame_id = request_animation_frame(&first_frame);
    let canceled_id = request_animation_frame(&canceled);
    check!(frame_id > 0.0 && canceled_id > frame_id);
    cancel_animation_frame(canceled_id);

    log("sync-end");
    check!(LOG.with(|l| l.borrow().len()) == 1);
}
