// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every check must hold.
module Test

open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Classes of one name, each constructed from its package.
check ((Node.Create "x").name () = "x") "pa's Node"
check ((pb_Node.Create 3.0).size () = 3.0) "pb's Node"

// Functions of one name: each package's, told apart by its own class.
let a =
    match pick true with
    | Node_or_string.Node n -> n.name ()
    | Node_or_string.String s -> s
check (a = "a") "pa's pick"
let b =
    match pb_pick true with
    | pb_Node_or_string.Pb_Node n -> n.size ()
    | pb_Node_or_string.String _ -> 0.0
check (b = 10.0) "pb's pick"
check ((match pb_pick false with pb_Node_or_string.String s -> s | _ -> "") = "small") "pb's pick, a string"

// pa's own Point, and geo's, a type declared elsewhere of the same name.
let own = Point.Create ()
check (own.x = 3.0) "pa's Point"
let geo: External.Point = toGeo own
check (not (isNull geo)) "geo's Point"

// Namespaces of one name.
check (util.Globals.tag () = "pa") "pa's util"
check (pb_util.Globals.tag () = "pb") "pb's util"

printfn "%d checks passed" passed
