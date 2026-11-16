// localStorage and sessionStorage from Rust: items set, read, removed and
// counted, and a JSON document stored as text and read back. Every check!
// must hold.
#![no_main]
use bindings::*;
use std::cell::Cell;

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
    static ROUND_TRIPPED: Cell<bool> = const { Cell::new(false) };
}

#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    PENDING.with(Cell::get)
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(ROUND_TRIPPED.with(Cell::get));
    // What the async part stored is still there.
    check!(local_storage().get_item("config").is_some());
    local_storage().clear();
    check!(local_storage().length() == 0.0);
}

fn keys(s: &Storage) -> Vec<String> {
    let mut keys: Vec<String> = (0..s.length() as u32).filter_map(|i| s.key(i as f64)).collect();
    keys.sort();
    keys
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    let local = local_storage();
    let session = session_storage();
    local.clear();
    session.clear();
    check!(local.length() == 0.0 && session.length() == 0.0);
    check!(local.get_item("missing").is_none() && local.key(0.0).is_none());

    // set and get.
    local.set_item("name", "ada");
    local.set_item("count", "42");
    check!(local.length() == 2.0);
    check!(local.get_item("name").as_deref() == Some("ada"));
    check!(local.get_item("count").as_deref() == Some("42"));
    check!(local.get_item("Name").is_none());
    // Setting again replaces and does not add.
    local.set_item("name", "grace");
    check!(local.get_item("name").as_deref() == Some("grace") && local.length() == 2.0);
    // An empty value is a value, not a missing one.
    local.set_item("empty", "");
    check!(local.get_item("empty").as_deref() == Some("") && local.length() == 3.0);
    // Keys can be anything, including the empty string; values are unicode.
    local.set_item("", "blank key");
    local.set_item("ключ 🔑", "значение ✓");
    check!(local.get_item("").as_deref() == Some("blank key"));
    check!(local.get_item("ключ 🔑").as_deref() == Some("значение ✓"));
    check!(local.length() == 5.0);

    // key(i) and length enumerate every key once.
    check!(keys(&local) == ["", "count", "empty", "name", "ключ 🔑"]);
    check!(local.key(5.0).is_none() && local.key(99.0).is_none());

    // The indexer form reads the same items; a missing key is undefined.
    check!(local.at("name").as_string() == "grace");
    local.set_at("viaindex", "1");
    check!(local.get_item("viaindex").as_deref() == Some("1"));
    local.remove_item("viaindex");

    // remove.
    local.remove_item("count");
    check!(local.get_item("count").is_none() && local.length() == 4.0);
    // Removing what is not there does nothing.
    local.remove_item("count");
    check!(local.length() == 4.0);

    // Session and local storage are separate areas.
    session.set_item("name", "session value");
    check!(session.get_item("name").as_deref() == Some("session value"));
    check!(local.get_item("name").as_deref() == Some("grace"));
    check!(session.length() == 1.0 && local.length() == 4.0);
    session.set_item("other", "x");
    check!(local.get_item("other").is_none());
    check!(keys(&session) == ["name", "other"]);
    session.clear();
    check!(session.length() == 0.0 && local.length() == 4.0);

    // A large value, and a number is kept as the text it was stored as.
    let big = "x".repeat(100_000);
    local.set_item("big", &big);
    check!(local.get_item("big").map(|v| v.len()) == Some(100_000));
    local.set_item("pi", &3.14.to_string());
    check!(local.get_item("pi").unwrap().parse::<f64>() == Ok(3.14));

    // clear empties it.
    local.clear();
    check!(local.length() == 0.0 && local.get_item("name").is_none() && local.key(0.0).is_none());

    // A StorageEvent can be made and read; the area it names is what it was given.
    let init = StorageEventInit::create();
    init.set_key(Some("k"));
    init.set_old_value(Some("1"));
    init.set_new_value(Some("2"));
    init.set_storage_area(Some(&local));
    let event = StorageEvent::new("storage", Some(&init));
    check!(event.key().as_deref() == Some("k") && event.old_value().as_deref() == Some("1") && event.new_value().as_deref() == Some("2"));
    check!(event.storage_area() == Some(local.clone()));
    check!(event.type_() == "storage" && StorageEvent::new("storage", None).key().is_none());

    // JSON: a document made as JSON text, stored, read back and parsed.
    PENDING.with(|p| p.set(p.get() + 1));
    js::spawn(async {
        let doc = ResponseInit::create();
        doc.set_status(Some(201.0));
        doc.set_status_text(Some("stored \"quoted\" text"));
        let text = Response::json(&doc, None).text().await.unwrap();
        check!(text == "{\"status\":201,\"statusText\":\"stored \\\"quoted\\\" text\"}");
        let local = local_storage();
        local.set_item("config", &text);
        check!(local.get_item("config").as_deref() == Some(text.as_str()));
        // Read back, then parsed.
        let stored = local.get_item("config").unwrap();
        let parsed = ResponseInit(Response::new(stored.as_str(), None).json().await.unwrap().0);
        check!(parsed.status() == Some(201.0));
        check!(parsed.status_text().as_deref() == Some("stored \"quoted\" text"));
        // Text that is not JSON does not parse.
        local.set_item("broken", "{not json");
        check!(Response::new(local.get_item("broken").unwrap().as_str(), None).json().await.is_err());
        local.remove_item("broken");
        ROUND_TRIPPED.with(|r| r.set(true));
        PENDING.with(|p| p.set(p.get() - 1));
    });
}
