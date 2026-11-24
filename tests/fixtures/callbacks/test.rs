// Calls the generated Rust bindings; every check! must hold.
#![no_main]
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
    static CHAINED: Cell<f64> = const { Cell::new(0.0) };
    static FREED: Cell<bool> = const { Cell::new(false) };
}

// Set when a closure is dropped: JavaScript collected its function.
struct Freed;
impl Drop for Freed {
    fn drop(&mut self) {
        FREED.with(|f| f.set(true));
    }
}

// Nonzero until the dropped closure has been freed.
#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    i32::from(!FREED.with(Cell::get))
}

// Promises are awaited, one after another; a rejection is an Err.
async fn chain() -> Result<f64, JsError> {
    let first = delayed(10.0).await?;
    let second = delayed(first).await?;
    Ok(second
        + rejected("stop")
            .await
            .map_or_else(|e| e.message().len() as f64, |_| 0.0))
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(AWAITED.with(Cell::get) == 42.0);
    check!(REASON.with(Cell::get));
    check!(SETTLED.with(Cell::get));
    check!(TICKS.with(Cell::get) == 6.0);
    check!(CHAINED.with(Cell::get) == 44.0);
    // A closure made with `new` and never released is freed with its function.
    check!(FREED.with(Cell::get));
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Kept by nothing once dropped: JavaScript collects it, and its closure is freed.
    let guard = Freed;
    drop(Listener::new(move |_: f64| {
        let _ = &guard;
    }));
    // Promises: a closure is called with the Result once one settles.
    delayed(21.0).then(|r| AWAITED.with(|a| a.set(r.unwrap_or(0.0))));
    rejected("nope").then(|r| REASON.with(|a| a.set(matches!(r, Err(e) if e.message() == "nope"))));
    settled().then(|r| SETTLED.with(|a| a.set(r.is_ok())));
    // In an async block they are awaited: `js::spawn` runs it across them.
    js::spawn(async {
        let total = chain().await.unwrap_or(-1.0);
        CHAINED.with(|c| c.set(total));
    });
    js::spawn(async {
        let ticks = ticker(3.0).iterate_async();
        while let Ok(Some(v)) = ticks.next().await {
            TICKS.with(|t| t.set(t.get() + v));
        }
    });
    check!(AWAITED.with(Cell::get) == 0.0);

    // An object that is also a function, and can be constructed.
    let s = scaler(2.0);
    check!(s.invoke(5.0) == 10.0 && s.factor() == 2.0 && s.construct(3.0).invoke(5.0) == 15.0);

    // A call that throws inside attempt gives a Result with the exception;
    // the error slot stays empty afterward.
    check!(js::take_error().is_none());
    check!(js::attempt(|| explode("boom")).is_err_and(|e| e.message() == "boom"));
    check!(js::take_error().is_none());
    check!(js::attempt(|| explode("again")).is_err_and(|e| e.message() == "again"));
    check!(js::attempt(flush).is_ok());

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
    check!(c.increment() == 1.0);
    check!(last.get() == 1.0);
    c.increment_with(4.0);
    check!(last.get() == 5.0 && calls.get() == 2);

    // A callback that takes an object and returns a value.
    check!(c.visit(&CounterVisitF::new(
        |counter: Counter| counter.count() == 5.0
    )));

    // For the length of a call a closure may borrow, and nothing is kept.
    let live = js::live_handles();
    let mut seen = Vec::new();
    let folded = CounterFoldF::scoped(
        |total, label: String| {
            seen.push(label);
            total
        },
        |f| c.fold(f),
    );
    check!(folded == 5.0 && seen == ["count"]);
    check!(js::live_handles() == live);

    // The same Listener is the same JavaScript function; release frees the closure.
    c.unsubscribe(&on_value);
    c.increment();
    check!(last.get() == 5.0 && calls.get() == 2);
    on_value.release();

    // A nullable callback property.
    check!(c.onchange().is_none());
    let prop = Rc::new(Cell::new(0.0));
    c.set_onchange(Some(&Listener::new({
        let prop = prop.clone();
        move |value: f64| prop.set(value)
    })));
    check!(c.onchange().is_some());
    c.increment();
    check!(prop.get() == 7.0);
    c.set_onchange(None);
    c.increment();
    check!(prop.get() == 7.0);

    // Several arguments, a string among them, and a result.
    check!(
        c.fold(&CounterFoldF::new(
            |total: f64, label: String| total * 2.0 + label.len() as f64
        )) == 8.0 * 2.0 + 5.0
    );

    // A callback kept and called later.
    let ran = Rc::new(Cell::new(0));
    for _ in 0..2 {
        let ran = ran.clone();
        later(&LaterF::new(move || ran.set(ran.get() + 1)));
    }
    check!(ran.get() == 0);
    check!(flush() == 2.0);
    check!(ran.get() == 2);

    // A union argument arrives as the enum.
    let number = Rc::new(Cell::new(0.0));
    let text = Rc::new(Cell::new(0usize));
    let each = EachIdF::new({
        let (number, text) = (number.clone(), text.clone());
        move |id: Id| match id {
            Id::Number(n) => number.set(n),
            Id::String(s) => text.set(s.len()),
        }
    });
    check!(each_id(&each) == 2.0);
    check!(number.get() == 1.0 && text.get() == 3);
}
