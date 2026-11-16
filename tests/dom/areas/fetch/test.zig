// Fetch from Zig: the page's own file fetched, Headers, Request, Response and
// Blob made here, and a fetch aborted. Promises settle after `run` returns:
// `busy` is nonzero while any is pending, and `finish` checks what arrived.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// How many promises have not settled; `busy` reports it.
var pending: u32 = 0;
export fn busy() u32 {
    return pending;
}

// What arrived, for `finish`.
var page_status: f64 = 0;
var page_ok = false;
var page_type_html = false;
var page_url_ends = false;
var page_has_title = false;
var page_len: usize = 0;
var hello_text = false;
var blob_text = false;
var json_number: f64 = 0;
var json_text = false;
var abort_rejected = false;
var abort_name_ok = false;
var bad_rejected = false;
var request_fetched = false;
var clone_texts = false;
var clone_first = false;
var formdata_text = false;

fn launch(p: b.Promise, comptime f: fn (?*anyopaque, bool, b.JsValue) void) void {
    pending += 1;
    p.then(f, null);
}

fn done() void {
    pending -= 1;
}

// FINDING 3: Promise has no type parameter (areas/types.zig:2017), so a settled value is a bare
// JsValue and a Promise<Response> is cast by hand: kind awkward.
// A promise's value is untyped: the one a fetch settles with is a Response.
fn asResponse(v: b.JsValue) b.Response {
    return .{ .h = v.h };
}

fn onPage(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    if (!ok) return;
    const res = asResponse(v);
    page_status = res.getStatus();
    page_ok = res.getOk();
    var buf: [64]u8 = undefined;
    page_type_html = if (res.getHeaders().get("content-type")) |t| std.mem.indexOf(u8, t.copy(&buf), "text/html") != null else false;
    const url = res.getUrl().copy(&buf);
    page_url_ends = std.mem.endsWith(u8, url, "/index.html");
    // The body is read once; the handle is kept for the promise that reads it.
    launch(res.text(), onPageText);
}

fn onPageText(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    if (!ok) return;
    const s = v.toString();
    page_len = s.len();
    var buf: [256]u8 = undefined;
    page_has_title = std.mem.indexOf(u8, s.copy(&buf), "<title>fetch</title>") != null;
}

fn onHello(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    hello_text = ok and v.toString().eql("hello");
}

fn onBlobText(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    blob_text = ok and v.toString().eql("hello blob");
}

fn onJsonNumber(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    if (ok) json_number = v.toNumber();
}

fn onJsonText(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    json_text = ok and v.toString().eql("quoted \"text\"");
}

fn onAbort(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    abort_rejected = !ok;
    abort_name_ok = !ok and b.jsErrorName(v).eql("AbortError");
}

fn onBad(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    bad_rejected = !ok and b.jsErrorName(v).eql("TypeError");
}

fn onRequest(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    request_fetched = ok and asResponse(v).getStatus() == 200;
}

fn onCloneFirst(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    clone_first = ok and v.toString().eql("twice");
}

fn onCloneSecond(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    clone_texts = ok and v.toString().eql("twice");
}

fn onFormText(_: ?*anyopaque, ok: bool, v: b.JsValue) void {
    defer done();
    var buf: [256]u8 = undefined;
    formdata_text = ok and std.mem.indexOf(u8, v.toString().copy(&buf), "name=\"k\"") != null;
}

fn bodyOf(text: []const u8) b.BodyInit {
    return .{ .string = b.JsString.from(text) };
}

export fn run() void {
    // Headers: names are case-insensitive, values combine on append.
    const headers = b.Headers.new(null);
    headers.append("Accept", "text/html");
    headers.append("accept", "application/json");
    headers.set("X-Token", "abc");
    expect(headers.has("ACCEPT") and headers.has("x-token"), @src());
    expect(headers.get("accept").?.eql("text/html, application/json"), @src());
    expect(headers.get("x-token").?.eql("abc"), @src());
    expect(headers.get("missing") == null, @src());
    headers.delete("x-token");
    expect(!headers.has("x-token"), @src());
    headers.set("a-b", "2");
    var names: u32 = 0;
    const it = headers.iterate();
    while (it.next()) |pair| {
        names += 1;
        // Headers iterate in sorted order, name first.
        if (names == 1) expect(pair.get(0).eql("a-b") and pair.get(1).eql("2"), @src());
    }
    expect(names == 2, @src());

    // Headers made from a plain object.
    const map = b.HeadersInit_map.create();
    map.setAt("x-one", "1");
    map.setAt("x-two", "2");
    const from_map = b.Headers.new(.{ .HeadersInit_map = map });
    expect(from_map.get("x-one").?.eql("1") and from_map.get("x-two").?.eql("2"), @src());

    // A Request carries what it was given.
    const request_init = b.RequestInit.create();
    request_init.setMethod("POST");
    request_init.setHeaders(.{ .Headers = headers });
    request_init.setBody(bodyOf("payload"));
    const request = b.Request.new(.{ .string = b.JsString.from("https://example.com/api?x=1") }, .{ .init = request_init });
    expect(request.getMethod().eql("POST"), @src());
    expect(request.getUrl().eql("https://example.com/api?x=1"), @src());
    expect(request.getHeaders().get("accept").?.eql("text/html, application/json"), @src());
    expect(!request.getBodyUsed(), @src());
    expect(request.getMode() == .cors, @src());
    expect(request.clone().getMethod().eql("POST"), @src());
    const get_request = b.Request.new(.{ .string = b.JsString.from("https://example.com/") }, .{});
    expect(get_request.getMethod().eql("GET"), @src());

    // A Response made here: status, headers, ok.
    const response_init = b.ResponseInit.create();
    response_init.setStatus(201);
    response_init.setStatusText("Made");
    response_init.setHeaders(.{ .HeadersInit_map = map });
    const made = b.Response.new(bodyOf("created"), .{ .init = response_init });
    expect(made.getStatus() == 201 and made.getOk(), @src());
    expect(made.getStatusText().eql("Made"), @src());
    expect(made.getHeaders().get("x-one").?.eql("1"), @src());
    expect(made.getType() == .default, @src());
    expect(!made.getBodyUsed(), @src());
    const failed = b.ResponseInit.create();
    failed.setStatus(500);
    expect(!b.Response.new(null, .{ .init = failed }).getOk(), @src());
    expect(b.Response.@"error"().getType() == .@"error", @src());
    expect(b.Response.redirect(.{ .string = b.JsString.from("https://example.com/next") }, .{ .status = 302 }).getStatus() == 302, @src());

    // FINDING 4: Body's instance `json()` is generated as `json2` (areas/network.zig:518) because
    // the static Response.json took the name: kind awkward. And a JSON object read back through
    // json2() is a bare JsValue with no way to read its properties, so only a number and a string
    // are round-tripped here: kind can't express (reading fields of an untyped value).
    // Response.json writes the body as JSON and says so.
    const json = b.Response.json(b.JsValue.number(7), .{});
    expect(json.getHeaders().get("content-type").?.eql("application/json"), @src());
    launch(json.json2(), onJsonNumber);
    launch(b.Response.json(b.JsValue.string("quoted \"text\""), .{}).json2(), onJsonText);

    // Bodies read as text.
    launch(b.Response.new(bodyOf("hello"), .{}).text(), onHello);
    const twice = b.Response.new(bodyOf("twice"), .{});
    const copy = twice.clone();
    launch(twice.text(), onCloneFirst);
    launch(copy.text(), onCloneSecond);

    // A Blob: size, type, slices, and text.
    const blob_parts = b.JsArrayOf(b.BlobPart).new();
    blob_parts.push(.{ .string = b.JsString.from("hello ") });
    blob_parts.push(.{ .string = b.JsString.from("blob") });
    const bag = b.BlobPropertyBag.create();
    bag.setType("text/plain");
    const blob = b.Blob.new(.{ .blobParts = blob_parts, .options = bag });
    expect(blob.getSize() == 10 and blob.getType().eql("text/plain"), @src());
    const head = blob.slice(.{ .start = 0, .end = 5 });
    expect(head.getSize() == 5, @src());
    expect(b.Blob.new(.{}).getSize() == 0, @src());
    launch(blob.text(), onBlobText);
    // A FormData as a body is multipart, and a Response reads it back as text.
    const form = b.FormData.new(null, null);
    form.appendWithString("k", "v");
    launch(b.Response.new(.{ .FormData = form }, .{}).text(), onFormText);

    // The page's own file, over the network.
    launch(b.fetch(.{ .string = b.JsString.from("./index.html") }, .{}), onPage);
    // (A missing file is left out: the test server fails the whole run on a 404.)
    // A Request object fetched.
    launch(b.fetch(.{ .Request = b.Request.new(.{ .string = b.JsString.from("./index.html") }, .{}) }, .{}), onRequest);
    // A fetch aborted before it can finish rejects with an AbortError.
    const controller = b.AbortController.new();
    const abort_init = b.RequestInit.create();
    abort_init.setSignal(controller.getSignal());
    launch(b.fetch(.{ .string = b.JsString.from("./index.html") }, .{ .init = abort_init }), onAbort);
    controller.abort(null);
    expect(controller.getSignal().getAborted(), @src());
    // A URL that cannot be fetched rejects with a TypeError.
    launch(b.fetch(.{ .string = b.JsString.from("http://") }, .{}), onBad);
}

export fn finish() void {
    expect(page_status == 200 and page_ok, @src());
    expect(page_type_html, @src());
    expect(page_url_ends, @src());
    expect(page_len > 0 and page_has_title, @src());
    expect(hello_text, @src());
    expect(blob_text, @src());
    expect(json_number == 7 and json_text, @src());
    expect(abort_rejected and abort_name_ok, @src());
    expect(bad_rejected, @src());
    expect(request_fetched, @src());
    expect(clone_first and clone_texts, @src());
    expect(formdata_text, @src());
    expect(pending == 0, @src());
}
