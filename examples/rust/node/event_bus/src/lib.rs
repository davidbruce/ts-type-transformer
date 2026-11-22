#![no_main]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use node::events::EventEmitter;
use node::{AnyArrayFn, JsArray, JsValue};

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "print"]
    fn host_print(ptr: *const u8, len: usize);
}

fn println(s: &str) {
    unsafe {
        host_print(s.as_ptr(), s.len());
        host_print(b"\n".as_ptr(), 1);
    }
}

struct Checks(u32);
impl Checks {
    fn check(&mut self, ok: bool) {
        if !ok {
            panic!("check failed");
        }
        self.0 += 1;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let mut c = Checks(0);

    let emitter = EventEmitter::<JsValue>::new(None);
    emitter.set_max_listeners(12.0);
    c.check(emitter.get_max_listeners() == 12.0);

    let metric_calls = Rc::new(Cell::new(0u32));
    let metric_sum = Rc::new(RefCell::new(0.0f64));
    let once_calls = Rc::new(Cell::new(0u32));
    let shutdown_calls = Rc::new(Cell::new(0u32));

    let on_metric = AnyArrayFn::new({
        let metric_calls = metric_calls.clone();
        let metric_sum = metric_sum.clone();
        move |args: JsArray| {
            metric_calls.set(metric_calls.get() + 1);
            if !args.is_empty() {
                *metric_sum.borrow_mut() += args.get(0).as_f64();
            }
        }
    });
    let on_once = AnyArrayFn::new({
        let once_calls = once_calls.clone();
        move |_: JsArray| once_calls.set(once_calls.get() + 1)
    });
    let on_shutdown = AnyArrayFn::new({
        let shutdown_calls = shutdown_calls.clone();
        move |_: JsArray| shutdown_calls.set(shutdown_calls.get() + 1)
    });

    emitter.on("metric", &on_metric);
    emitter.once("metric", &on_once);
    emitter.on("shutdown", &on_shutdown);

    c.check(emitter.listener_count("metric", None) == 2.0);
    c.check(emitter.event_names().len() == 2);

    let emitted1 = emitter.emit("metric", [JsValue::from(101.0), JsValue::from("ok")]);
    c.check(emitted1 && metric_calls.get() == 1 && once_calls.get() == 1);
    c.check(emitter.listener_count("metric", None) == 1.0);

    let emitted2 = emitter.emit("metric", [JsValue::from(202.0), JsValue::from("ok")]);
    c.check(
        emitted2
            && metric_calls.get() == 2
            && *metric_sum.borrow() == 303.0
            && once_calls.get() == 1,
    );

    c.check(!emitter.emit("idle", JsArray::new()));

    emitter.remove_listener("metric", &on_metric);
    c.check(
        emitter.listener_count("metric", None) == 0.0
            && !emitter.emit("metric", [1.0]),
    );
    c.check(metric_calls.get() == 2);

    emitter.remove_all_listeners("shutdown");
    c.check(emitter.event_names().is_empty() && shutdown_calls.get() == 0);

    println(&format!(
        "maxListeners={:.0} metricCalls={} metricSum={:.0}",
        emitter.get_max_listeners(),
        metric_calls.get(),
        *metric_sum.borrow()
    ));
    println(&format!(
        "onceCalls={} shutdownCalls={}",
        once_calls.get(),
        shutdown_calls.get()
    ));
    println(&format!("PASS: event_bus ({} checks)", c.0));
}
