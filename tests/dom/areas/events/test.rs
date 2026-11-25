// Events from Rust: listeners and their options, events made and dispatched,
// bubbling, cancelation and an AbortSignal. Every check! must hold.
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

fn counter(n: &Rc<Cell<i32>>) -> EventListener {
    let n = n.clone();
    EventListener::new(move |_: Event| n.set(n.get() + 1))
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();
    let outer: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    let inner: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    outer.append_child(inner.clone());
    body.append_child(outer.clone());

    // A plain listener, added, fired and removed.
    let hits = Rc::new(Cell::new(0));
    let on_a = counter(&hits);
    inner.add_event_listener("a", on_a.clone(), None);
    check!(inner.dispatch_event(&Event::new("a")));
    check!(hits.get() == 1);
    inner.dispatch_event(&Event::new("a"));
    check!(hits.get() == 2);
    inner.remove_event_listener("a", on_a.clone(), None);
    inner.dispatch_event(&Event::new("a"));
    check!(hits.get() == 2);
    // Another type does not reach it.
    inner.add_event_listener("a", on_a.clone(), None);
    inner.dispatch_event(&Event::new("b"));
    check!(hits.get() == 2);
    inner.remove_event_listener("a", on_a.clone(), None);

    // The same listener added twice is one registration.
    let twice = Rc::new(Cell::new(0));
    let l = counter(&twice);
    inner.add_event_listener("t", l.clone(), None);
    inner.add_event_listener("t", l.clone(), None);
    inner.dispatch_event(&Event::new("t"));
    check!(twice.get() == 1);
    inner.remove_event_listener("t", l.clone(), None);

    // once: fires one time.
    let onces = Rc::new(Cell::new(0));
    let once_opts = AddEventListenerOptions::create();
    once_opts.set_once(Some(true));
    check!(once_opts.once() == Some(true));
    check!(once_opts.passive().is_none());
    inner.add_event_listener("o", counter(&onces), &once_opts);
    for _ in 0..3 {
        inner.dispatch_event(&Event::new("o"));
    }
    check!(onces.get() == 1);

    // Bubbling: an event that bubbles reaches the parent; one that does not, stays.
    let order: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));
    let log = |name: &'static str| {
        let order = order.clone();
        EventListener::new(move |e: Event| order.borrow_mut().push(format!("{name}:{}", e.event_phase())))
    };
    let on_outer = log("outer");
    let on_inner = log("inner");
    outer.add_event_listener("x", on_outer.clone(), None);
    inner.add_event_listener("x", on_inner.clone(), None);
    inner.dispatch_event(&Event::new("x"));
    check!(order.borrow().is_empty() == false && *order.borrow() == ["inner:2"]);
    order.borrow_mut().clear();
    let bubbling = EventInit::create();
    bubbling.set_bubbles(Some(true));
    check!(bubbling.bubbles() == Some(true));
    let ev = Event::new_with("x", &bubbling);
    check!(ev.bubbles() && !ev.cancelable() && ev.type_() == "x");
    check!(ev.target().is_none() && !ev.is_trusted());
    inner.dispatch_event(&ev);
    check!(*order.borrow() == ["inner:2", "outer:3"]);
    check!(ev.target() == Some(inner.clone().into()));
    // The path is only kept while the event is dispatched.
    check!(ev.composed_path().len() == 0);

    // Capture runs the ancestor first; the listener is removed with the same flag.
    order.borrow_mut().clear();
    let capture = AddEventListenerOptions::create();
    capture.set_capture(Some(true));
    let on_cap = log("capture");
    outer.add_event_listener("x", on_cap.clone(), &capture);
    inner.dispatch_event(&Event::new_with("x", &bubbling));
    check!(*order.borrow() == ["capture:1", "inner:2", "outer:3"]);
    outer.remove_event_listener("x", on_cap.clone(), true);
    order.borrow_mut().clear();
    inner.dispatch_event(&Event::new_with("x", &bubbling));
    check!(*order.borrow() == ["inner:2", "outer:3"]);

    // stopPropagation keeps the event from the parent; stopImmediatePropagation from its siblings.
    outer.remove_event_listener("x", on_outer.clone(), None);
    inner.remove_event_listener("x", on_inner.clone(), None);
    let reached_outer = Rc::new(Cell::new(0));
    let second = Rc::new(Cell::new(0));
    outer.add_event_listener("s", counter(&reached_outer), None);
    let stopper = EventListener::new(|e: Event| e.stop_propagation());
    inner.add_event_listener("s", stopper.clone(), None);
    inner.add_event_listener("s", counter(&second), None);
    inner.dispatch_event(&Event::new_with("s", &bubbling));
    check!(reached_outer.get() == 0 && second.get() == 1);
    inner.remove_event_listener("s", stopper.clone(), None);
    let immediate = EventListener::new(|e: Event| e.stop_immediate_propagation());
    inner.add_event_listener("i", immediate, None);
    inner.add_event_listener("i", counter(&second), None);
    inner.dispatch_event(&Event::new("i"));
    check!(second.get() == 1);

    // preventDefault only counts on a cancelable event; dispatchEvent returns false when canceled.
    let cancelable = EventInit::create();
    cancelable.set_cancelable(Some(true));
    let preventer = EventListener::new(|e: Event| e.prevent_default());
    inner.add_event_listener("p", preventer.clone(), None);
    let plain = Event::new("p");
    check!(inner.dispatch_event(&plain) && !plain.default_prevented());
    let cancel = Event::new_with("p", &cancelable);
    check!(cancel.cancelable() && !cancel.default_prevented());
    check!(!inner.dispatch_event(&cancel));
    check!(cancel.default_prevented());

    // A CustomEvent carries a detail. A listener gets an Event, downcast via CustomEvent::cast.
    let seen = Rc::new(RefCell::new(String::new()));
    let on_custom = EventListener::new({
        let seen = seen.clone();
        move |e: Event| {
            check!(MouseEvent::cast(&e).is_none());
            let custom = CustomEvent::<String>::cast(&e).unwrap();
            *seen.borrow_mut() = custom.detail();
        }
    });
    inner.add_event_listener("custom", on_custom, None);
    let init = CustomEventInit::<String>::create();
    init.set_detail(Some("payload".to_string()));
    check!(init.detail().as_deref() == Some("payload"));
    inner.dispatch_event(&CustomEvent::<String>::new_with("custom", &init));
    check!(*seen.borrow() == "payload");
    let numeric = CustomEventInit::<f64>::create();
    numeric.set_detail(Some(41.5));
    check!(CustomEvent::<f64>::new_with("n", &numeric).detail() == 41.5);

    // A MouseEvent made with an init dict, seen by a typed listener.
    let clicks: Rc<RefCell<Vec<(f64, f64, f64, bool, bool)>>> = Rc::new(RefCell::new(Vec::new()));
    let on_click = JsListener::new({
        let clicks = clicks.clone();
        move |e: PointerEvent| clicks.borrow_mut().push((e.client_x(), e.client_y(), e.button(), e.ctrl_key(), e.shift_key()))
    });
    let mouse = MouseEventInit::create();
    mouse.set_client_x(Some(12.0));
    mouse.set_client_y(Some(34.0));
    mouse.set_button(Some(1.0));
    mouse.set_ctrl_key(Some(true));
    mouse.set_bubbles(Some(true));
    check!(mouse.client_x() == Some(12.0) && mouse.ctrl_key() == Some(true));
    // The typed key "click" says a PointerEvent; a MouseEvent is dispatched here, so use a string.
    let on_click_any = EventListener::new({
        let clicks = clicks.clone();
        move |e: Event| {
            let m = MouseEvent(e.0.clone());
            clicks.borrow_mut().push((m.client_x(), m.client_y(), m.button(), m.ctrl_key(), m.shift_key()))
        }
    });
    outer.add_event_listener("click", on_click_any.clone(), None);
    inner.dispatch_event(&MouseEvent::new_with("click", &mouse));
    check!(*clicks.borrow() == [(12.0, 34.0, 1.0, true, false)]);
    // A real click() produces a PointerEvent for the typed listener.
    clicks.borrow_mut().clear();
    outer.add_event_listener(HTMLElementEventMap::CLICK, &on_click, None);
    inner.click();
    check!(clicks.borrow().len() == 2);
    check!(clicks.borrow()[0].2 == 0.0 && !clicks.borrow()[0].3);
    outer.remove_event_listener(HTMLElementEventMap::CLICK, &on_click, None);
    outer.remove_event_listener("click", on_click_any.clone(), None);

    // A KeyboardEvent made with an init dict.
    let keys = Rc::new(RefCell::new(Vec::<String>::new()));
    let on_key = JsListener::new({
        let keys = keys.clone();
        move |e: KeyboardEvent| keys.borrow_mut().push(format!("{}/{}/{}/{}/{}", e.key(), e.code(), e.shift_key(), e.repeat(), e.location()))
    });
    outer.add_event_listener(HTMLElementEventMap::KEYDOWN, &on_key, None);
    let key = KeyboardEventInit::create();
    key.set_key(Some("A"));
    key.set_code(Some("KeyA"));
    key.set_shift_key(Some(true));
    key.set_repeat(Some(true));
    key.set_location(Some(KeyboardEvent::DOM_KEY_LOCATION_LEFT));
    key.set_bubbles(Some(true));
    check!(key.key().as_deref() == Some("A"));
    let down = KeyboardEvent::new_with("keydown", &key);
    check!(down.key() == "A" && down.code() == "KeyA" && down.shift_key() && down.repeat());
    check!(down.get_modifier_state("Shift") && !down.get_modifier_state("Control"));
    inner.dispatch_event(&down);
    check!(*keys.borrow() == ["A/KeyA/true/true/1"]);
    outer.remove_event_listener(HTMLElementEventMap::KEYDOWN, &on_key, None);
    inner.dispatch_event(&KeyboardEvent::new_with("keydown", &key));
    check!(keys.borrow().len() == 1);

    // An AbortSignal removes its listener when it aborts.
    let controller = AbortController::new();
    let signal = controller.signal();
    check!(!signal.aborted());
    let aborts = Rc::new(Cell::new(0));
    let with_signal = AddEventListenerOptions::create();
    with_signal.set_signal(Some(&signal));
    check!(with_signal.signal() == Some(signal.clone()));
    inner.add_event_listener("sig", counter(&aborts), &with_signal);
    inner.dispatch_event(&Event::new("sig"));
    check!(aborts.get() == 1);
    let on_abort_seen = Rc::new(Cell::new(0));
    signal.add_event_listener("abort", counter(&on_abort_seen), None);
    controller.abort();
    check!(signal.aborted() && on_abort_seen.get() == 1);
    inner.dispatch_event(&Event::new("sig"));
    check!(aborts.get() == 1);
    check!(js::attempt(|| signal.throw_if_aborted()).is_err());

    // A window-level event and a listener on document.
    let docs = Rc::new(Cell::new(0));
    doc.add_event_listener("ping", counter(&docs), None);
    doc.dispatch_event(&Event::new("ping"));
    check!(docs.get() == 1);

    // A passive listener may not prevent default: the request is ignored.
    let passive = AddEventListenerOptions::create();
    passive.set_passive(Some(true));
    check!(passive.passive() == Some(true));
    inner.add_event_listener("pp", preventer.clone(), &passive);
    let pe = Event::new_with("pp", &cancelable);
    check!(inner.dispatch_event(&pe) && !pe.default_prevented());
    on_click.release();
    on_key.release();
}
