import gleam/int
import gleam/io
import gleam/option.{None, Some}
import node/areas/path
import node/areas/path/format_input_path_object as path_format
import node/areas/path/parsed_path

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() -> Nil {
  let joined =
    path.join(["/workspace", "packages", "core", "..", "cli", "index.ts"])
  let norm = path.normalize("/var//log/./app/../http/access.log")
  let resolved =
    path.resolve(["/srv/app", "../shared", "config/prod.json"])
  let rel = path.relative("/srv/app/src/ui", "/srv/app/dist/assets")

  let parsed = path.parse("/srv/app/archive.tar.gz")
  let p_root = parsed_path.get_root(parsed)
  let p_dir = parsed_path.get_dir(parsed)
  let p_base = parsed_path.get_base(parsed)
  let p_name = parsed_path.get_name(parsed)
  let p_ext = parsed_path.get_ext(parsed)

  let fmt =
    path_format.create()
    |> path_format.with_dir("/etc/service")
    |> path_format.with_name("daemon")
    |> path_format.with_ext(".conf")
  let formatted = path.format(fmt)

  let passed =
    check(joined == "/workspace/packages/cli/index.ts", "path.join")
    + check(norm == "/var/log/http/access.log", "path.normalize")
    + check(resolved == "/srv/shared/config/prod.json", "path.resolve")
    + check(rel == "../../dist/assets", "path.relative")
    + check(path.dirname("/srv/app/archive.tar.gz") == "/srv/app", "path.dirname")
    + check(
      path.basename("/srv/app/archive.tar.gz", None) == "archive.tar.gz",
      "path.basename",
    )
    + check(
      path.basename("/srv/app/archive.tar.gz", Some(".gz")) == "archive.tar",
      "path.basename suffix",
    )
    + check(path.extname("/srv/app/archive.tar.gz") == ".gz", "path.extname")
    + check(
      path.is_absolute("/srv/app") && !path.is_absolute("src/app"),
      "path.is_absolute",
    )
    + check(
      p_root == "/"
      && p_dir == "/srv/app"
      && p_base == "archive.tar.gz"
      && p_name == "archive.tar"
      && p_ext == ".gz",
      "path.parse",
    )
    + check(formatted == "/etc/service/daemon.conf", "path.format")
    + check(
      path.matches_glob("/src/components/Button.tsx", "/src/**/*.tsx")
      && !path.matches_glob("/src/components/Button.css", "/src/**/*.tsx"),
      "path.matchesGlob",
    )

  io.println("join=" <> joined <> " norm=" <> norm)
  io.println(
    "resolve=" <> resolved <> " relative=" <> rel <> " format=" <> formatted,
  )
  io.println(
    "parsed=(root="
    <> p_root
    <> " dir="
    <> p_dir
    <> " base="
    <> p_base
    <> " ext="
    <> p_ext
    <> ")",
  )
  io.println("PASS: path_inspector (" <> int.to_string(passed) <> " checks)")
}
