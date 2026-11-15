// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every check must hold.
import bindings as b
import gleam/io

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Classes of one name, each constructed from its package.
  check(b.node_name(b.node_new("x")) == "x", "pa's Node")
  check(b.pb_node_size(b.pb_node_new(3.0)) == 3.0, "pb's Node")

  // Functions of one name: each package's, told apart by its own class.
  let assert b.NodeOrStringNode(a) = b.pick(True)
  check(b.node_name(a) == "a", "pa's pick")
  let assert b.NodeOrStringString(_) = b.pick(False)
  let assert b.PbNodeOrStringPbNode(n) = b.pb_pick(True)
  check(b.pb_node_size(n) == 10.0, "pb's pick")
  let assert b.PbNodeOrStringString(_) = b.pb_pick(False)

  // Namespaces of one name.
  check(b.util_tag() == "pa", "pa's util")
  check(b.pb_util_tag() == "pb", "pb's util")

  io.println("checks passed")
}
