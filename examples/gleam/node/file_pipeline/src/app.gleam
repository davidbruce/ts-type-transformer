import gleam/bool
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string
import node/areas/events/event_emitter as events
import node/areas/fs
import node/areas/fs/rm_options
import node/areas/fs/stats
import node/buffer
import node/mkdir_options
import node/areas/path
import node/namespaces/node_js/event_emitter as emitter
import node/runtime
import node/types as t

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn str(s: String) {
  t.PathLikeString(s)
}

fn read_text(p: String) -> String {
  buffer.to_string(
    fs.read_file_sync(t.PathOrFileDescriptorString(p), None),
    None,
    None,
    None,
  )
}

fn write_text(p: String, body: String) -> Nil {
  fs.write_file_sync(
    t.PathOrFileDescriptorString(p),
    t.StringOrNodeJSArrayBufferViewString(body),
    None,
  )
}

pub fn main() -> Nil {
  let bus = events.new(None)
  let stages = runtime.ref_new(0)
  let done_count = runtime.ref_new(0)

  let ev_stage = runtime.dynamic("stage")
  let ev_done = runtime.dynamic("done")
  emitter.on(bus, ev_stage, fn(_) {
    runtime.ref_update(stages, fn(n) { n + 1 })
    Nil
  })
  emitter.once(bus, ev_done, fn(_) {
    runtime.ref_update(done_count, fn(n) { n + 1 })
    Nil
  })

  let root = fs.mkdtemp_sync("/tmp/tsrs-node-pipe-", None)
  let in_dir = path.join([root, "in"])
  let out_dir = path.join([root, "out"])
  let mk = mkdir_options.create()
  fs.mkdir_sync_recursive(str(in_dir), mk)
  fs.mkdir_sync_recursive(str(out_dir), mk)

  // Stage 1: seed
  write_text(path.join([in_dir, "a.log"]), "100 OK\n")
  write_text(path.join([in_dir, "b.log"]), "200 OK\n")
  write_text(path.join([in_dir, "c.log"]), "300 OK\n")
  emitter.emit(bus, ev_stage, [runtime.dynamic("seed")])
  let passed = check(runtime.ref_get(stages) == 1, "seed stage")

  // Stage 2: bundle
  let logs = list.sort(fs.readdir_sync(str(in_dir), None), string.compare)
  let passed = passed + check(logs == ["a.log", "b.log", "c.log"], "input logs listed")
  let combined = path.join([out_dir, "combined.log"])
  let passed =
    list.fold(logs, passed, fn(acc, name) {
      let c = check(path.extname(name) == ".log", "extname is .log")
      let chunk = read_text(path.join([in_dir, name]))
      fs.append_file_sync(
        t.PathOrFileDescriptorString(combined),
        t.StringOrUint8ArrayString(chunk),
        None,
      )
      acc + c
    })
  emitter.emit(bus, ev_stage, [runtime.dynamic("bundle")])
  let passed = passed + check(runtime.ref_get(stages) == 2, "bundle stage")

  // Stage 3: verify
  let comb_text = read_text(combined)
  let st_comb = fs.stat_sync(str(combined))
  let passed =
    passed
    + check(
      stats.get_size(st_comb) == 21.0
      && string.contains(comb_text, "200 OK"),
      "combined output verified",
    )

  let enoent_caught = case runtime.attempt(fn() { read_text(path.join([out_dir, "missing.log"])) }) {
    Ok(_) -> False
    Error(e) -> string.contains(runtime.js_error_message(e), "ENOENT")
  }
  let passed = passed + check(enoent_caught, "missing file raised ENOENT")

  emitter.emit(bus, ev_stage, [runtime.dynamic("verify")])
  emitter.emit(bus, ev_done, [
    runtime.dynamic(int.to_float(string.length(comb_text))),
  ])
  let passed =
    passed
    + check(
      runtime.ref_get(stages) == 3 && runtime.ref_get(done_count) == 1,
      "pipeline completion events",
    )

  let rm =
    rm_options.create()
    |> rm_options.with_recursive(True)
    |> rm_options.with_force(True)
  fs.rm_sync(str(root), Some(rm))
  let passed = passed + check(!fs.exists_sync(str(root)), "pipeline workspace removed")

  io.println(
    "pipeline_root="
    <> root
    <> " combined_bytes="
    <> int.to_string(string.length(comb_text)),
  )
  io.println(
    "stages="
    <> int.to_string(runtime.ref_get(stages))
    <> " done="
    <> int.to_string(runtime.ref_get(done_count))
    <> " enoent="
    <> string.lowercase(bool.to_string(enoent_caught)),
  )
  io.println("PASS: file_pipeline (" <> int.to_string(passed) <> " checks)")
}
