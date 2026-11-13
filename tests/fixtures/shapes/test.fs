// Calls the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Class pairing: constructor, static method, constant.
let s = Shape.Create(Shape_kind.Square, 3.0)
check (s.area () = 9.0) "constructor"
check (Shape.unit().area () = 1.0) "static method"
check (Shape.SIDES = 4.0) "constant"

// Literal union -> enum, in both directions.
check (s.kind = Shape_kind.Square) "enum out"
check (Shape.Create(Shape_kind.Circle, 1.0).kind = Shape_kind.Circle) "enum in"

// Properties.
s.size <- 4.0
check (s.size = 4.0) "property"

// Inheritance, and passing a Shape where a Named is wanted.
check (s.name = "square-3") "inherited property"
check (s.describe () = "square-3 (16)") "inherited method"
check (nameOf s = "square-3") "upcast"

// Overloads.
let big = s.scale 2.0
check (big.size = 8.0) "overload 1"
check (s.scale(2.0, "big").name = "big") "overload 2"

// Optional parameter.
s.resize 2.6
check (s.size = 2.6) "optional omitted"
s.resize (2.6, true)
check (s.size = 3.0) "optional given"

// Union parameter: an erased union.
check (s.label (U2.Case1 "a") = "string:a") "union string"
check (s.label (U2.Case2 7.0) = "number:7") "union number"

// Nullable with `null`: the type's own null.
check (isNull s.parent) "null"
check (obj.ReferenceEquals(big.parent, s)) "non-null"
big.parent <- null
check (isNull big.parent) "set null"

// Global functions.
check (totalArea (s, Shape.unit ()) = 10.0) "function"
check (obj.ReferenceEquals(largest (s, null), s)) "null argument"
check (obj.ReferenceEquals(largest (Shape.unit (), s), s)) "nullable result"

// Nullable with `undefined`: an option.
check ((orUnit None).area () = 1.0) "undefined argument"
check (obj.ReferenceEquals(orUnit (Some s), s)) "defined argument"
check (find "nothing").IsNone "undefined result"
check ((find "unit").Value.area () = 1.0) "defined result"

printfn "%d checks passed" passed
