// fetch and its objects from C: a page file fetched, Request, Response,
// Headers and Blob made directly, Response.json(), an aborted fetch.
// Promises settle later: busy() says so, finish() checks the rest. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)
#define STR(s, text) JsString_equals((s), (text))

#define CHAINS 12
static int pending;  // chains started and not yet over
static int done;     // chains that ended where they should

__attribute__((export_name("busy"))) int busy(void) { return pending; }

static bool starts_with(JsString s, const char* prefix) {
    char buf[96];
    JsString_copy(s, buf, sizeof buf);
    for (int i = 0; prefix[i]; i++) if (buf[i] != prefix[i]) return false;
    return true;
}
static bool ends_with(JsString s, const char* suffix) {
    char buf[256];
    uint32_t n = JsString_copy(s, buf, sizeof buf), m = ts_strlen(suffix);
    if (n >= sizeof buf || n < m) return false;
    for (uint32_t i = 0; i < m; i++) if (buf[n - m + i] != suffix[i]) return false;
    return true;
}
static void over(bool ok) { pending--; if (ok) done++; }

// 1. A page file: status, headers, text.
static void page_text(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    JsString text = js_to_string(value);
    CHECK(starts_with(text, "<!doctype html>\n<title>fetch</title>"));
    CHECK(JsString_len(text) > 100);
    over(ok);
}
// FINDING 9: Promise is untyped: Promise_then hands over a JsValue, and the Response (or text, Blob)
// it holds is recast by hand, as in `Response r = { value.h }`.
static void page_response(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    Response r = { value.h };
    CHECK(Response_get_ok(r) && Response_get_status(r) == 200);
    CHECK(Response_get_type(r) == ResponseType_basic);
    CHECK(ends_with(Response_get_url(r), "/index.html"));
    CHECK(Headers_has(Response_get_headers(r), "content-type"));
    CHECK(starts_with(Headers_get(Response_get_headers(r), "Content-Type"), "text/html"));
    CHECK(!Body_get_bodyUsed(r));
    Promise_then(Body_text(r), page_text, 0);
    CHECK(Body_get_bodyUsed(r));
}

// 2. Another served file keeps its type; a data: URL and a 404 status are fetched too.
static void missing_response(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    Response r = { value.h };
    CHECK(!Response_get_ok(r) && Response_get_status(r) == 404);
    over(ok);
}
static void wasm_response(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    Response r = { value.h };
    CHECK(Response_get_ok(r) && STR(Headers_get(Response_get_headers(r), "content-type"), "application/wasm"));
    over(ok);
}
static void data_text(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok && STR(js_to_string(value), "from a data url"));
    over(ok);
}
static void data_response(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    Response r = { value.h };
    CHECK(Response_get_status(r) == 200 && starts_with(Headers_get(Response_get_headers(r), "content-type"), "text/plain"));
    Promise_then(Body_text(r), data_text, 0);
}

// 3. A Response made here: read as text, clone read as text.
static void body_hello(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok && STR(js_to_string(value), "hello"));
    over(ok);
}
static void json_number(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok && js_to_number(value) == 42);
    over(ok);
}

// 4. Response.json() of an arbitrary plain object and array: fields come back as JSON.
static void json_object(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok);
    CHECK(js_object_has(value, "status") && js_object_has(value, "label") && !js_object_has(value, "missing"));
    CHECK(js_to_number(js_object_get(value, "status")) == 201);
    CHECK(STR(js_to_string(js_object_get(value, "label")), "made"));
    JsArray items = js_to_array(js_object_get(value, "items"));
    CHECK(JsArray_length(items) == 2 && js_to_number(JsArray_get(items, 1)) == 20);
    over(ok);
}

// 5. Blob: size, slices, text.
static void blob_text(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok && STR(js_to_string(value), "hello world"));
    over(ok);
}
static void slice_text(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(ok && STR(js_to_string(value), "world"));
    over(ok);
}

// 6. An aborted fetch rejects with an AbortError.
static void aborted(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    CHECK(!ok);
    CHECK(STR(js_error_name(value), "AbortError"));
    over(!ok);
}
// 7. A Response body read through Body_blob.
static void body_blob(void* ctx, bool ok, JsValue value) {
    (void)ctx;
    Blob b = { value.h };
    CHECK(ok && Blob_get_size(b) == 6 && STR(Blob_get_type(b), "text/plain;charset=utf-8"));
    over(ok);
}

__attribute__((export_name("run"))) void run(void) {
    // Fetch files the page is served with, plus a 404 response.
    pending += 4;
    Promise_then(fetch(RequestInfo_of(JsString_from("./index.html"))), page_response, 0);
    Promise_then(fetch(RequestInfo_of(JsString_from("./test.wasm"))), wasm_response, 0);
    Promise_then(fetch(RequestInfo_of(JsString_from("data:text/plain,from a data url"))), data_response, 0);
    Promise_then(fetch(RequestInfo_of(JsString_from("/status/404"))), missing_response, 0);

    // A Request built directly.
    RequestInit post = RequestInit_create();
    RequestInit_set_method(post, "POST");
    Headers sent = Headers_new((HeadersInit){ .tag = -1 });
    Headers_append(sent, "X-Token", "abc");
    RequestInit_set_headers(post, HeadersInit_of(sent));
    RequestInit_set_body(post, BodyInit_of(JsString_from("payload")));
    Request request = Request_new(RequestInfo_of(JsString_from("./index.html")), post);
    CHECK(STR(Request_get_method(request), "POST"));
    CHECK(ends_with(Request_get_url(request), "/index.html") && starts_with(Request_get_url(request), "http"));
    CHECK(STR(Headers_get(Request_get_headers(request), "x-token"), "abc"));
    CHECK(STR(Request_get_method(Request_new(RequestInfo_of(JsString_from("./index.html")))), "GET"));
    CHECK(STR(Request_get_method(Request_clone(request)), "POST"));
    CHECK(!AbortSignal_get_aborted(Request_get_signal(request)));
    // A URL that does not parse throws at once.
    CHECK(Request_new(RequestInfo_of(JsString_from("http://"))).h == 0 && js_failed());
    JsValue bad_url = js_take_error();
    CHECK(bad_url.h != 0 && STR(js_error_name(bad_url), "TypeError"));
    // HeadersInit_map builds a plain record of headers.
    HeadersInit_map hmap = HeadersInit_map_create();
    HeadersInit_map_set_at(hmap, "x-from-map", "yes");
    Headers from_map = Headers_new(HeadersInit_of(hmap));
    CHECK(STR(Headers_get(from_map, "x-from-map"), "yes"));

    // Headers: case-insensitive names, appended values joined, sorted iteration.
    Headers h = Headers_new((HeadersInit){ .tag = -1 });
    Headers_append(h, "Accept", "text/html");
    Headers_append(h, "accept", "application/json");
    Headers_set(h, "X-Count", "1");
    CHECK(STR(Headers_get(h, "ACCEPT"), "text/html, application/json"));
    CHECK(Headers_has(h, "x-count") && !Headers_has(h, "x-other"));
    Headers_set(h, "x-count", "2");
    CHECK(STR(Headers_get(h, "X-Count"), "2"));
    CHECK(Headers_get(h, "x-other").h == 0);
    Headers_delete(h, "x-count");
    CHECK(!Headers_has(h, "X-Count"));
    JsArray pair = JsArray_new();
    JsArray_push(pair, js_string("b-key"));
    JsArray_push(pair, js_string("b-value"));
    JsArray pairs = JsArray_new();
    JsArray_push(pairs, (JsValue){ pair.h });
    Headers from_pairs = Headers_new(HeadersInit_of(pairs));
    CHECK(STR(Headers_get(from_pairs, "b-key"), "b-value"));
    Headers_append(h, "zeta", "z");
    Headers_append(h, "alpha", "a");
    JsIterator keys = Headers_iterate(h);
    JsValue entry;
    int entries = 0;
    while (Headers_iterate_next(keys, &entry)) entries++;
    CHECK(entries == 3);

    // A Response made here: fields, then its body as text, a clone's body, JSON.
    ResponseInit init = ResponseInit_create();
    ResponseInit_set_status(init, js_some_double(201));
    ResponseInit_set_statusText(init, "Created");
    Headers reply_headers = Headers_new((HeadersInit){ .tag = -1 });
    Headers_set(reply_headers, "Content-Type", "text/plain");
    ResponseInit_set_headers(init, HeadersInit_of(reply_headers));
    Response reply = Response_new(BodyInit_of(JsString_from("hello")), init);
    CHECK(Response_get_status(reply) == 201 && Response_get_ok(reply));
    CHECK(STR(Response_get_statusText(reply), "Created"));
    CHECK(STR(Headers_get(Response_get_headers(reply), "content-type"), "text/plain"));
    CHECK(Response_get_type(reply) == ResponseType_default);
    Response copy = Response_clone(reply);
    pending += 2;
    Promise_then(Body_text(reply), body_hello, 0);
    Promise_then(Body_text(copy), body_hello, 0);
    CHECK(Body_get_bodyUsed(reply));
    Response error = Response_error();
    CHECK(Response_get_type(error) == ResponseType_error && Response_get_status(error) == 0 && !Response_get_ok(error));
    Response moved = Response_redirect("/elsewhere", js_some_double(301));
    CHECK(Response_get_status(moved) == 301 && ends_with(Headers_get(Response_get_headers(moved), "location"), "/elsewhere"));
    pending++;
    Promise_then(Body_json(Response_json(js_number(42))), json_number, 0);
    JsValue shape = js_object();
    js_object_set(shape, "status", js_number(201));
    js_object_set(shape, "label", js_string("made"));
    JsArray list = JsArray_new();
    JsArray_push(list, js_number(10));
    JsArray_push(list, js_number(20));
    js_object_set(shape, "items", js_value(list));
    Response as_json = Response_json(shape);
    CHECK(starts_with(Headers_get(Response_get_headers(as_json), "content-type"), "application/json"));
    pending++;
    Promise_then(Body_json(as_json), json_object, 0);

    // Blob: size, type, slices, text.
    JsArray_BlobPart parts = JsArray_BlobPart_new();
    JsArray_BlobPart_push(parts, BlobPart_of(JsString_from("hello ")));
    JsArray_BlobPart_push(parts, BlobPart_of(JsString_from("world")));
    BlobPropertyBag bag = BlobPropertyBag_create();
    BlobPropertyBag_set_type(bag, "text/plain");
    Blob blob = Blob_new(parts, bag);
    CHECK(Blob_get_size(blob) == 11 && STR(Blob_get_type(blob), "text/plain"));
    Blob tail = Blob_slice(blob, js_some_double(6), js_none_double(), 0);
    CHECK(Blob_get_size(tail) == 5);
    pending += 2;
    Promise_then(Blob_text(blob), blob_text, 0);
    Promise_then(Blob_text(tail), slice_text, 0);
    // A Response body read as a Blob keeps its size and type.
    pending++;
    Promise_then(Body_blob(Response_new(BodyInit_of(JsString_from("abcdef")))), body_blob, 0);

    // Abort a fetch before it settles.
    AbortController controller = AbortController_new();
    RequestInit with_signal = RequestInit_create();
    RequestInit_set_signal(with_signal, AbortController_get_signal(controller));
    pending++;
    Promise_then(fetch(RequestInfo_of(JsString_from("./index.html")), with_signal), aborted, 0);
    AbortController_abort(controller);
    CHECK(AbortSignal_get_aborted(AbortController_get_signal(controller)));
}

__attribute__((export_name("finish"))) void finish(void) {
    CHECK(pending == 0);
    CHECK(done == CHAINS);
}
