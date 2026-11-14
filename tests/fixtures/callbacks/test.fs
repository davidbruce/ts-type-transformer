// Calls the generated F# bindings; every check must hold.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let c = Counter.Create()
let mutable last = 0.0
let mutable calls = 0

// Fable wraps a module-level function in a new lambda each time it is passed,
// so a callback that must keep its identity is held as a value.
let onValue: Listener =
    let f =
        fun value ->
            last <- value
            calls <- calls + 1

    f

// A callback is called.
c.subscribe onValue
check (c.increment () = 1.0) "result"
check (last = 1.0) "callback argument"
c.increment 4.0 |> ignore
check (last = 5.0 && calls = 2) "optional argument"

// A callback that takes an object and returns a value.
check (c.visit (fun counter -> counter.count = 5.0)) "object argument, result"

// The same function value is the same JavaScript function.
c.unsubscribe onValue
c.increment () |> ignore
check (last = 5.0 && calls = 2) "identity"

// A nullable callback property.
check c.onchange.IsNone "null callback"
let mutable prop = 0.0
c.onchange <- Some(fun value -> prop <- value)
check c.onchange.IsSome "callback set"
c.increment () |> ignore
check (prop = 7.0) "property callback"
c.onchange <- None
c.increment () |> ignore
check (prop = 7.0) "callback cleared"

// Several arguments, a string among them, and a result.
check (c.fold (fun total label -> total * 2.0 + float label.Length) = 8.0 * 2.0 + 5.0) "two arguments"

// A callback kept and called later.
let mutable ran = 0
later (fun () -> ran <- ran + 1)
later (fun () -> ran <- ran + 1)
check (ran = 0) "not called yet"
check (flush () = 2.0) "flush"
check (ran = 2) "called later"

// Exceptions are F# exceptions; promises are Fable's JS.Promise.
let message = try explode "boom" |> ignore; "" with e -> e.Message
check (message = "boom") "exception"
let pending: JS.Promise<float> = delayed 21.0
check (not (isNull (box pending))) "promise"

printfn "%d checks passed" passed
