import bindings as b
import bindings/perms
import gleam/io
import gleam/list

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  let re = b.combine_perms([perms.Read], [perms.Exec])
  check(
    list.contains(re, perms.Read) && list.contains(re, perms.Exec),
    "combine flags",
  )
  check(
    b.has_perm([perms.Read, perms.Write], [perms.Write]),
    "has combined flag",
  )
  check(
    !b.has_perm([perms.Read, perms.Write], [perms.Exec]),
    "missing flag",
  )
  check(
    b.has_perm([perms.ReadWrite], [perms.Read]),
    "composite constant decomposed",
  )
  let def = b.default_perms()
  check(
    list.contains(def, perms.Read) && list.contains(def, perms.Exec),
    "default flags",
  )
  check(
    b.default_perms_with([perms.Write]) == [perms.Write],
    "explicit flag",
  )
  io.println("checks passed")
}
