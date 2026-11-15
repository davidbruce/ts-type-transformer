// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every check must hold.
import bindings/areas/pa
import bindings/areas/pb
import bindings/namespaces/pb_util
import bindings/namespaces/util
import bindings/types as t
import gleam/io

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Classes of one name, each constructed from its package.
  check(pa.node_name(pa.node_new("x")) == "x", "pa's Node")
  check(pb.pb_node_size(pb.pb_node_new(3.0)) == 3.0, "pb's Node")

  // Functions of one name: each package's, told apart by its own class.
  let assert t.NodeOrStringNode(a) = pa.pick(True)
  check(pa.node_name(a) == "a", "pa's pick")
  let assert t.NodeOrStringString(_) = pa.pick(False)
  let assert t.PbNodeOrStringPbNode(n) = pb.pb_pick(True)
  check(pb.pb_node_size(n) == 10.0, "pb's pick")
  let assert t.PbNodeOrStringString(_) = pb.pb_pick(False)

  // pa's own Point, and geo's, a type declared elsewhere of the same name.
  let own = pa.point_new()
  check(pa.point_get_x(own) == 3.0, "pa's Point")
  let _geo: t.Point2 = pa.to_geo(own)

  // Namespaces of one name.
  check(util.util_tag() == "pa", "pa's util")
  check(pb_util.pb_util_tag() == "pb", "pb's util")

  io.println("checks passed")
}
