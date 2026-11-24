// Drives a real page through the generated DOM bindings; every check! must hold.
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
    let doc = document();
    let body = doc.body();

    // A typed key selects the element type: createElement("div") is an HTMLDivElement.
    let div: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    let el: HTMLElement = div.into();
    el.set_id("made-by-rust");
    el.set_text_content("hello from rust");
    el.class_list().add(["box", "made"]);
    body.append_child(el.clone());
    check!(doc.get_element_by_id("made-by-rust") == Some(el.clone()));
    check!(el.class_list().contains("box") && el.class_list().contains("made"));

    // Read an element that was already in the page, and copy its text.
    let seed = doc.get_element_by_id("seed").unwrap();
    check!(seed.text_content() == "seed text");
    check!(seed.get_attribute("data-x").as_deref() == Some("42"));
    check!(seed.get_attribute("data-missing").is_none());
    let copy: HTMLParagraphElement = doc.create_element(HTMLElementTagNameMap::P, None);
    copy.set_text_content(&seed.text_content());
    el.append_child(copy.clone());

    // A subtype where its ancestor is wanted: into() moves, as_ref() borrows.
    let extra = doc.create_element(HTMLElementTagNameMap::P, None);
    el.insert_before(extra.clone(), Some(copy.as_ref()));
    check!(el.children().length() == 2.0);
    extra.remove();
    check!(el.children().length() == 1.0);

    // Generic types are generic: querySelectorAll("p") is a NodeListOf<HTMLParagraphElement>.
    let paragraphs = el.query_selector_all(HTMLElementTagNameMap::P);
    check!(paragraphs.length() == 1.0);
    check!(paragraphs.item(0.0).text_content() == "seed text");

    // The same method takes a selector and finds an Element; the type it is said to be is a cast.
    check!(el.query_selector("p") == Some(copy.clone().into()));
    check!(el.query_selector("p").map(|e| js::unchecked_cast::<HTMLParagraphElement>(&e)) == Some(copy.clone()));
    check!(el.query_selector_all("p, span").length() == 1.0);

    // A typed key selects the event type: the listener of "click" takes a PointerEvent.
    let button: HTMLElement = doc
        .create_element(HTMLElementTagNameMap::BUTTON, None)
        .into();
    button.set_text_content("press");
    el.append_child(button.clone());
    let clicks = Rc::new(Cell::new(0));
    let on_click = JsListener::new({
        let (clicks, el) = (clicks.clone(), el.clone());
        move |_: PointerEvent| {
            clicks.set(clicks.get() + 1);
            el.set_attribute("data-count", &clicks.get().to_string());
        }
    });
    button.add_event_listener(HTMLElementEventMap::CLICK, &on_click, None);
    button.click();
    button.click();
    check!(clicks.get() == 2);
    button.remove_event_listener(HTMLElementEventMap::CLICK, &on_click, None);
    button.click();
    check!(clicks.get() == 2);
    // Frees the closure.
    on_click.release();

    // An options object, { once: true }, and events made and dispatched here.
    let pings = Rc::new(Cell::new(0));
    let once = AddEventListenerOptions::create();
    once.set_once(true);
    let on_ping = EventListener::new({
        let (pings, el) = (pings.clone(), el.clone());
        move |_: Event| {
            pings.set(pings.get() + 1);
            el.set_attribute("data-pings", &pings.get().to_string());
        }
    });
    // One method: a typed key and its listener, or a string and any listener.
    // An optional union argument is any member, or None.
    el.add_event_listener("ping", on_ping, &once);
    el.dispatch_event(&Event::new("ping"));
    el.dispatch_event(&Event::new("ping"));
    check!(pings.get() == 1);

    // Style, layout and the window.
    el.style().set_property("color", "red");
    check!(el.style().color() == "red");
    check!(el.get_bounding_client_rect().width() > 0.0);
    check!(el.children().length() == 2.0);
    check!(inner_width() > 0.0);

    // Collections are iterators.
    let tags: Vec<String> = el.children().iter().map(|child| child.tag_name()).collect();
    check!(tags == ["P", "BUTTON"]);
    doc.set_title("driven by rust");

    // An invalid selector throws inside attempt: caught as a JsError without leaking to the error slot.
    check!(
        js::attempt(|| el.query_selector("<bad"))
            .is_err_and(|e| e.message().contains("not a valid selector"))
    );
    check!(js::take_error().is_none());

    // A real request: a promise of a Response, then of its text, each awaited.
    js::spawn(async {
        let Ok(response) = fetch("./verify.mjs").await else {
            return;
        };
        let text = response.text().await;
        FETCHED.with(|f| {
            f.set(response.status() == 200.0 && text.is_ok_and(|t| t.contains("verify")))
        });
    });

    // Handles are counted: what goes out of scope is freed.
    let live = js::live_handles();
    for _ in 0..500 {
        let found = doc.get_element_by_id("made-by-rust").unwrap();
        check!(found.class_list().contains("box") && found.id() == "made-by-rust");
    }
    check!(js::live_handles() == live);
}
