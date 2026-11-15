// Calls a module library through the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A default export, imported as `default`.
check (origin().x = 0.0) "default export"

// Exported under other names: called and constructed by them.
check (area (3.0, 4.0) = 12.0) "renamed function"
let f = Figure.Create 2.0
check (f.grow () = 3.0 && f.size = 3.0) "renamed class"
check ((Badge.Create "x").label = "x") "re-exported default class"

// Constants are read.
check (LIMIT = 10.0) "constant"
check (bump () = 1.0) "function"

// A union told apart by the classes as exported.
let kind (v: Figure_or_Badge) =
    match v with
    | Figure_or_Badge.Figure f -> "figure " + string f.size
    | Figure_or_Badge.Badge b -> "badge " + b.label
check (kind (pick 0.0) = "figure 2") "instanceof the imported Figure"
check (kind (pick 1.0) = "badge b") "instanceof the imported Badge"

printfn "%d checks passed" passed
