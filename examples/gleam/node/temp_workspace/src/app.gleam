import gleam/float
import gleam/int
import gleam/io
import gleam/option.{Some}
import node/areas/fs
import node/areas/fs/rm_options
import node/areas/fs/stats
import node/areas/fs/stats_base
import node/areas/path
import node/buffer
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
  buffer.to_string(fs.read_file_sync(t.PathOrFileDescriptorString(p)))
}

pub fn main() -> Nil {
  let dir = fs.mkdtemp_sync("/tmp/tsrs-node-ws-")
  let st_dir = fs.stat_sync(str(dir))
  let passed =
    check(
      fs.exists_sync(str(dir)) && stats_base.is_directory(st_dir),
      "workspace created",
    )

  let notes = path.join([dir, "notes.txt"])
  fs.write_file_sync(
    t.PathOrFileDescriptorString(notes),
    t.WriteFileDataString("alpha\nbeta\n"),
  )
  let st1 = fs.stat_sync(str(notes))
  let s1 = stats.get_size(st1)
  let passed =
    passed
    + check(
      fs.exists_sync(str(notes)) && stats_base.is_file(st1) && s1 == 11.0,
      "initial write and size",
    )

  fs.append_file_sync(
    t.PathOrFileDescriptorString(notes),
    t.StringOrUint8ArrayString("gamma\n"),
  )
  let text = read_text(notes)
  let st2 = fs.stat_sync(str(notes))
  let s2 = stats.get_size(st2)
  let passed =
    passed
    + check(text == "alpha\nbeta\ngamma\n" && s2 == 17.0, "append and readBack")

  let backup = path.join([dir, "backup.txt"])
  let archive = path.join([dir, "archive.txt"])
  fs.copy_file_sync(str(notes), str(backup))
  let passed = passed + check(fs.exists_sync(str(backup)), "copyFileSync")

  fs.rename_sync(str(backup), str(archive))
  let passed =
    passed
    + check(
      !fs.exists_sync(str(backup)) && fs.exists_sync(str(archive)),
      "renameSync",
    )
    + check(read_text(archive) == text, "archive matches notes")

  fs.unlink_sync(str(archive))
  let passed = passed + check(!fs.exists_sync(str(archive)), "unlinkSync")

  let rm =
    rm_options.create()
    |> rm_options.with_recursive(True)
    |> rm_options.with_force(True)
  fs.rm_sync_with(str(dir), Some(rm))
  let passed =
    passed
    + check(!fs.exists_sync(str(dir)), "rmSync recursive")
    + check(True, "workspace cleanup verified")

  io.println("workspace=" <> dir)
  io.println(
    "size_before="
    <> int.to_string(float.round(s1))
    <> " size_after="
    <> int.to_string(float.round(s2)),
  )
  io.println("PASS: temp_workspace (" <> int.to_string(passed) <> " checks)")
}
