// F# keeps TypeScript's names and namespaces: nothing here clashes. Every check must hold.
module Test

open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let g = Gizmo.Create ()
check (g.isTransforming ()) "method"
check (not (Gizmo.isTransforming ())) "static"
check (g.fooBar () = 1.0 && g.foo_bar () = 2.0) "fooBar and foo_bar"
let s = shape ()
check (s.area () = 3.0 && Shape_area s = 30.0) "method and function"
let p = point 4.0
check (p.x = 4.0 && (p.create 5.0).x = 5.0) "property create"
check ((ns_Node.Create ()).id () = 2.0) "class"
check ((ns.Node.Create ()).id () = 1.0) "namespace's class"

printfn "%d checks passed" passed
