// URL and URLSearchParams from Gleam: parsing, building, resolving, and the
// live link between a URL and its search parameters.
import bindings/areas/core/string_or_url
import bindings/areas/network/url
import bindings/areas/network/url_search_params as params
import bindings/areas/network/url_search_params_iterator as params_iter
import bindings/runtime
import cell
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

pub fn main() {
  let count = cell.new(0)
  let check = fn(ok: Bool, what: String) {
    case ok {
      True -> cell.set(count, cell.get(count) + 1)
      False -> panic as what
    }
  }

  // Parsing the parts.
  let u = url.new("https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag")
  check(url.get_protocol(u) == "https:", "protocol")
  check(url.get_host(u) == "example.com:8080", "host")
  check(url.get_hostname(u) == "example.com", "hostname")
  check(url.get_port(u) == "8080", "port")
  check(url.get_pathname(u) == "/a/b/c.html", "pathname")
  check(url.get_search(u) == "?x=1&y=two", "search")
  check(url.get_hash(u) == "#frag", "hash")
  check(url.get_username(u) == "user" && url.get_password(u) == "pw", "credentials")
  check(url.get_origin(u) == "https://example.com:8080", "origin")
  check(url.get_href(u) == "https://user:pw@example.com:8080/a/b/c.html?x=1&y=two#frag", "href")
  check(url.to_string(u) == url.get_href(u) && url.to_json(u) == url.get_href(u), "toString and toJSON")

  // Normalisation: default port dropped, host lowercased, dots resolved.
  let n = url.new("HTTP://EXAMPLE.com:80/a/./b/../c")
  check(url.get_href(n) == "http://example.com/a/c", "normalised")
  check(url.get_port(n) == "", "default port is empty")

  // Building by setters.
  let b = url.new("http://example.com/")
  url.set_protocol(b, "https")
  url.set_hostname(b, "api.example.org")
  url.set_port(b, "9000")
  url.set_pathname(b, "/v1/items list")
  url.set_search(b, "q=a b")
  url.set_hash(b, "top")
  check(url.get_href(b) == "https://api.example.org:9000/v1/items%20list?q=a%20b#top", "built href")
  url.set_href(b, "http://other.example/x")
  check(url.get_host(b) == "other.example" && url.get_pathname(b) == "/x", "setHref")
  url.set_host(b, "h.example:81")
  check(url.get_port(b) == "81", "setHost")
  url.set_username(b, "me")
  check(url.get_href(b) == "http://me@h.example:81/x", "setUsername")

  // Relative resolution against a base.
  let base = "https://example.com/dir/sub/page.html?old=1#h"
  check(url.get_href(url.new_with("other.html", base)) == "https://example.com/dir/sub/other.html", "sibling")
  check(url.get_href(url.new_with("../up.html", base)) == "https://example.com/dir/up.html", "parent")
  check(url.get_href(url.new_with("/root", base)) == "https://example.com/root", "absolute path")
  check(url.get_href(url.new_with("?new=2", base)) == "https://example.com/dir/sub/page.html?new=2", "query only")
  check(url.get_href(url.new_with("#other", base)) == "https://example.com/dir/sub/page.html?old=1#other", "fragment only")
  check(url.get_href(url.new_with("//cdn.example/lib.js", base)) == "https://cdn.example/lib.js", "scheme relative")
  check(url.get_href(url.new_with("http://abs.example/", base)) == "http://abs.example/", "absolute wins")
  check(url.get_href(url.new_of_with(string_or_url.Url(u), string_or_url.String("http://ignored/"))) == url.get_href(u), "from a URL")

  // canParse and parse.
  check(url.can_parse("https://example.com"), "canParse")
  check(!url.can_parse("not a url"), "cannot parse")
  check(url.can_parse_with("/relative", "https://example.com"), "canParse with a base")
  check(!url.can_parse("/relative"), "relative needs a base")
  check(url.parse("nope") == None, "parse gives None")
  check(option.map(url.parse("ftp://f.example/x"), url.get_protocol) == Some("ftp:"), "parse gives Some")
  check(runtime.attempt(fn() { url.new("not a url") }) |> is_error, "constructor throws")
  let assert Error(e) = runtime.attempt(fn() { url.new("::bad") })
  check(string.contains(runtime.js_error_message(e), "URL"), "the error names the URL: " <> runtime.js_error_message(e))

  // Search parameters.
  let p = params.new_with("a=1&b=2&a=3&c=x%20y")
  check(params.get(p, "a") == Some("1"), "get returns the first")
  check(params.get_all(p, "a") == ["1", "3"], "getAll")
  check(params.get(p, "c") == Some("x y"), "decoded")
  check(params.get(p, "zzz") == None, "missing")
  check(params.has(p, "b") && !params.has(p, "zzz"), "has")
  check(params.has_with(p, "a", "3") && !params.has_with(p, "a", "9"), "has with a value")
  check(params.get_size(p) == 4.0, "size")
  params.append(p, "d", "4&5")
  check(params.to_string(p) == "a=1&b=2&a=3&c=x+y&d=4%265", "append encodes")
  params.set(p, "a", "9")
  check(params.get_all(p, "a") == ["9"], "set replaces all")
  check(params.to_string(p) == "a=9&b=2&c=x+y&d=4%265", "set keeps the first place")
  params.delete(p, "b")
  check(!params.has(p, "b"), "delete")
  params.sort(p)
  check(list.map(params.to_list(p), fn(kv) { kv.0 }) == ["a", "c", "d"], "sort")
  let q = params.new_with("z=1&a=2&m=3&a=1")
  params.sort(q)
  check(params.to_string(q) == "a=2&a=1&m=3&z=1", "sort is stable")

  // Iteration.
  check(params.to_list(q) == [#("a", "2"), #("a", "1"), #("m", "3"), #("z", "1")], "entries as a list")
  check(
    params_iter.to_list(params.entries(q), runtime.codec_tuple2(runtime.codec_string(), runtime.codec_string()))
      == [#("a", "2"), #("a", "1"), #("m", "3"), #("z", "1")],
    "entries iterator",
  )
  check(params_iter.to_list(params.keys(q), runtime.codec_string()) == ["a", "a", "m", "z"], "keys")
  check(params_iter.to_list(params.values(q), runtime.codec_string()) == ["2", "1", "3", "1"], "values")
  let seen = cell.new([])
  params.for_each(q, fn(value, name, _) { cell.set(seen, [name <> "=" <> value, ..cell.get(seen)]) })
  check(list.reverse(cell.get(seen)) == ["a=2", "a=1", "m=3", "z=1"], "forEach")

  // Other initialisers.
  let pairs = params.new_array_with([["k", "v"], ["k", "w"]])
  check(params.to_string(pairs) == "k=v&k=w", "from pairs")
  let copy = params.new_url_search_params_with(pairs)
  params.append(copy, "n", "1")
  check(params.get_size(pairs) == 2.0 && params.get_size(copy) == 3.0, "a copy is independent")
  check(params.to_string(params.new()) == "", "empty")

  // A URL's searchParams are live both ways.
  let live = url.new("https://example.com/p?x=1")
  let sp = url.get_search_params(live)
  check(params.get(sp, "x") == Some("1"), "searchParams reads the query")
  params.append(sp, "y", "a b")
  check(url.get_search(live) == "?x=1&y=a+b", "searchParams writes the URL")
  url.set_search(live, "?only=1")
  check(params.to_string(sp) == "only=1", "the URL writes searchParams")
  params.delete(sp, "only")
  check(url.get_search(live) == "", "an empty query is dropped")
  check(url.get_href(live) == "https://example.com/p", "href without a query")

  io.println(int.to_string(cell.get(count)) <> " checks passed")
}

fn is_error(r: Result(a, b)) -> Bool {
  case r {
    Ok(_) -> False
    Error(_) -> True
  }
}
