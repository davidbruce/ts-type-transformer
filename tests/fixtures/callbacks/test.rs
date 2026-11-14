// Calls the generated Rust bindings; every check! must hold.
#![no_main]
mod bindings;
use bindings::*;
use std::cell::Cell;
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

// What the asynchronous code saw, checked by `finish`.
thread_local! {
    static AWAITED: Cell<f64> = const { Cell::new(0.0) };
    static REASON: Cell<bool> = const { Cell::new(false) };
    static SETTLED: Cell<bool> = const { Cell::new(false) };
    static TICKS: Cell<f64> = const { Cell::new(0.0) };
}

// Asks for each value of an async iterator when the last arrives.
fn next_tick(it: AsyncIterator<f64>) {
    let p = it.next();
    p.then(move |r| {
        if let Ok(Some(v)) = r {
            TICKS.with(|t| t.set(t.get() + v));
            next_tick(it);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(AWAITED.with(Cell::get) == 42.0);
    check!(REASON.with(Cell::get));
    check!(SETTLED.with(Cell::get));
    check!(TICKS.with(Cell::get) == 6.0);
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Promises: a closure is called with the Result once one settles.
    delayed(21.0).then(|r| AWAITED.with(|a| a.set(r.unwrap_or(0.0))));
    rejected("nope").then(|r| REASON.with(|a| a.set(matches!(r, Err(e) if e.message() == "nope"))));
    settled().then(|r| SETTLED.with(|a| a.set(r.is_ok())));
    next_tick(ticker(3.0).iterate_async());
    check!(AWAITED.with(Cell::get) == 0.0);

    // An object that is also a function, and can be constructed.
    let s = scaler(2.0);
    check!(s.invoke(5.0) == 10.0 && s.get_factor() == 2.0 && s.construct(3.0).invoke(5.0) == 15.0);

    // A call that throws returns a zero value; its exception is kept to take.
    check!(take_error().is_none());
    check!(explode("boom") == 0.0);
    check!(take_error().map(|e| e.message()) == Some("boom".to_string()));
    check!(take_error().is_none());

    let c = Counter::new();
    let last = Rc::new(Cell::new(0.0));
    let calls = Rc::new(Cell::new(0));

    // A callback is a closure; keep it to pass the same one again.
    let on_value = Listener::new({
        let (last, calls) = (last.clone(), calls.clone());
        move |value: f64| {
            last.set(value);
            calls.set(calls.get() + 1);
        }
    });
    c.subscribe(&on_value);
    check!(c.increment(None) == 1.0);
    check!(last.get() == 1.0);
    c.increment(Some(4.0));
    check!(last.get() == 5.0 && calls.get() == 2);

    // A callback that takes an object and returns a value.
    check!(c.visit(&Counter_visit_f::new(|counter: Counter| counter.get_count() == 5.0)));

    // The same Listener is the same JavaScript function; release frees the closure.
    c.unsubscribe(&on_value);
    c.increment(None);
    check!(last.get() == 5.0 && calls.get() == 2);
    on_value.release();

    // A nullable callback property.
    check!(c.get_onchange().is_none());
    let prop = Rc::new(Cell::new(0.0));
    c.set_onchange(Some(Listener::new({
        let prop = prop.clone();
        move |value: f64| prop.set(value)
    })));
    check!(c.get_onchange().is_some());
    c.increment(None);
    check!(prop.get() == 7.0);
    c.set_onchange(None);
    c.increment(None);
    check!(prop.get() == 7.0);

    // Several arguments, a string among them, and a result.
    check!(c.fold(&Counter_fold_f::new(|total: f64, label: String| total * 2.0 + label.len() as f64)) == 8.0 * 2.0 + 5.0);

    // A callback kept and called later.
    let ran = Rc::new(Cell::new(0));
    for _ in 0..2 {
        let ran = ran.clone();
        later(&later_f::new(move || ran.set(ran.get() + 1)));
    }
    check!(ran.get() == 0);
    check!(flush() == 2.0);
    check!(ran.get() == 2);

    // A union argument arrives as the enum.
    let number = Rc::new(Cell::new(0.0));
    let text = Rc::new(Cell::new(0usize));
    let each = eachId_f::new({
        let (number, text) = (number.clone(), text.clone());
        move |id: Id| match id {
            Id::number(n) => number.set(n),
            Id::string(s) => text.set(s.len()),
        }
    });
    check!(eachId(&each) == 2.0);
    check!(number.get() == 1.0 && text.get() == 3);
}
