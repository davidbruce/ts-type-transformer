module Demos.Node.EventBus

open Node

let mutable private passed = 0

let private check (ok: bool) (what: string) =
    if ok then
        passed <- passed + 1
    else
        failwithf "check failed: %s" what

let emitter = Events.EventEmitter.Create ()
emitter.setMaxListeners 12.0 |> ignore
check (emitter.getMaxListeners () = 12.0) "maxListeners"

let mutable metricCalls = 0
let mutable metricSum = 0.0
let mutable onceCalls = 0
let mutable shutdownCalls = 0

let onMetric =
    Events.EventEmitterAddListenerListener.Create(fun args ->
        metricCalls <- metricCalls + 1
        if args.Count > 0 then
            metricSum <- metricSum + unbox<float> args.[0])

let onOnce =
    Events.EventEmitterAddListenerListener.Create(fun _ -> onceCalls <- onceCalls + 1)

let onShutdown =
    Events.EventEmitterAddListenerListener.Create(fun _ -> shutdownCalls <- shutdownCalls + 1)

emitter.on ("metric", onMetric) |> ignore
emitter.once ("metric", onOnce) |> ignore
emitter.on ("shutdown", onShutdown) |> ignore

check (emitter.listenerCount "metric" = 2.0) "initial listenerCount"
check (emitter.eventNames().Count = 2) "eventNames"

let emitted1 = emitter.emit ("metric", box 101.0, box "ok")
check (emitted1 && metricCalls = 1 && onceCalls = 1) "first metric emit"
check (emitter.listenerCount "metric" = 1.0) "once removes itself"

let emitted2 = emitter.emit ("metric", box 202.0, box "ok")
check (emitted2 && metricCalls = 2 && metricSum = 303.0 && onceCalls = 1) "second metric emit"

check (not (emitter.emit "idle")) "idle emit false"

emitter.removeListener ("metric", onMetric) |> ignore
check (emitter.listenerCount "metric" = 0.0 && not (emitter.emit ("metric", box 1.0))) "removeListener"
check (metricCalls = 2) "metric count unchanged after remove"

emitter.removeAllListeners "shutdown" |> ignore
check (emitter.eventNames().Count = 0 && shutdownCalls = 0) "removeAllListeners"

printfn "maxListeners=%.0f metricCalls=%d metricSum=%.0f" (emitter.getMaxListeners ()) metricCalls metricSum
printfn "onceCalls=%d shutdownCalls=%d" onceCalls shutdownCalls
printfn "PASS: event_bus (%d checks)" passed
