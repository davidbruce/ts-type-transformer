// The network objects from Rust: a real fetch, and Request, Response,
// Headers and Blob made here. Awaits run in spawned tasks; `busy` says how
// many are pending and `finish` counts those that completed. Every check!
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
    static COMPLETED: Cell<i32> = const { Cell::new(0) };
}

// A task that is counted while it runs and once it has completed.
fn task(f: impl Future<Output = ()> + 'static) {
    PENDING.with(|p| p.set(p.get() + 1));
    js::spawn(async move {
        f.await;
        PENDING.with(|p| p.set(p.get() - 1));
        COMPLETED.with(|c| c.set(c.get() + 1));
    });
}

#[unsafe(no_mangle)]
pub extern "C" fn busy() -> i32 {
    PENDING.with(Cell::get)
}

#[unsafe(no_mangle)]
pub extern "C" fn finish() {
    check!(busy() == 0);
    check!(COMPLETED.with(Cell::get) == 5);
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Headers: case-insensitive names, append joins values.
    let headers = Headers::new();
    headers.set("Content-Type", "text/plain");
    headers.append("X-Tag", "a");
    headers.append("x-tag", "b");
    check!(headers.get("content-type").as_deref() == Some("text/plain"));
    check!(headers.get("X-TAG").as_deref() == Some("a, b"));
    check!(headers.has("x-tag") && !headers.has("x-other") && headers.get("x-other").is_none());
    headers.delete("x-tag");
    check!(!headers.has("x-tag"));
    headers.set("accept", "*/*");
    let pairs: Vec<(String, String)> = headers.iter().map(|p| (p.get0(), p.get1())).collect();
    check!(pairs == [("accept".to_string(), "*/*".to_string()), ("content-type".to_string(), "text/plain".to_string())]);
    check!(headers.keys().iter().collect::<Vec<_>>() == ["accept", "content-type"]);
    let from_map = Headers::new_with([("x-a", "1")]);
    check!(from_map.get("x-a").as_deref() == Some("1"));
    check!(Headers::new_with(&headers).get("accept").as_deref() == Some("*/*"));

    // Request: defaults, then an init.
    let plain = Request::new("https://example.com/path?q=1");
    check!(plain.url() == "https://example.com/path?q=1" && plain.method() == "GET");
    check!(plain.mode() == RequestMode::Cors && plain.cache() == RequestCache::Default);
    let init = RequestInit::create();
    init.set_method(Some("POST"));
    init.set_body("payload");
    init.set_headers(&headers);
    init.set_cache(Some(RequestCache::NoStore));
    check!(init.method().as_deref() == Some("POST"));
    let post = Request::new_with("/submit", &init);
    check!(post.method() == "POST" && post.url().ends_with("/submit") && post.url().starts_with("http"));
    check!(post.cache() == RequestCache::NoStore);
    check!(post.headers().get("content-type").as_deref() == Some("text/plain;charset=UTF-8") || post.headers().get("content-type").as_deref() == Some("text/plain"));
    check!(!post.signal().aborted());
    let copy = post.js_clone();
    check!(copy.url() == post.url() && copy != post);
    // A request with a body that is not allowed with GET throws.
    let bad = RequestInit::create();
    bad.set_body("x");
    check!(js::attempt(|| Request::new_with("/x", &bad)).is_err());
    check!(js::take_error().is_none());

    // Response: status, headers, body.
    let rinit = ResponseInit::create();
    rinit.set_status(Some(201.0));
    rinit.set_status_text(Some("Made"));
    rinit.set_headers([("x-a", "1")]);
    let made = Response::new_with("hello body", &rinit);
    check!(made.status() == 201.0 && made.ok() && made.status_text() == "Made");
    check!(made.headers().get("x-a").as_deref() == Some("1"));
    check!(made.headers().get("content-type").as_deref() == Some("text/plain;charset=UTF-8"));
    check!(!made.body_used() && made.url().is_empty() && !made.redirected());
    check!(made.type_() == ResponseType::Default);
    check!(!Response::error().ok() && Response::error().type_() == ResponseType::Error && Response::error().status() == 0.0);
    let moved = Response::redirect_with("https://example.com/", 301.0);
    check!(moved.status() == 301.0 && moved.headers().get("location").as_deref() == Some("https://example.com/"));
    check!(js::attempt(|| Response::new_with(None, &{ let i = ResponseInit::create(); i.set_status(Some(99.0)); i })).is_err());
    check!(js::take_error().is_none());

    // The body is read once.
    task(async move {
        let copy = made.js_clone();
        check!(copy.text().await.as_deref() == Ok("hello body"));
        check!(made.body_used() == false && copy.body_used());
        check!(made.text().await.as_deref() == Ok("hello body"));
        check!(made.body_used());
        // A second read of a used body rejects.
        check!(made.text().await.is_err());
    });

    // Response.json: an arbitrary plain object and array, as a body and parsed back.
    task(async {
        let object = JsValue::object();
        object.set("n", 7.0);
        object.set("label", "seven");
        object.set("flag", true);
        let json = Response::json(&object);
        check!(json.headers().get("content-type").as_deref() == Some("application/json"));
        let copy = json.js_clone();
        check!(copy.text().await.as_deref() == Ok("{\"n\":7,\"label\":\"seven\",\"flag\":true}"));
        let parsed = json.json().await.unwrap();
        check!(parsed.has("n") && parsed.has("label") && !parsed.has("missing"));
        check!(parsed.get("n").as_f64() == 7.0 && parsed.get("label").as_string() == "seven" && parsed.get("flag").as_bool());
        let numbers = JsArray::<f64>::from([1.0, 2.5, 3.0]);
        let list = Response::json(&numbers);
        let back: JsArray<f64> = list.json().await.unwrap().as_array();
        check!(back.len() == 3 && back.get(0) == 1.0 && back.get(1) == 2.5 && back.get(2) == 3.0);
        check!(Response::json(&numbers).text().await.as_deref() == Ok("[1,2.5,3]"));
    });

    // Blob: size, type, slices and text.
    let parts = JsArray::<BlobPart>::from(["hello ", "blob ", "world"]);
    let options = BlobPropertyBag::create();
    options.set_type(Some("text/plain"));
    let blob = Blob::new_with(&parts, &options);
    check!(blob.size() == 16.0 && blob.type_() == "text/plain");
    let piece = blob.slice_with(6.0, 10.0, "text/x-piece");
    check!(piece.size() == 4.0 && piece.type_() == "text/x-piece");
    check!(Blob::new().size() == 0.0 && Blob::new().type_().is_empty());
    task(async move {
        check!(blob.text().await.as_deref() == Ok("hello blob world"));
        check!(piece.text().await.as_deref() == Ok("blob"));
        let bytes = blob.bytes().await.unwrap();
        let mut first = [0u8; 5];
        bytes.read_bytes(0, &mut first);
        check!(bytes.len() == 16 && &first == b"hello");
        // A blob as a response body keeps its type.
        let response = Response::new_with(&blob, None);
        check!(response.headers().get("content-type").as_deref() == Some("text/plain"));
        check!(response.blob().await.unwrap().size() == 16.0);
    });

    // A real fetch of a file the page serves.
    task(async {
        let Ok(response) = fetch("./index.html").await else {
            check!(false);
            return;
        };
        check!(response.ok() && response.status() == 200.0 && response.status_text() == "OK");
        check!(response.url().ends_with("/index.html") && !response.redirected());
        check!(response.type_() == ResponseType::Basic);
        let content_type = response.headers().get("content-type").unwrap_or_default();
        check!(content_type.starts_with("text/html"));
        let text = response.text().await.unwrap_or_default();
        check!(text.contains("<title>fetch</title>") && text.contains("<script"));
        check!(response.body_used());
        // A Request object fetches the same, with its own headers.
        let init = RequestInit::create();
        init.set_method(Some("GET"));
        init.set_cache(Some(RequestCache::NoStore));
        let request = Request::new_with("./index.html", &init);
        let again = fetch(&request).await.unwrap();
        check!(again.status() == 200.0 && again.text().await.unwrap_or_default() == text);
        // A 404 resolves with ok == false, not a rejection.
        let missing = fetch("/status/404").await.unwrap();
        check!(!missing.ok() && missing.status() == 404.0);
        check!(missing.text().await.unwrap_or_default() == "status 404");
    });

    // AbortController cancels a request: the promise rejects with an AbortError.
    task(async {
        let controller = AbortController::new();
        let init = RequestInit::create();
        init.set_signal(Some(&controller.signal()));
        let pending = fetch_with("./index.html", &init);
        controller.abort();
        match pending.await {
            Ok(_) => check!(false),
            Err(e) => check!(e.name() == "AbortError"),
        }
        check!(controller.signal().aborted());
        // A signal already aborted rejects at once.
        let init = RequestInit::create();
        init.set_signal(Some(&AbortSignal::abort()));
        check!(fetch_with("./index.html", &init).await.is_err());
    });
}
