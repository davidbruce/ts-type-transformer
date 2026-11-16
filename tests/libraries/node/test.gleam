// Node's fs, path and events from Gleam: files written and read in a temporary
// directory, paths taken apart and joined, an EventEmitter's listeners.
import bindings/areas/events/event_emitter as events
import bindings/areas/fs as fs
import bindings/areas/fs/rm_options
import bindings/areas/fs/stats_base
import bindings/areas/main/buffer
import bindings/namespaces/node_js/event_emitter as emitter
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn join(a: String, b: String) -> String {
  a <> "/" <> b
}

fn str(s: String) {
  t.PathLikeString(s)
}

pub fn main() {
  // ---- path ----
  // FINDING 5: every function of the path module (join, basename, parse, ...) calls
  // PK["path"].path.X, and the module has no `path` property: TypeError. Left out;
  // the paths below are joined by hand.
  let passed = 0

  // ---- fs: a temporary directory under target/e2e ----
  let dir = fs.mkdtemp_sync("target/e2e/gleam-node-", None)
  let file = join(dir, "hello.txt")
  // FINDING 7: a read with an encoding (utf8) cannot be made: GlobalBufferEncoding
  // is an opaque type with no values. Read as a Buffer, then to_string.
  let read = fn(p) {
    buffer.to_string(fs.read_file_sync(t.PathOrFileDescriptorString(p), None), None, None, None)
  }
  let write = fn(p, text) {
    fs.write_file_sync(
      t.PathOrFileDescriptorString(p),
      t.StringOrNodeJSArrayBufferViewString(text),
      None,
    )
  }
  let passed =
    passed
    + check(string.starts_with(dir, "target/e2e/gleam-node-"), "mkdtemp")
    + check(fs.exists_sync(str(dir)), "the directory exists")
    + check(!fs.exists_sync(str(file)), "no file yet")
  write(file, "hello, node")
  let passed =
    passed
    + check(fs.exists_sync(str(file)), "writeFileSync")
    + check(read(file) == "hello, node", "readFileSync")
  fs.append_file_sync(
    t.PathOrFileDescriptorString(file),
    t.StringOrUint8ArrayString("!\nsecond line"),
    None,
  )
  let passed =
    passed
    + check(read(file) == "hello, node!\nsecond line", "appendFileSync")
    + check(list.length(string.split(read(file), "\n")) == 2, "lines")

  // More files, a subdirectory, a listing.
  let sub = join(dir, "sub")
  fs.mkdir_sync(str(sub), None)
  write(join(sub, "b.txt"), "bbb")
  write(join(dir, "a.json"), "{\"n\": 1}")
  let names = list.sort(fs.readdir_sync(str(dir), None), string.compare)
  let passed =
    passed
    + check(names == ["a.json", "hello.txt", "sub"], "readdirSync")
    + check(fs.readdir_sync(str(sub), None) == ["b.txt"], "readdirSync of a subdirectory")

  // Stats: kind and size.
  let st = fs.stat_sync(str(file))
  let sd = fs.stat_sync(str(sub))
  let passed =
    passed
    + check(stats_base.is_file(st) && !stats_base.is_directory(st), "a file's stats")
    + check(stats_base.is_directory(sd) && !stats_base.is_file(sd), "a directory's stats")
    + check(stats_base.get_size(st, runtime.codec_float()) == 24.0, "stat size")

  // Renamed and copied.
  fs.rename_sync(str(join(dir, "a.json")), str(join(dir, "b.json")))
  fs.copy_file_sync(str(file), str(join(dir, "copy.txt")), None)
  let passed =
    passed
    + check(!fs.exists_sync(str(join(dir, "a.json"))), "renamed away")
    + check(read(join(dir, "b.json")) == "{\"n\": 1}", "renamed to")
    + check(read(join(dir, "copy.txt")) == read(file), "copy_file_sync")
  fs.unlink_sync(str(join(dir, "copy.txt")))
  let passed = passed + check(!fs.exists_sync(str(join(dir, "copy.txt"))), "unlinkSync")

  // Missing files throw.
  let missing = runtime.attempt(fn() { read(join(dir, "nope.txt")) })
  let passed =
    passed
    + case missing {
      Ok(_) -> panic as "a missing file should throw"
      Error(e) -> {
        let message = runtime.js_error_message(e)
        check(string.contains(message, "ENOENT"), "ENOENT")
        + check(
          decode.run(runtime.dynamic(e), decode.at(["code"], decode.string)) == Ok("ENOENT"),
          "the error's code",
        )
      }
    }

  // Cleaned up, recursively.
  let recursive = rm_options.create()
  rm_options.set_recursive(recursive, Some(True))
  fs.rm_sync(str(dir), Some(recursive))
  let passed = passed + check(!fs.exists_sync(str(dir)), "rmSync recursive")

  // ---- events ----
  let bus = events.new(None)
  let name = runtime.dynamic("greet")
  let log = cell.new([])
  // FINDING 6: a listener `(...args: any[]) => void` is given Array.from(first
  // argument) by the glue, so a listener only counts calls: emit's own
  // arguments cannot be read.
  let on_greet = fn(_) { cell.set(log, ["greet", ..cell.get(log)]) }
  emitter.on(bus, name, on_greet)
  let count = cell.new(0)
  let on_once = fn(_) { cell.set(count, cell.get(count) + 1) }
  emitter.once(bus, name, on_once)
  let passed =
    passed
    + check(emitter.listener_count(bus, name, None) == 2.0, "two listeners")
    + check(emitter.emit(bus, name, []), "emit reports listeners")
    + check(cell.get(log) == ["greet"], "the listener was called")
    + check(cell.get(count) == 1, "once ran")
    + check(emitter.listener_count(bus, name, None) == 1.0, "once is gone after one emit")
  let _ = emitter.emit(bus, name, [])
  let passed =
    passed
    + check(cell.get(log) == ["greet", "greet"], "emit again")
    + check(cell.get(count) == 1, "once ran only once")
    + check(!emitter.emit(bus, runtime.dynamic("nobody"), []), "emit with no listener")
    + check(list.length(emitter.event_names(bus)) == 1, "event_names")
  let _ = emitter.remove_listener(bus, name, on_greet)
  let passed =
    passed
    + check(emitter.listener_count(bus, name, None) == 0.0, "removeListener")
    + check(!emitter.emit(bus, name, []), "no listeners left")
    + check(cell.get(log) == ["greet", "greet"], "the removed listener was not called")
  let _ = emitter.set_max_listeners(bus, 3.0)
  let passed = passed + check(emitter.get_max_listeners(bus) == 3.0, "max listeners")

  // events.once: a promise of the arguments of the next emit.
  let promise = events.once(bus, runtime.dynamic("ready"), None)
  let _ = emitter.emit(bus, runtime.dynamic("ready"), [runtime.dynamic("go"), runtime.dynamic("now")])
  runtime.promise_then(promise, fn(result) {
    let assert Ok(args) = result
    let words =
      list.map(args, fn(a) {
        let assert Ok(s) = decode.run(a, decode.string)
        s
      })
    let passed = passed + check(words == ["go", "now"], "events.once resolves with the arguments")
    io.println(int.to_string(passed) <> " checks passed")
  })
}
