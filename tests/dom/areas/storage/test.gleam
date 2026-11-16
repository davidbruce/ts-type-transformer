// Web storage from Gleam: localStorage and sessionStorage, which are separate,
// keep strings, count their keys, and hold JSON text that round-trips.
import bindings/areas/core as dom
import bindings/areas/network/body
import bindings/areas/network/response
import bindings/areas/storage/storage
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic
import gleam/dynamic/decode
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
  let local = dom.get_local_storage()
  let session = dom.get_session_storage()
  storage.clear(local)
  storage.clear(session)
  check(storage.get_length(local) == 0.0, "local starts empty")
  check(storage.get_length(session) == 0.0, "session starts empty")

  // Set, get, overwrite, remove.
  storage.set_item(local, "name", "ada")
  check(storage.get_item(local, "name") == Some("ada"), "getItem")
  check(storage.get_length(local) == 1.0, "length 1")
  storage.set_item(local, "name", "grace")
  check(storage.get_item(local, "name") == Some("grace"), "overwritten")
  check(storage.get_length(local) == 1.0, "overwrite keeps the length")
  check(storage.get_item(local, "missing") == None, "missing key is None")
  storage.remove_item(local, "name")
  check(storage.get_item(local, "name") == None, "removed")
  check(storage.get_length(local) == 0.0, "length 0 again")
  storage.remove_item(local, "never-set")
  check(storage.get_length(local) == 0.0, "removing a missing key is harmless")

  // Values are strings: numbers come back as text.
  storage.set_item(local, "n", int.to_string(42))
  check(storage.get_item(local, "n") == Some("42"), "number as text")
  storage.set_item(local, "empty", "")
  check(storage.get_item(local, "empty") == Some(""), "empty string is a value")
  storage.set_item(local, "unicode", "caf\u{e9} \u{1f600}")
  check(storage.get_item(local, "unicode") == Some("caf\u{e9} \u{1f600}"), "unicode")
  storage.set_item(local, "multi\nline", "a\nb")
  check(storage.get_item(local, "multi\nline") == Some("a\nb"), "odd key and value")

  // The two stores are apart.
  storage.set_item(session, "only-session", "s")
  check(storage.get_item(local, "only-session") == None, "session value not in local")
  storage.set_item(local, "only-local", "l")
  check(storage.get_item(session, "only-local") == None, "local value not in session")
  check(storage.get_length(session) == 1.0, "session length")
  check(storage.get_length(local) == 5.0, "local length")

  // key(i): every key appears exactly once, none past the end.
  let keys =
    list.index_map(list.repeat(0, 5), fn(_, i) { storage.key(local, int.to_float(i)) })
  check(list.all(keys, option.is_some), "five keys")
  let names = list.filter_map(keys, option.to_result(_, Nil))
  check(list.sort(names, string.compare) == ["empty", "multi\nline", "n", "only-local", "unicode"], "key names")
  check(storage.key(local, 5.0) == None, "no key past the end")
  check(storage.key(local, 99.0) == None, "no key far past the end")

  // Index access by name sees the same data.
  check(decode.run(storage.get_at(local, "n"), decode.string) == Ok("42"), "get by name")
  check(decode.run(storage.get_at(local, "nothing"), decode.string) |> is_error, "get by missing name")
  storage.set_at(local, "viaindex", runtime.dynamic("v"))
  check(storage.get_item(local, "viaindex") == Some("v"), "set by name")
  check(storage.get_length(local) == 6.0, "length after set by name")
  storage.set_at(local, "viaindex", dynamic.int(7))
  check(storage.get_item(local, "viaindex") == Some("7"), "a number set by name is text")

  // Clear.
  storage.clear(local)
  check(storage.get_length(local) == 0.0 && storage.get_item(local, "n") == None, "clear")
  check(storage.get_item(session, "only-session") == Some("s"), "clearing local leaves session")
  storage.remove_item(session, "only-session")
  check(storage.get_length(session) == 0.0, "session emptied")

  // The quota is finite: a huge value throws.
  let huge = string.repeat("x", 12_000_000)
  check(runtime.attempt(fn() { storage.set_item(local, "huge", huge) }) |> is_error, "quota exceeded throws")
  check(storage.get_item(local, "huge") == None, "nothing stored")
  let assert Error(e) = runtime.attempt(fn() { storage.set_item(local, "huge", huge) })
  check(string.contains(runtime.js_error_message(e), "quota"), "the error says quota: " <> runtime.js_error_message(e))

  // JSON text stored and read back; parsed with a Response's json().
  let json = "{\"name\":\"ada\",\"langs\":[\"gleam\",\"rust\"],\"n\":3}"
  storage.set_item(local, "doc", json)
  let assert Some(stored) = storage.get_item(local, "doc")
  check(stored == json, "JSON text is kept exactly")
  let parsed = response.new(Some(t.BodyInitString(stored)), None)
  runtime.promise_then(body.json(parsed), fn(result) {
    let assert Ok(value) = result
    check(decode.run(value, decode.at(["name"], decode.string)) == Ok("ada"), "parsed name")
    check(decode.run(value, decode.at(["n"], decode.int)) == Ok(3), "parsed number")
    check(decode.run(value, decode.at(["langs"], decode.list(decode.string))) == Ok(["gleam", "rust"]), "parsed list")
    io.println(int.to_string(cell.get(count)) <> " checks passed")
  })
}

fn is_error(r: Result(a, b)) -> Bool {
  case r {
    Ok(_) -> False
    Error(_) -> True
  }
}
