// Observers from Rust: mutations seen as records, size and visibility seen
// by callbacks, and a signal that times out. `busy` says how many callbacks
// are still due; `finish` checks what they saw. Every check! must hold.
#![no_main]
use bindings::*;
use std::cell::{Cell, RefCell};

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
    static SEEN: RefCell<Vec<String>> = const { RefCell::new(Vec::new()) };
    static KEEP: RefCell<Vec<JsHandle>> = const { RefCell::new(Vec::new()) };
}

fn see(entry: String) {
    SEEN.with(|s| s.borrow_mut().push(entry));
}
fn seen(prefix: &str) -> Vec<String> {
    SEEN.with(|s| s.borrow().iter().filter(|e| e.starts_with(prefix)).cloned().collect())
}
fn due() {
    PENDING.with(|p| p.set(p.get() + 1));
}
fn arrived() {
    PENDING.with(|p| p.set(p.get() - 1));
}

#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    PENDING.with(Cell::get)
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(busy() == 0);
    // MutationObserver: one callback for the batch, in the order the changes were made (the
    // last is made while a second observer is also watching, which is then disconnected).
    check!(seen("mutation:") == ["mutation:childList:added=2:removed=0", "mutation:attributes:data-state:old=Some(\"a\")", "mutation:attributes:data-state:old=Some(\"b\")", "mutation:childList:added=0:removed=1", "mutation:attributes:data-other:old=None"]);
    check!(seen("mutation-batch") == ["mutation-batch:5"]);
    check!(seen("after-disconnect").is_empty());
    // ResizeObserver: first call on observe, with the starting size; again when it changes.
    check!(seen("resize:") == ["resize:target=box:content=120x40:border=124x44", "resize:target=box:content=60x40:border=64x44"]);
    // IntersectionObserver: the visible one is intersecting, the one far below is not.
    let mut seen_intersection = seen("intersect:");
    seen_intersection.sort();
    check!(seen_intersection == ["intersect:far:false:ratio=0", "intersect:near:true:ratio=1"]);
    // The signal timed out with a TimeoutError, once.
    check!(seen("timeout:") == ["timeout:TimeoutError"]);
    check!(seen("canceled").is_empty());
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let doc = document();
    let body = doc.body();

    // ---- MutationObserver, read with takeRecords (synchronous).
    let host: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    body.append_child(host.clone());
    let quiet = MutationObserver::new(&MutationCallback::new(|_, _| see("never-called".to_string())));
    let child_list = MutationObserverInit::create();
    child_list.set_child_list(Some(true));
    quiet.observe(&host, Some(&child_list));
    check!(quiet.take_records().len() == 0);
    let a: HTMLSpanElement = doc.create_element(HTMLElementTagNameMap::SPAN, None);
    let b: HTMLSpanElement = doc.create_element(HTMLElementTagNameMap::SPAN, None);
    host.append_child(a.clone());
    host.append_child(b.clone());
    let records = quiet.take_records();
    check!(records.len() == 2);
    let first = records.get(0);
    check!(first.type_() == MutationRecordType::ChildList && first.target() == host.clone().into());
    check!(first.added_nodes().length() == 1.0 && first.removed_nodes().length() == 0.0);
    check!(first.added_nodes().item(0.0) == Some(a.clone().into()));
    check!(first.previous_sibling().is_none() && first.next_sibling().is_none());
    check!(records.get(1).previous_sibling() == Some(a.clone().into()));
    check!(quiet.take_records().len() == 0);
    // Removal: the record names what left and where it was.
    host.remove_child(a.clone());
    let removal = quiet.take_records();
    check!(removal.len() == 1 && removal.get(0).removed_nodes().item(0.0) == Some(a.clone().into()));
    check!(removal.get(0).next_sibling() == Some(b.clone().into()));
    // Changes below the target are not seen without `subtree`; attributes are not seen without `attributes`.
    let grandchild: HTMLElement = doc.create_element(HTMLElementTagNameMap::I, None).into();
    b.append_child(grandchild.clone());
    host.set_attribute("data-x", "1");
    check!(quiet.take_records().len() == 0);
    // With subtree, they are.
    let deep = MutationObserverInit::create();
    deep.set_child_list(Some(true));
    deep.set_subtree(Some(true));
    quiet.observe(&host, Some(&deep));
    b.append_child(doc.create_text_node("t"));
    let nested = quiet.take_records();
    check!(nested.len() == 1 && nested.get(0).target() == b.clone().into());
    // Attributes: old values, and a filter on names.
    let attrs = MutationObserverInit::create();
    attrs.set_attributes(Some(true));
    attrs.set_attribute_old_value(Some(true));
    attrs.set_attribute_filter(Some(&JsArray::<String>::from(["title", "lang"])));
    check!(attrs.attributes() == Some(true) && attrs.attribute_filter().unwrap().len() == 2);
    quiet.observe(&host, Some(&attrs));
    host.set_attribute("title", "one");
    host.set_attribute("title", "two");
    host.set_attribute("data-ignored", "x");
    host.remove_attribute("title");
    let changes = quiet.take_records();
    check!(changes.len() == 3);
    let summary: Vec<String> = changes.iter().map(|r| format!("{:?}:{}:{:?}", r.type_() == MutationRecordType::Attributes, r.attribute_name().unwrap(), r.old_value())).collect();
    check!(summary == ["true:title:None", "true:title:Some(\"one\")", "true:title:Some(\"two\")"]);
    check!(changes.get(0).attribute_namespace().is_none());
    // Character data.
    let text = doc.create_text_node("before");
    host.append_child(text.clone());
    let chars = MutationObserverInit::create();
    chars.set_character_data(Some(true));
    chars.set_character_data_old_value(Some(true));
    let char_observer = MutationObserver::new(&MutationCallback::new(|_, _| ()));
    char_observer.observe(&text, Some(&chars));
    text.set_data("after");
    let edits = char_observer.take_records();
    check!(edits.len() == 1 && edits.get(0).type_() == MutationRecordType::CharacterData);
    check!(edits.get(0).old_value().as_deref() == Some("before") && edits.get(0).target() == text.clone().into());
    check!(text.data() == "after");
    // An observer with no kind of change to watch throws.
    check!(js::attempt(|| quiet.observe(&host, Some(&MutationObserverInit::create()))).is_err());
    check!(js::take_error().is_none());
    // disconnect: nothing more is recorded, and what was pending is dropped.
    host.append_child(doc.create_element(HTMLElementTagNameMap::P, None));
    quiet.disconnect();
    check!(quiet.take_records().len() == 0);
    host.append_child(doc.create_element(HTMLElementTagNameMap::P, None));
    check!(quiet.take_records().len() == 0);

    // ---- MutationObserver with a callback: one call for a batch, after the code that made it.
    let watched: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    watched.set_attribute("data-state", "a");
    body.append_child(watched.clone());
    due();
    let observer = MutationObserver::new(&MutationCallback::new(|records, _observer| {
        see(format!("mutation-batch:{}", records.len()));
        for r in records.iter() {
            let line = match r.type_() {
                MutationRecordType::ChildList => format!("mutation:childList:added={}:removed={}", r.added_nodes().length(), r.removed_nodes().length()),
                MutationRecordType::Attributes => format!("mutation:attributes:{}:old={:?}", r.attribute_name().unwrap(), r.old_value()),
                MutationRecordType::CharacterData => "mutation:characterData".to_string(),
            };
            see(line);
        }
        arrived();
    }));
    let everything = MutationObserverInit::create();
    everything.set_child_list(Some(true));
    everything.set_attributes(Some(true));
    everything.set_attribute_old_value(Some(true));
    observer.observe(&watched, Some(&everything));
    // A fragment adds both at once: one record.
    let fragment = doc.create_document_fragment();
    fragment.append_child(doc.create_element(HTMLElementTagNameMap::B, None));
    fragment.append_child(doc.create_element(HTMLElementTagNameMap::B, None));
    watched.append_child(fragment);
    watched.set_attribute("data-state", "b");
    watched.set_attribute("data-state", "c");
    watched.first_element_child().unwrap().remove();
    // Not yet: the callback runs after this function returns.
    check!(seen("mutation").is_empty());
    KEEP.with(|k| k.borrow_mut().push(observer.0.clone()));

    // ---- A disconnected observer's callback never runs.
    let dropped = MutationObserver::new(&MutationCallback::new(|_, _| see("after-disconnect".to_string())));
    dropped.observe(&watched, Some(&everything));
    watched.set_attribute("data-other", "1");
    dropped.disconnect();
    KEEP.with(|k| k.borrow_mut().push(dropped.0.clone()));

    // ---- ResizeObserver: fires on observe with the size, and again on change.
    let sized: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    sized.set_id("box");
    sized.style().set_css_text("width: 120px; height: 40px; border: 2px solid black");
    body.append_child(sized.clone());
    due();
    let resize = ResizeObserver::new(&ResizeObserverCallback::new({
        let sized = sized.clone();
        let mut calls = 0;
        move |entries, observer| {
            for entry in entries.iter() {
                let rect = entry.content_rect();
                let border = entry.border_box_size().get(0);
                see(format!(
                    "resize:target={}:content={}x{}:border={}x{}",
                    entry.target().id(),
                    rect.width(),
                    rect.height(),
                    border.inline_size(),
                    border.block_size()
                ));
                check!(entry.content_box_size().len() == 1);
                check!(entry.target() == sized.clone().into());
            }
            calls += 1;
            if calls == 1 {
                // Shrinking triggers the second call.
                sized.style().set_width("60px");
            } else {
                observer.unobserve(&sized);
                arrived();
            }
        }
    }));
    KEEP.with(|k| k.borrow_mut().push(resize.0.clone()));
    resize.observe(&sized, None);

    // ---- IntersectionObserver: one element on screen, one far below it.
    let near: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    near.set_id("near");
    near.style().set_css_text("position: fixed; left: 0; top: 0; width: 20px; height: 20px");
    let far: HTMLDivElement = doc.create_element(HTMLElementTagNameMap::DIV, None);
    far.set_id("far");
    far.style().set_css_text("position: absolute; left: 0; top: 50000px; width: 20px; height: 20px");
    body.append_child(near.clone());
    body.append_child(far.clone());
    PENDING.with(|p| p.set(p.get() + 2));
    let intersect = IntersectionObserver::new(
        &IntersectionObserverCallback::new(|entries, _| {
            for entry in entries.iter() {
                see(format!("intersect:{}:{}:ratio={}", entry.target().id(), entry.is_intersecting(), entry.intersection_ratio()));
                check!(entry.bounding_client_rect().width() == 20.0);
                check!(entry.root_bounds().is_some() && entry.time() > 0.0);
                arrived();
            }
        }),
        None,
    );
    check!(intersect.root().is_none() && intersect.root_margin() == "0px 0px 0px 0px");
    check!(intersect.thresholds().len() == 1 && intersect.thresholds().get(0) == 0.0);
    KEEP.with(|k| k.borrow_mut().push(intersect.0.clone()));
    intersect.observe(&near);
    intersect.observe(&far);
    // Options: a threshold list is a number or an array of them (FINDING 7: typed as the
    // VibratePattern alias, which is the TypeScript definition's own doing; awkward to read).
    let options = IntersectionObserverInit::create();
    options.set_threshold(JsArray::<f64>::from([0.0, 0.5, 1.0]));
    options.set_root_margin(Some("10px"));
    let with_options = IntersectionObserver::new(&IntersectionObserverCallback::new(|_, _| ()), Some(&options));
    check!(with_options.thresholds().len() == 3 && with_options.root_margin() == "10px 10px 10px 10px");
    with_options.observe(&near);
    check!(with_options.take_records().len() == 0);
    with_options.disconnect();

    // ---- AbortSignal.timeout aborts by itself, with a TimeoutError.
    let signal = AbortSignal::timeout(30.0);
    check!(!signal.aborted());
    due();
    signal.add_event_listener(
        "abort",
        EventListener::new({
            let signal = signal.clone();
            move |_: Event| {
                let reason = DOMException(signal.reason().0);
                see(format!("timeout:{}", reason.name()));
                check!(signal.aborted());
                arrived();
            }
        }),
        None,
    );
    // One that is canceled by its own controller is not a timeout.
    let controller = AbortController::new();
    let combined = AbortSignal::any([controller.signal(), AbortSignal::timeout(100000.0)]);
    check!(!combined.aborted());
    controller.abort(None);
    check!(combined.aborted());
    check!(DOMException(combined.reason().0).name() == "AbortError");
}
