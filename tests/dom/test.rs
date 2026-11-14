// Drives a real page through the generated DOM bindings; every check! must hold.
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

// What a real request saw, checked by `finish`.
thread_local! {
    static FETCHED: Cell<bool> = const { Cell::new(false) };
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(FETCHED.with(Cell::get));
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = get_document();
    let body = doc.get_body();

    // A typed key selects the element type: createElement("div") is an HTMLDivElement.
    let div: HTMLDivElement = doc.createElement(HTMLElementTagNameMapKeys::div, None);
    let el: HTMLElement = div.into();
    el.set_id("made-by-rust");
    el.set_textContent(Some("hello from rust"));
    el.get_classList().add(["box", "made"].into_iter().collect());
    body.appendChild(el.clone());
    check!(doc.getElementById("made-by-rust").map(|e| e.0 .0) == Some(el.0 .0));
    check!(el.get_classList().contains("box") && el.get_classList().contains("made"));

    // Read an element that was already in the page, and copy its text.
    let seed = doc.getElementById("seed").unwrap();
    check!(seed.get_textContent() == "seed text");
    check!(seed.getAttribute("data-x").as_deref() == Some("42"));
    check!(seed.getAttribute("data-missing").is_none());
    let copy: HTMLParagraphElement = doc.createElement(HTMLElementTagNameMapKeys::p, None);
    copy.set_textContent(Some(&seed.get_textContent()));
    el.appendChild(copy.clone());

    // A subtype where its ancestor is wanted: into() moves, as_ref() borrows.
    let extra: HTMLParagraphElement = doc.createElement(HTMLElementTagNameMapKeys::p, None);
    el.insertBefore(extra.clone(), Some(copy.clone().into()));
    check!(el.get_children().get_length() == 2.0);
    extra.remove();
    check!(el.get_children().get_length() == 1.0);

    // Generic types are generic: querySelectorAll("p") is a NodeListOf<HTMLParagraphElement>.
    let paragraphs = el.querySelectorAll(HTMLElementTagNameMapKeys::p);
    check!(paragraphs.get_length() == 1.0);
    check!(paragraphs.item(0.0).get_textContent() == "seed text");

    // A typed key selects the event type: the listener of "click" takes a PointerEvent.
    let button: HTMLElement = doc.createElement(HTMLElementTagNameMapKeys::button, None).into();
    button.set_textContent(Some("press"));
    el.appendChild(button.clone());
    let clicks = Rc::new(Cell::new(0));
    let on_click = Listener::new({
        let (clicks, el) = (clicks.clone(), el.clone());
        move |_: PointerEvent| {
            clicks.set(clicks.get() + 1);
            el.setAttribute("data-count", &clicks.get().to_string());
        }
    });
    button.addEventListener(HTMLElementEventMapKeys::click, on_click.clone(), None);
    button.click();
    button.click();
    check!(clicks.get() == 2);
    button.removeEventListener(HTMLElementEventMapKeys::click, on_click.clone(), None);
    button.click();
    check!(clicks.get() == 2);
    // Frees the closure.
    on_click.release();

    // An options object, { once: true }, and events made and dispatched here.
    let pings = Rc::new(Cell::new(0));
    let once = AddEventListenerOptions::create();
    once.set_once(Some(true));
    let on_ping = EventListener::new({
        let (pings, el) = (pings.clone(), el.clone());
        move |_: Event| {
            pings.set(pings.get() + 1);
            el.setAttribute("data-pings", &pings.get().to_string());
        }
    });
    el.addEventListener_with_string("ping", EventListenerOrEventListenerObject::EventListener(on_ping), Some(boolean_or_AddEventListenerOptions::AddEventListenerOptions(once)));
    el.dispatchEvent(&Event::new("ping", None));
    el.dispatchEvent(&Event::new("ping", None));
    check!(pings.get() == 1);

    // Style, layout and the window.
    el.get_style().setProperty("color", Some("red"), None);
    check!(el.get_style().get_color() == "red");
    check!(el.getBoundingClientRect().get_width() > 0.0);
    check!(el.get_children().get_length() == 2.0);
    check!(get_innerWidth() > 0.0);

    // Collections are iterators.
    let tags: Vec<String> = el.get_children().iter().map(|child| child.get_tagName()).collect();
    check!(tags == ["P", "BUTTON"]);
    doc.set_title("driven by rust");

    // An invalid selector throws: the call returns None, and the exception is kept.
    check!(el.querySelector_with_string::<Element>("<bad").is_none());
    check!(take_error().is_some_and(|e| e.message().contains("not a valid selector")));

    // A real request: a promise of a Response, then of its text.
    fetch(Request_or_string_or_URL::string("./verify.mjs".into()), None).then(|r| {
        if let Ok(response) = r {
            let ok = response.get_status() == 200.0;
            response.text().then(move |t| FETCHED.with(|f| f.set(ok && t.is_ok_and(|t| t.contains("verify")))));
        }
    });

    // Handles are counted: what goes out of scope is freed.
    let live = live_handles();
    for _ in 0..500 {
        let found = doc.getElementById("made-by-rust").unwrap();
        check!(found.get_classList().contains("box") && found.get_id() == "made-by-rust");
    }
    check!(live_handles() == live);
}
