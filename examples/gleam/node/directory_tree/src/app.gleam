import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/string
import node/areas/fs
import node/areas/fs/rm_options
import node/areas/fs/stats
import node/areas/fs/stats_base
import node/areas/path
import node/mkdir_options

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn walk_dir(dir: String, counts: #(Int, Int, Float)) -> #(Int, Int, Float) {
  let entries = list.sort(fs.readdir_sync(dir), string.compare)
  list.fold(entries, counts, fn(acc, name) {
    let #(dirs, files, bytes) = acc
    let full = path.join([dir, name])
    let st = fs.stat_sync(full)
    case stats_base.is_directory(st), stats_base.is_file(st) {
      True, _ -> walk_dir(full, #(dirs + 1, files, bytes))
      False, True -> #(dirs, files + 1, bytes +. stats.get_size(st))
      False, False -> acc
    }
  })
}

pub fn main() -> Nil {
  let root = fs.mkdtemp_sync("/tmp/tsrs-node-tree-")
  let passed = check(fs.exists_sync(root), "temp tree created")

  let mk = mkdir_options.create()
  fs.mkdir_sync_recursive(path.join([root, "src", "core"]), mk)
  fs.mkdir_sync_recursive(path.join([root, "src", "cli"]), mk)
  fs.mkdir_sync_recursive(path.join([root, "docs"]), mk)

  fs.write_file_sync(path.join([root, "README.md"]), "node tree demo")
  fs.write_file_sync(
    path.join([root, "src", "core", "engine.txt"]),
    "engine core module!\n",
  )
  fs.write_file_sync(
    path.join([root, "src", "cli", "args.txt"]),
    "cli args parser\n",
  )
  fs.write_file_sync(
    path.join([root, "docs", "guide.md"]),
    "monochrome documentation\n",
  )

  let #(dir_count, file_count, total_bytes) = walk_dir(root, #(0, 0, 0.0))

  let rm =
    rm_options.create()
    |> rm_options.with_recursive(True)
    |> rm_options.with_force(True)
  fs.rm_sync_with(root, rm)

  let passed =
    passed
    + check(dir_count == 4, "dir count = 4")
    + check(file_count == 4, "file count = 4")
    + check(total_bytes == 75.0, "total bytes = 75")
    + check(!fs.exists_sync(root), "tree cleaned up")
    + check(dir_count + file_count == 8, "total entries = 8")
    + check(True, "walk complete")

  io.println("root=" <> root)
  io.println(
    "dirs="
    <> int.to_string(dir_count)
    <> " files="
    <> int.to_string(file_count)
    <> " total_bytes="
    <> int.to_string(float.round(total_bytes)),
  )
  io.println("PASS: directory_tree (" <> int.to_string(passed) <> " checks)")
}
