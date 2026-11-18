// Fetch from C++: a real request for the page itself, Request, Response and
// Headers made directly, JSON, Blob and an aborted fetch. The promises settle
// after `run`; `busy` says so and `finish` counts what completed.
// Every CHECK must hold.
#include <string>
#include <utility>

#include "areas/core/AbortController.hpp"
#include "areas/core/Document.hpp"
#include "areas/core/functions.hpp"
#include "areas/network/Headers.hpp"
#include "areas/network/Request.hpp"
#include "areas/network/RequestInit.hpp"
#include "areas/network/Response.hpp"
#include "areas/network/ResponseInit.hpp"
#include "areas/storage/Blob.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// FINDING 5: as in the CSS program, a value from JavaScript of a type the API
// does not give (here, the object json() resolves with) needs a cast the
// bindings lack; the copied handle's reference goes to the type wanted.
template <class D, class B>
static D downcast(const B& b) {
    B copy = b;
    return D(ts::handle, std::exchange(copy.h, 0));
}

// FINDING 6: Response declares the static json(data), which hides Body's
// json() for a Response; the base class is named to read the body.
static ts::Promise<ts::Value> body_json(const Response& r) { return static_cast<const Body&>(r).json(); }

static int pending = 0;
static int completed = 0;

// A request for the page, and what comes back.
static ts::Task fetch_page() {
    pending++;
    ts::Result<Response> r = co_await fetch("./index.html");
    CHECK(bool(r));
    if (r) {
        CHECK(r->ok() && r->status() == 200 && r->statusText() == "OK");
        CHECK(r->type() == ResponseType::basic);
        CHECK(r->url().ends_with("/index.html") && !r->redirected());
        CHECK(r->headers().get("content-type").value_or("").find("text/html") != std::string::npos);
        CHECK(r->headers().has("content-type") && !r->headers().has("x-nothing"));
        CHECK(!r->bodyUsed());
        ts::Result<std::string> text = co_await r->text();
        CHECK(bool(text));
        CHECK(text && text->find("<title>fetch</title>") != std::string::npos);
        CHECK(r->bodyUsed());
        // The body can be read once.
        ts::Result<std::string> again = co_await r->text();
        CHECK(!again);
    }
    completed++;
    pending--;
}

// Another file of the page: its script, as text and as bytes, plus a 404 response.
static ts::Task fetch_script() {
    pending++;
    ts::Result<Response> r = co_await fetch("./glue.mjs");
    CHECK(bool(r));
    if (r) {
        CHECK(r->status() == 200);
        CHECK(r->headers().get("content-type").value_or("").find("javascript") != std::string::npos);
        Response twin = r->clone();
        ts::Result<std::string> text = co_await r->text();
        CHECK(text && text->find("makeEnv") != std::string::npos);
        ts::Result<Uint8Array<ArrayBuffer>> bytes = co_await static_cast<const Body&>(twin).bytes();
        CHECK(bytes && text && bytes->size() == text->size());
    }
    ts::Result<Response> missing = co_await fetch("/status/404");
    CHECK(bool(missing));
    if (missing) {
        CHECK(!missing->ok() && missing->status() == 404);
        ts::Result<std::string> body = co_await missing->text();
        CHECK(body && *body == "status 404");
    }
    completed++;
    pending--;
}

// A request with a method, headers and a body, read back from the Request.
static ts::Task request_made() {
    pending++;
    Headers headers = Headers::create();
    headers.set("X-Token", "abc");
    RequestInit init = RequestInit::create();
    init.method("POST");
    init.headers(headers);
    init.body(std::string("payload"));
    Request req = Request::create(std::string("./echo"), init);
    CHECK(req.method() == "POST" && req.url().ends_with("/echo"));
    CHECK(req.headers().get("x-token") == "abc");
    Request copy = req.clone();
    CHECK(copy.method() == "POST" && copy.headers().get("X-Token") == "abc");
    ts::Result<std::string> body = co_await req.text();
    CHECK(body && *body == "payload");
    ts::Result<std::string> copied = co_await copy.text();
    CHECK(copied && *copied == "payload");
    completed++;
    pending--;
}

// A Response made here: status, headers, text.
static ts::Task response_made() {
    pending++;
    ResponseInit init = ResponseInit::create();
    init.status(201);
    init.statusText("Made");
    Headers h = Headers::create();
    h.set("Content-Type", "text/plain");
    h.append("X-Tag", "one");
    h.append("X-Tag", "two");
    init.headers(h);
    Response res = Response::create(std::string("made by cpp"), init);
    CHECK(res.status() == 201 && res.ok() && res.statusText() == "Made");
    CHECK(res.headers().get("content-type") == "text/plain");
    CHECK(res.headers().get("x-tag") == "one, two");
    Response twin = res.clone();
    ts::Result<std::string> text = co_await res.text();
    CHECK(text && *text == "made by cpp");
    ts::Result<Blob> blob = co_await twin.blob();
    CHECK(blob && blob->size() == 11);
    CHECK(Response::error().type() == ResponseType::error);
    CHECK(Response::redirect(std::string("http://example.com/")).status() == 302);
    completed++;
    pending--;
}

// Response.json(): an object goes in, and the same comes out.
static ts::Task json_round_trip() {
    pending++;
    // A dictionary is a plain object: {status: 7, statusText: "seven"}.
    ResponseInit data = ResponseInit::create();
    data.status(7);
    data.statusText("seven");
    Response res = Response::json(ts::Value(ts::handle, [&] {
        ResponseInit copy = data;
        return std::exchange(copy.h, 0);
    }()));
    CHECK(res.headers().get("content-type") == "application/json");
    ts::Result<ts::Value> parsed = co_await body_json(res);
    CHECK(bool(parsed));
    if (parsed) {
        ResponseInit back = downcast<ResponseInit>(*parsed);
        CHECK(back.status() == 7 && back.statusText() == "seven");
    }
    // A string is JSON too.
    Response s = Response::json(ts::Value::string("hi"));
    ts::Result<std::string> raw = co_await s.text();
    CHECK(raw && *raw == "\"hi\"");
    Response n = Response::json(ts::Value::number(12.5));
    ts::Result<ts::Value> num = co_await body_json(n);
    CHECK(num && num->to_number() == 12.5);
    // Not JSON: json() rejects.
    Response bad = Response::create(std::string("not json"));
    ts::Result<ts::Value> failure = co_await body_json(bad);
    CHECK(!failure && failure.error().name() == "SyntaxError");
    completed++;
    pending--;
}

// A Blob: size, type, text, slices.
static ts::Task blob_made() {
    pending++;
    BlobPropertyBag bag = BlobPropertyBag::create();
    bag.type("text/plain");
    Blob blob = Blob::create(ts::Array<BlobPart>{BlobPart(std::string("hello ")), BlobPart(std::string("blob"))}, bag);
    CHECK(blob.size() == 10 && blob.type() == "text/plain");
    ts::Result<std::string> text = co_await blob.text();
    CHECK(text && *text == "hello blob");
    Blob tail = blob.slice(6, 10);
    CHECK(tail.size() == 4);
    ts::Result<std::string> tailText = co_await tail.text();
    CHECK(tailText && *tailText == "blob");
    CHECK(Blob::create().size() == 0);
    ts::Result<Uint8Array<ArrayBuffer>> bytes = co_await blob.bytes();
    CHECK(bytes && bytes->size() == 10 && bytes->to_vector()[0] == 'h');
    completed++;
    pending--;
}

// An aborted fetch rejects with an AbortError.
static AbortController controller;
static ts::Task fetch_aborted() {
    pending++;
    controller = AbortController::create();
    RequestInit init = RequestInit::create();
    init.signal(controller.signal());
    ts::Promise<Response> p = fetch("./index.html", init);
    controller.abort();
    CHECK(controller.signal().aborted());
    ts::Result<Response> r = co_await p;
    CHECK(!r);
    CHECK(!r && r.error().name() == "AbortError");
    completed++;
    pending--;
}

// A signal already aborted never starts the request.
static ts::Task fetch_preaborted() {
    pending++;
    AbortController c = AbortController::create();
    c.abort();
    RequestInit init = RequestInit::create();
    init.signal(c.signal());
    ts::Result<Response> r = co_await fetch("./index.html", init);
    CHECK(!r && r.error().name() == "AbortError");
    completed++;
    pending--;
}

extern "C" __attribute__((export_name("busy"))) int busy() { return pending; }

extern "C" __attribute__((export_name("finish"))) void finish() {
    CHECK(completed == 8);
    CHECK(pending == 0);
}

extern "C" __attribute__((export_name("run"))) void run() {
    // Headers: case-insensitive names, appended values, iteration, deletion.
    Headers h = Headers::create();
    h.set("Accept", "text/html");
    h.append("Accept", "application/json");
    h.set("X-One", "1");
    CHECK(h.get("accept") == "text/html, application/json");
    CHECK(h.has("ACCEPT") && !h.get("missing"));
    h.delete_("X-One");
    CHECK(!h.has("x-one"));
    int count = 0;
    std::string names;
    for (ts::Tuple<std::string, std::string> kv : h) {
        count++;
        names += kv.get<0>();
    }
    CHECK(count == 1 && names == "accept");

    fetch_page();
    fetch_script();
    request_made();
    response_made();
    json_round_trip();
    blob_made();
    fetch_aborted();
    fetch_preaborted();
    CHECK(pending > 0);
}
