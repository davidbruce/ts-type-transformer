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

// An object that is also a function, and can be constructed.
let s = scaler 2.0
check (s.Invoke 5.0 = 10.0 && s.factor = 2.0 && (s.Create 3.0).Invoke 5.0 = 15.0) "callable object"

// Exceptions are F# exceptions; promises are Fable's JS.Promise.
let message = try explode "boom" |> ignore; "" with e -> e.Message
check (message = "boom") "exception"

// Promises are awaited in an async block; a rejection is an exception.
async {
    let! doubled = Async.AwaitPromise (delayed 21.0)
    check (doubled = 42.0) "a promise resolves"
    let! failed = Async.AwaitPromise (rejected "nope") |> Async.Catch
    check (match failed with Choice2Of2 e -> e.Message = "nope" | _ -> false) "a promise rejects"
    do! Async.AwaitPromise (settled ())

    // An async iterator: `next ()` is a promise of the next value, or of None.
    let ticks = (ticker 3.0).asyncIterator ()
    let mutable sum = 0.0
    let mutable more = true
    while more do
        let! tick = Async.AwaitPromise (ticks.next ())
        match tick with
        | Some t -> sum <- sum + t
        | None -> more <- false
    check (sum = 6.0) "an async iterator"
    printfn "%d checks passed" passed
}
|> Async.StartImmediate
