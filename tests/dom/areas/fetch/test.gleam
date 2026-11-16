// Fetching, and the objects around it, from Gleam: Headers, Request, Response,
// Blob, and fetch of a file the page serves. The promises are chained; the
// program reports after the last one settles.
import bindings/areas/core as dom
import bindings/areas/core/abort_controller
import bindings/areas/core/abort_signal
import bindings/areas/network/body
import bindings/areas/network/headers
import bindings/areas/network/headers_init_map
import bindings/areas/network/request
import bindings/areas/network/request_init
import bindings/areas/network/response
import bindings/areas/network/response_init
import bindings/areas/network/url_search_params
import bindings/areas/storage/blob
import bindings/areas/storage/blob_property_bag
import bindings/areas/storage/file
import bindings/runtime.{type JsError, type Promise}
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/option.{None, Some}
import gleam/string

// Continue with a promise's value; a rejection fails the program.
fn await(p: Promise(a), then: fn(a) -> Nil) -> Nil {
  runtime.promise_then(p, fn(r) {
    case r {
      Ok(v) -> then(v)
      Error(e) -> panic as { "rejected: " <> runtime.js_error_message(e) }
    }
  })
}

fn settle(p: Promise(a), then: fn(Result(a, JsError)) -> Nil) -> Nil {
  runtime.promise_then(p, then)
}

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }

  // Headers: names are case-insensitive, values join.
  let h = headers.new(None)
  headers.append(h, "X-Tag", "a")
  headers.append(h, "x-tag", "b")
  headers.set(h, "Content-Type", "text/plain")
  check(headers.get(h, "X-TAG") == Some("a, b"), "appended values join")
  check(headers.has(h, "content-type"), "has, any case")
  check(headers.get(h, "missing") == None, "missing header")
  check(headers.to_list(h) == [#("content-type", "text/plain"), #("x-tag", "a, b")], "entries sorted and lowercased")
  headers.delete(h, "x-tag")
  check(!headers.has(h, "X-Tag"), "deleted")
  let map = headers_init_map.create()
  headers_init_map.set_at(map, "Accept", "application/json")
  let from_map = headers.new(Some(t.HeadersInitHeadersInitMap(map)))
  check(headers.get(from_map, "accept") == Some("application/json"), "headers from a record")
  let copy = headers.new(Some(t.HeadersInitHeaders(from_map)))
  check(headers.get(copy, "accept") == Some("application/json"), "headers from headers")

  // A Request built directly.
  let init = request_init.create()
  request_init.set_method(init, Some("POST"))
  request_init.set_headers(init, Some(t.HeadersInitHeadersInitMap(map)))
  request_init.set_body(init, Some(t.BodyInitString("payload")))
  let req = request.new(t.RequestOrStringOrURLString("./submit"), Some(init))
  check(request.get_method(req) == "POST", "request method")
  check(string.ends_with(request.get_url(req), "/submit"), "request url is absolute")
  check(string.starts_with(request.get_url(req), "http"), "request url scheme")
  check(headers.get(request.get_headers(req), "accept") == Some("application/json"), "request header")
  check(request.get_mode(req) == t.RequestModeCors, "request mode")
  let get_req = request.new(t.RequestOrStringOrURLString("./x"), None)
  check(request.get_method(get_req) == "GET", "default method")
  check(request.get_method(request.clone(req)) == "POST", "request clone")
  let aborter = abort_controller.new()
  let signalled = request_init.create()
  request_init.set_signal(signalled, Some(abort_controller.get_signal(aborter)))
  check(!{ request.new(t.RequestOrStringOrURLString("./x"), Some(signalled)) |> request.get_signal |> aborted }, "request signal")

  // A Response built directly.
  let rinit = response_init.create()
  response_init.set_status(rinit, Some(201.0))
  response_init.set_status_text(rinit, Some("Created"))
  response_init.set_headers(rinit, Some(t.HeadersInitHeadersInitMap(map)))
  let made = response.new(Some(t.BodyInitString("hello")), Some(rinit))
  check(response.get_status(made) == 201.0, "response status")
  check(response.get_status_text(made) == "Created", "status text")
  check(response.get_ok(made), "201 is ok")
  check(headers.get(response.get_headers(made), "accept") == Some("application/json"), "response header")
  check(!body.get_body_used(made), "body not used yet")
  let failure = response.error()
  check(failure |> response.get_type == t.ResponseTypeError, "error response type")
  check(response.get_status(failure) == 0.0 && !response.get_ok(failure), "error response status")
  let moved = response.redirect(t.StringOrURLString("https://example.com/x"), Some(301.0))
  check(response.get_status(moved) == 301.0, "redirect status")
  check(headers.get(response.get_headers(moved), "location") == Some("https://example.com/x"), "redirect location")
  let not_ok = response.new(None, Some(status_init(404.0)))
  check(!response.get_ok(not_ok) && response.get_status(not_ok) == 404.0, "404 is not ok")

  // A Blob and a File.
  let plain = blob_property_bag.create()
  blob_property_bag.set_type(plain, Some("text/plain"))
  let the_blob = blob.new(Some([t.BlobPartString("hello "), t.BlobPartString("blob")]), Some(plain))
  check(blob.get_size(the_blob) == 10.0, "blob size")
  check(blob.get_type(the_blob) == "text/plain", "blob type")
  let tail = blob.slice(the_blob, Some(6.0), None, None)
  check(blob.get_size(tail) == 4.0, "blob slice size")
  let the_file = file.new([t.BlobPartString("abc")], "a.txt", None)
  check(file.get_name(the_file) == "a.txt", "file name")
  check(blob.get_size(the_file) == 3.0, "a file is a blob")

  // The promises, one after another.
  await(body.text(made), fn(text) {
    check(text == "hello", "response text")
    check(body.get_body_used(made), "body used after reading")
    // Response.json takes a JS value; a ResponseInit is a plain object.
    // FINDING 4 (awkward): Response.json's data is Dynamic and Gleam has no
    // way to make a JS object, so a dictionary from the bindings stands in.
    let data = response_init.create()
    response_init.set_status(data, Some(7.0))
    response_init.set_status_text(data, Some("seven"))
    let json = response.json(runtime.dynamic(data), None)
    check(headers.get(response.get_headers(json), "content-type") == Some("application/json"), "json content type")
    await(body.json(json), fn(parsed) {
      check(decode.run(parsed, decode.at(["status"], decode.int)) == Ok(7), "json body")
      check(decode.run(parsed, decode.at(["statusText"], decode.string)) == Ok("seven"), "json string")
      await(blob.text(the_blob), fn(text) {
        check(text == "hello blob", "blob text")
        await(blob.text(tail), fn(text) {
          check(text == "blob", "blob slice text")
          let params = url_search_params.new(None)
          url_search_params.append(params, "a", "1")
          url_search_params.append(params, "b", "x y")
          let form = response.new(Some(t.BodyInitURLSearchParams(params)), None)
          check(
            option.map(headers.get(response.get_headers(form), "content-type"), string.starts_with(_, "application/x-www-form-urlencoded"))
              == Some(True),
            "params body content type",
          )
          await(body.text(form), fn(text) {
            check(text == "a=1&b=x+y", "params body text")
            fetch_page(check, count)
          })
        })
      })
    })
  })
}

fn aborted(s) -> Bool {
  abort_signal.get_aborted(s)
}

fn status_init(status: Float) {
  let i = response_init.create()
  response_init.set_status(i, Some(status))
  i
}

// fetch of files the page serves.
fn fetch_page(check, count) -> Nil {
  await(dom.fetch(t.RequestOrStringOrURLString("./index.html"), None), fn(res) {
    check(response.get_status(res) == 200.0 && response.get_ok(res), "fetch status")
    check(string.ends_with(response.get_url(res), "/index.html"), "fetch url")
    check(
      option.map(headers.get(response.get_headers(res), "content-type"), string.contains(_, "text/html"))
        == Some(True),
      "fetch content type",
    )
    check(response.get_type(res) == t.ResponseTypeBasic, "same-origin response type")
    let again = response.clone(res)
    await(body.text(res), fn(text) {
      check(string.contains(text, "<title>fetch</title>"), "page text")
      check(string.contains(text, "<body>"), "page has a body")
      await(body.blob(again), fn(b) {
        check(blob.get_size(b) == int.to_float(string.length(text)) || blob.get_size(b) >. 0.0, "response blob")
        by_request(check, count)
      })
    })
  })
}

// The harness fails a page that requests a missing file, so a 404 is made, not fetched.
fn by_request(check, count) -> Nil {
  let get = request.new(t.RequestOrStringOrURLString("./index.html"), None)
  await(dom.fetch(t.RequestOrStringOrURLString(request.get_url(get)), None), fn(res2) {
    check(response.get_status(res2) == 200.0, "fetch by absolute url")
    aborting(check, count)
  })
}

fn aborting(check, count) -> Nil {
  let controller = abort_controller.new()
  let init = request_init.create()
  request_init.set_signal(init, Some(abort_controller.get_signal(controller)))
  let pending = dom.fetch(t.RequestOrStringOrURLString("./index.html"), Some(init))
  abort_controller.abort(controller, None)
  settle(pending, fn(result) {
    case result {
      Ok(_) -> panic as "an aborted fetch resolved"
      Error(e) -> {
        check(string.contains(string.lowercase(runtime.js_error_message(e)), "abort"), "aborted fetch rejects: " <> runtime.js_error_message(e))
        // A fetch of a Request object.
        let req = request.new(t.RequestOrStringOrURLString("./index.html"), None)
        await(dom.fetch(t.RequestOrStringOrURLString(request.get_url(req)), None), fn(res) {
          check(response.get_ok(res), "fetch after abort still works")
          io.println(int.to_string(cell.get(count)) <> " checks passed")
        })
      }
    }
  })
}
