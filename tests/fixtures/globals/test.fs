// Calls the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Global values.
check (settings.level = 2.0 && not settings.debug) "global object"
settings.debug <- true
check settings.debug "global object is live"
check (VERSION = "1.2.3") "global string"

// A global with members of its own.
check (Clock.now () = 1000.0) "member of a global"
check (Clock.ZERO = 0.0) "constant of a global"

// Namespaces, nested ones included: a namespace's functions by its name.
let a = Geometry.point (0.0, 0.0)
let b = Geometry.point (3.0, 4.0)
check (Geometry.distance (a, b) = 5.0) "namespace function"
check (b.x = 3.0) "namespace interface"
check (Geometry.UNIT = 1.0) "namespace value"
check (Geometry.Deep.twice 4.0 = 8.0) "nested namespace"

// Rest parameters.
check (join ("-", "a", "b", "c") = "a-b-c") "rest"
check (join "-" = "") "rest, empty"

// Numeric enums.
check (bump Level.Low = Level.High) "numeric enum"
check (bump Level.High = Level.High) "numeric enum, unchanged"

printfn "%d checks passed" passed
