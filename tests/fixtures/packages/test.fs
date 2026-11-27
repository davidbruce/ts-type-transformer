// Two packages that both export `Node`, `pick` and `util`: each call reaches
// its own package's. Every check must hold.
module Test

open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Classes of one name, each in a module of its package.
check ((Pa.Node.Create "x").name () = "x") "pa's Node"
check ((Pb.Node.Create 3.0).size () = 3.0) "pb's Node"

// Functions of one name: each package's, told apart by its own class.
let a =
    match Pa.pick true with
    | PickResult.Node n -> n.name ()
    | PickResult.String s -> s
check (a = "a") "pa's pick"
let b =
    match Pb.pick true with
    | PbPickResult.Node n -> n.size ()
    | PbPickResult.String _ -> 0.0
check (b = 10.0) "pb's pick"
check ((match Pb.pick false with PbPickResult.String s -> s | _ -> "") = "small") "pb's pick, a string"

// pa's own Point, and geo's (transitive dep), with its `lat` field.
let own = Pa.Point.Create ()
check (own.x = 3.0) "pa's Point"
let geo: geo_Point = Pa.toGeo own
check (geo.lat = 3.0) "geo's Point"

// Namespaces of one name.
check (Pa.util.tag () = "pa") "pa's util"
check (Pb.util.tag () = "pb") "pb's util"

printfn "%d checks passed" passed
