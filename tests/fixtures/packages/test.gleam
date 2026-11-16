// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every check must hold.
import bindings/areas/pa
import bindings/areas/pb
import bindings/areas/pa/util
import bindings/areas/pb/util as pb_util
import bindings/types as t
import gleam/io
import bindings/areas/pa/node
import bindings/areas/pa/point
import bindings/areas/pb/node as pb_node

fn check(ok: Bool, what: String) -> Nil {
  case ok {
    True -> Nil
    False -> panic as what
  }
}

pub fn main() {
  // Classes of one name, each constructed from its package.
  check(node.name(node.new("x")) == "x", "pa's Node")
  check(pb_node.size(pb_node.new(3.0)) == 3.0, "pb's Node")

  // Functions of one name: each package's, told apart by its own class.
  let assert t.NodeOrStringNode(a) = pa.pick(True)
  check(node.name(a) == "a", "pa's pick")
  let assert t.NodeOrStringString(_) = pa.pick(False)
  let assert t.PbNodeOrStringPbNode(n) = pb.pick(True)
  check(pb_node.size(n) == 10.0, "pb's pick")
  let assert t.PbNodeOrStringString(_) = pb.pick(False)

  // pa's own Point, and geo's, a type declared elsewhere of the same name.
  let own = point.new()
  check(point.get_x(own) == 3.0, "pa's Point")
  let _geo: t.Point2 = pa.to_geo(own)

  // Namespaces of one name.
  check(util.tag() == "pa", "pa's util")
  check(pb_util.tag() == "pb", "pb's util")

  io.println("checks passed")
}
