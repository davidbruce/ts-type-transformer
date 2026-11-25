// Node's fs, path and events from Gleam: files written and read in a temporary
// directory, paths taken apart and joined, an EventEmitter's listeners.
import bindings/areas/events/event_emitter as events
import bindings/areas/fs
import bindings/areas/fs/rm_options
import bindings/areas/fs/stats_base
import bindings/areas/path
import bindings/areas/path/format_input_path_object as path_format
import bindings/areas/path/parsed_path
import bindings/buffer
import bindings/mkdir_options
import bindings/namespaces/node_js/event_emitter as emitter
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic/decode
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() {
  // ---- path ----
  let parsed = path.parse("/home/user/dir/file.txt")
  let to_format = path_format.create()
  path_format.set_dir(to_format, Some("/tmp"))
  path_format.set_name(to_format, Some("note"))
  path_format.set_ext(to_format, Some(".md"))
  let passed =
    check(path.join(["a", "b", "../c", "d.txt"]) == "a/c/d.txt", "path.join")
    + check(path.normalize("/a//b/./c/..") == "/a/b", "path.normalize")
    + check(path.resolve(["/a", "b", "../c"]) == "/a/c", "path.resolve")
    + check(
      path.is_absolute("/x") && !path.is_absolute("x/y"),
      "path.isAbsolute",
    )
    + check(path.relative("/a/b/c", "/a/d") == "../../d", "path.relative")
    + check(path.dirname("/a/b/c.txt") == "/a/b", "path.dirname")
    + check(path.basename("/a/b/c.txt") == "c.txt", "path.basename")
    + check(
      path.basename_with("/a/b/c.txt", Some(".txt")) == "c",
      "path.basename with a suffix",
    )
    + check(path.extname("archive.tar.gz") == ".gz", "path.extname")
    + check(path.matches_glob("/a/b/c.txt", "/a/**/*.txt"), "path.matchesGlob")
    + check(
      parsed_path.get_root(parsed) == "/"
        && parsed_path.get_dir(parsed) == "/home/user/dir",
      "path.parse root and dir",
    )
    + check(
      parsed_path.get_base(parsed) == "file.txt"
        && parsed_path.get_name(parsed) == "file"
        && parsed_path.get_ext(parsed) == ".txt",
      "path.parse base, name and ext",
    )
    + check(path.format(to_format) == "/tmp/note.md", "path.format")

  // ---- fs: a temporary directory under target/e2e ----
  let dir = fs.mkdtemp_sync("target/e2e/gleam-node-")
  let file = path.join([dir, "hello.txt"])
  let read = fn(p) {
    fs.read_file_sync_with_read_file_sync_options_buffer_encoding(
      p,
      t.BufferEncodingUtf8,
    )
  }
  let read_buf = fn(p) { buffer.to_string(fs.read_file_sync(p)) }
  let write = fn(p, text) { fs.write_file_sync(p, text) }
  let passed =
    passed
    + check(string.starts_with(dir, "target/e2e/gleam-node-"), "mkdtemp")
    + check(fs.exists_sync(dir), "the directory exists")
    + check(!fs.exists_sync(file), "no file yet")
  write(file, "hello, node")
  let passed =
    passed
    + check(fs.exists_sync(file), "writeFileSync")
    + check(read(file) == "hello, node", "readFileSync")
    + check(read_buf(file) == "hello, node", "readFileSync as Buffer")
  fs.append_file_sync(file, "!\nsecond line")
  let passed =
    passed
    + check(read(file) == "hello, node!\nsecond line", "appendFileSync")
    + check(list.length(string.split(read(file), "\n")) == 2, "lines")

  // More files, a subdirectory, a listing.
  let sub = path.join([dir, "sub"])
  fs.mkdir_sync(sub)
  write(path.join([sub, "b.txt"]), "bbb")
  write(path.join([dir, "a.json"]), "{\"n\": 1}")
  let names = list.sort(fs.readdir_sync(dir), string.compare)
  let passed =
    passed
    + check(names == ["a.json", "hello.txt", "sub"], "readdirSync")
    + check(fs.readdir_sync(sub) == ["b.txt"], "readdirSync of a subdirectory")

  // Stats: kind and size.
  let st = fs.stat_sync(file)
  let sd = fs.stat_sync(sub)
  let passed =
    passed
    + check(
      stats_base.is_file(st) && !stats_base.is_directory(st),
      "a file's stats",
    )
    + check(
      stats_base.is_directory(sd) && !stats_base.is_file(sd),
      "a directory's stats",
    )
    + check(stats_base.get_size(st, runtime.codec_float()) == 24.0, "stat size")

  // Renamed and copied.
  fs.rename_sync(path.join([dir, "a.json"]), path.join([dir, "b.json"]))
  fs.copy_file_sync(file, path.join([dir, "copy.txt"]))
  let passed =
    passed
    + check(!fs.exists_sync(path.join([dir, "a.json"])), "renamed away")
    + check(read(path.join([dir, "b.json"])) == "{\"n\": 1}", "renamed to")
    + check(read(path.join([dir, "copy.txt"])) == read(file), "copy_file_sync")
  fs.unlink_sync(path.join([dir, "copy.txt"]))
  let passed =
    passed + check(!fs.exists_sync(path.join([dir, "copy.txt"])), "unlinkSync")

  // A nested directory, made recursively; mkdir options say so.
  let deep = path.join([dir, "x", "y", "z"])
  let opts = mkdir_options.create()
  let first = fs.mkdir_sync_recursive(deep, opts)
  let passed =
    passed
    + check(
      decode.run(runtime.dynamic(opts), decode.at(["recursive"], decode.bool))
        == Ok(True),
      "mkdir options are recursive",
    )
    + check(fs.exists_sync(deep), "a nested directory")
    + check(
      first == Some(path.join([dir, "x"])),
      "mkdir reports the first directory made",
    )

  // Missing files throw.
  let missing = runtime.attempt(fn() { read(path.join([dir, "nope.txt"])) })
  let passed =
    passed
    + case missing {
      Ok(_) -> panic as "a missing file should throw"
      Error(e) -> {
        let message = runtime.js_error_message(e)
        check(string.contains(message, "ENOENT"), "ENOENT")
        + check(
          decode.run(runtime.dynamic(e), decode.at(["code"], decode.string))
            == Ok("ENOENT"),
          "the error's code",
        )
      }
    }

  // Cleaned up, recursively.
  let recursive = rm_options.create()
  rm_options.set_recursive(recursive, Some(True))
  fs.rm_sync_with(dir, Some(recursive))
  let passed = passed + check(!fs.exists_sync(dir), "rmSync recursive")

  // ---- events ----
  let bus = events.new()
  let name = runtime.dynamic("greet")
  let log = cell.new([])
  let on_greet = fn(args) {
    let words =
      list.map(args, fn(a) {
        let assert Ok(s) = decode.run(a, decode.string)
        s
      })
    cell.set(log, [string.join(words, "+"), ..cell.get(log)])
  }
  emitter.on(bus, name, on_greet)
  let count = cell.new(0)
  let on_once = fn(_) { cell.set(count, cell.get(count) + 1) }
  emitter.once(bus, name, on_once)
  let passed =
    passed
    + check(emitter.listener_count(bus, name) == 2.0, "two listeners")
    + check(
      emitter.emit(bus, name, [runtime.dynamic("hi"), runtime.dynamic("there")]),
      "emit reports listeners",
    )
    + check(cell.get(log) == ["hi+there"], "the listener got every argument")
    + check(cell.get(count) == 1, "once ran")
    + check(
      emitter.listener_count(bus, name) == 1.0,
      "once is gone after one emit",
    )
  let _ = emitter.emit(bus, name, [runtime.dynamic("again")])
  let passed =
    passed
    + check(cell.get(log) == ["again", "hi+there"], "emit again")
    + check(cell.get(count) == 1, "once ran only once")
    + check(
      !emitter.emit(bus, runtime.dynamic("nobody"), []),
      "emit with no listener",
    )
    + check(list.length(emitter.event_names(bus)) == 1, "event_names")
  let _ = emitter.remove_listener(bus, name, on_greet)
  let passed =
    passed
    + check(emitter.listener_count(bus, name) == 0.0, "removeListener")
    + check(!emitter.emit(bus, name, []), "no listeners left")
    + check(
      cell.get(log) == ["again", "hi+there"],
      "the removed listener was not called",
    )
  let _ = emitter.set_max_listeners(bus, 3.0)
  let passed =
    passed + check(emitter.get_max_listeners(bus) == 3.0, "max listeners")

  // events.once: a promise of the arguments of the next emit.
  let promise = events.once(bus, runtime.dynamic("ready"))
  let _ =
    emitter.emit(bus, runtime.dynamic("ready"), [
      runtime.dynamic("go"),
      runtime.dynamic("now"),
    ])
  runtime.promise_then(promise, fn(result) {
    let assert Ok(args) = result
    let words =
      list.map(args, fn(a) {
        let assert Ok(s) = decode.run(a, decode.string)
        s
      })
    let passed =
      passed
      + check(words == ["go", "now"], "events.once resolves with the arguments")
    io.println(int.to_string(passed) <> " checks passed")
  })
}
