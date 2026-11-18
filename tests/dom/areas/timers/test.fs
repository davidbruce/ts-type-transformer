// Timers, microtasks, animation frames and the clock from F#. The checks run
// as the callbacks fire; the report comes last. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let handler (f: unit -> unit) : TimerHandler = !^ (Function.Create f)

let order = ResizeArray<string>()
let note (s: string) = order.Add s

let t0 = performance.now ()
check (t0 >= 0.0) "performance.now is non-negative"
check (performance.timeOrigin > 1.0e12) "timeOrigin is an epoch time"
let t1 = performance.now ()
check (t1 >= t0) "performance.now never goes back"

// Ordering: sync, then microtasks, then (zero-delay) timers.
let timeoutId = Globals.setTimeout (handler (fun () -> note "timeout") , 0.0)
Globals.queueMicrotask (fun () -> note "micro1")
Globals.queueMicrotask (fun () -> note "micro2")
note "sync"
check (timeoutId > 0.0) "setTimeout returns an id"
check (order |> Seq.toList = [ "sync" ]) "nothing has run yet"

// A cleared timeout never fires.
let mutable cancelled = false
let cancelId = Globals.setTimeout (handler (fun () -> cancelled <- true), 5.0)
Globals.clearTimeout (Some cancelId)

// Timers fire in order of their delay.
let fired = ResizeArray<string>()
Globals.setTimeout (handler (fun () -> fired.Add "slow"), 60.0) |> ignore
Globals.setTimeout (handler (fun () -> fired.Add "fast"), 10.0) |> ignore
Globals.setTimeout (handler (fun () -> fired.Add "mid"), 30.0) |> ignore

// Equal delays keep the order they were set in.
let fifo = ResizeArray<string>()
for n in [ "a"; "b"; "c" ] do
    Globals.setTimeout (handler (fun () -> fifo.Add n), 20.0) |> ignore

// Marks and measures on the performance timeline.
performance.mark "start" |> ignore
// An interval counts, then is cleared from inside its callback.
let mutable ticks = 0
let mutable intervalId = 0.0
let started = performance.now ()
intervalId <-
    Globals.setInterval (
        handler (fun () ->
            ticks <- ticks + 1
            if ticks = 4 then Globals.clearInterval (Some intervalId)),
        15.0
    )

// A timeout with extra arguments.
let mutable got = ""
window.setTimeout (!^ (Function.Create (fun (args: ResizeArray<obj>) -> got <- args |> Seq.map string |> String.concat ","; null)), 1.0, "a", "b") |> ignore

// requestAnimationFrame: a callback with a timestamp; cancelled ones never run.
let mutable frames = 0
let mutable frameStamp = -1.0
let mutable frameCancelled = false
let mutable onFrame: FrameRequestCallback = fun _ -> ()
onFrame <-
    fun stamp ->
        frames <- frames + 1
        frameStamp <- stamp
        if frames < 3 then Globals.requestAnimationFrame onFrame |> ignore
let frameId = Globals.requestAnimationFrame onFrame
check (frameId > 0.0) "requestAnimationFrame returns an id"
let cancelledFrame = Globals.requestAnimationFrame (fun _ -> frameCancelled <- true)
Globals.cancelAnimationFrame cancelledFrame

// Wait for everything, polling with a timer.
let rec finish () =
    if ticks >= 4 && frames >= 3 && fired.Count = 3 && fifo.Count = 3 && got <> "" then
        let elapsed = performance.now () - started
        check (order |> Seq.toList = [ "sync"; "micro1"; "micro2"; "timeout" ]) "sync, microtasks, then timers"
        check (not cancelled) "a cleared timeout did not fire"
        check (List.ofSeq fired = [ "fast"; "mid"; "slow" ]) "timers fire by delay"
        check (ticks = 4) "the interval stopped when cleared"
        check (elapsed >= 45.0) "four 15ms ticks took about 60ms"
        check (got = "a,b") "timeout with arguments"
        check (List.ofSeq fifo = [ "a"; "b"; "c" ]) "equal delays fire in order set"
        performance.mark "end" |> ignore
        let measure = performance.measure ("span", "start", "end")
        check (measure.duration >= 45.0 && measure.entryType = "measure") "performance.measure spans the marks"
        check (performance.getEntriesByName("start").Count = 1) "the mark is on the timeline"
        check ((performance.getEntriesByName "span").[0].duration = measure.duration) "the measure is listed"
        check (frames = 3) "animation frames ran, chained"
        check (not frameCancelled) "a cancelled frame did not run"
        check (frameStamp > 0.0 && frameStamp <= performance.now ()) "frame timestamp is a clock reading"
        check (performance.now () > t1) "the clock moved on"
        // After more waiting, nothing more fires.
        Globals.setTimeout (
            handler (fun () ->
                check (ticks = 4) "the cleared interval stays quiet"
                check (not cancelled) "the cleared timeout stays quiet"
                printfn "%d checks passed" passed),
            80.0
        )
        |> ignore
    else
        Globals.setTimeout (handler finish, 10.0) |> ignore

Globals.setTimeout (handler finish, 10.0) |> ignore
