// DOM events from F#: listeners added and removed, options, events made and
// dispatched, bubbling, cancelling, and an AbortSignal. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

let same (a: obj) (b: obj) = obj.ReferenceEquals(a, b)

let make (id: string) : HTMLDivElement =
    let d: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
    d.id <- id
    d

// outer > inner > target, in the page.
let outer = make "outer"
let inner = make "inner"
let target = make "target"
outer.appendChild inner |> ignore
inner.appendChild target |> ignore
document.body.appendChild outer |> ignore

let log = ResizeArray<string>()
let logged () = String.concat "," log
let clear () = log.Clear()
let note (name: string) : EventListener = fun (_: Event) -> log.Add name

// A plain listener, then removed (the same function value is passed).
let onPing = note "ping"
target.addEventListener ("ping", !^ onPing)
check (target.dispatchEvent (Event.Create "ping")) "dispatchEvent returns true when not cancelled"
check (logged () = "ping") "listener called"
target.removeEventListener ("ping", !^ onPing)
target.dispatchEvent (Event.Create "ping") |> ignore
check (logged () = "ping") "listener removed"

// Several listeners run in the order added; the same one added twice runs once.
clear ()
let la = note "a"
let lb = note "b"
target.addEventListener ("seq", !^ la)
target.addEventListener ("seq", !^ lb)
target.addEventListener ("seq", !^ la)
target.dispatchEvent (Event.Create "seq") |> ignore
check (logged () = "a,b") "listeners in order, duplicates ignored"

// once: true.
clear ()
target.addEventListener ("once", !^ (note "once"), AddEventListenerOptions.Create (once = true))
target.dispatchEvent (Event.Create "once") |> ignore
target.dispatchEvent (Event.Create "once") |> ignore
check (logged () = "once") "once listener runs once"

// Bubbling: a bubbling event reaches the ancestors, a non-bubbling one does not.
clear ()
outer.addEventListener ("hit", !^ (note "outer"))
inner.addEventListener ("hit", !^ (note "inner"))
target.addEventListener ("hit", !^ (note "target"))
target.dispatchEvent (Event.Create ("hit", EventInit.Create (bubbles = true))) |> ignore
check (logged () = "target,inner,outer") "bubbles target to outer"
clear ()
target.dispatchEvent (Event.Create "hit") |> ignore
check (logged () = "target") "does not bubble by default"

// FINDING 2: `AddEventListenerOptions.Create (capture = true)` alone binds to the inherited
// EventListenerOptions.Create (Bindings.10.core.fs:400) and returns an EventListenerOptions,
// which addEventListener does not accept; a second named argument picks the right one.
// Capture runs ancestors first.
clear ()
outer.addEventListener ("cap", !^ (note "outer-capture"), true)
outer.addEventListener ("cap", !^ (note "outer-bubble"))
target.addEventListener ("cap", !^ (note "target"))
target.dispatchEvent (Event.Create ("cap", EventInit.Create (bubbles = true))) |> ignore
check (logged () = "outer-capture,target,outer-bubble") "capture before target before bubble"
clear ()
inner.addEventListener ("cap2", !^ (note "inner-capture"), AddEventListenerOptions.Create (capture = true, once = false))
target.addEventListener ("cap2", !^ (note "target"))
target.dispatchEvent (Event.Create "cap2") |> ignore
check (logged () = "inner-capture,target") "capture option reaches a non-bubbling event"

// stopPropagation at the middle.
clear ()
inner.addEventListener ("stop", !^ (fun (e: Event) -> log.Add "inner"; e.stopPropagation ()))
outer.addEventListener ("stop", !^ (note "outer"))
target.dispatchEvent (Event.Create ("stop", EventInit.Create (bubbles = true))) |> ignore
check (logged () = "inner") "stopPropagation"
// stopImmediatePropagation also skips the element's other listeners.
clear ()
target.addEventListener ("imm", !^ (fun (e: Event) -> log.Add "first"; e.stopImmediatePropagation ()))
target.addEventListener ("imm", !^ (note "second"))
target.dispatchEvent (Event.Create "imm") |> ignore
check (logged () = "first") "stopImmediatePropagation"

// target, currentTarget and phase.
let mutable seenTarget = false
let mutable seenCurrent = false
let mutable phase = -1.0
outer.addEventListener (
    "who",
    !^ (fun (e: Event) ->
        seenTarget <- same e.target target
        seenCurrent <- same e.currentTarget outer
        phase <- e.eventPhase)
)
target.dispatchEvent (Event.Create ("who", EventInit.Create (bubbles = true))) |> ignore
check seenTarget "event.target"
check seenCurrent "event.currentTarget"
check (phase = 3.0) "eventPhase is bubbling"

// preventDefault on a cancelable event.
target.addEventListener ("cancel-me", !^ (fun (e: Event) -> e.preventDefault ()))
let cancelable = Event.Create ("cancel-me", EventInit.Create (cancelable = true))
check (not cancelable.defaultPrevented) "not prevented yet"
let result = target.dispatchEvent cancelable
check (cancelable.defaultPrevented) "defaultPrevented"
check (not result) "dispatchEvent returns false when cancelled"
// A non-cancelable event cannot be cancelled.
let fixedEvent = Event.Create "cancel-me"
target.dispatchEvent fixedEvent |> ignore
check (not fixedEvent.defaultPrevented) "non-cancelable stays unprevented"

// passive: preventDefault is ignored.
target.addEventListener ("passive", !^ (fun (e: Event) -> e.preventDefault ()), AddEventListenerOptions.Create (passive = true))
let passiveEvent = Event.Create ("passive", EventInit.Create (cancelable = true))
target.dispatchEvent passiveEvent |> ignore
check (not passiveEvent.defaultPrevented) "passive listener cannot prevent default"

// CustomEvent with detail.
let mutable detail = ""
target.addEventListener (
    "custom",
    !^ (fun (e: Event) ->
        let c = e :?> CustomEvent<string>
        detail <- c.detail)
)
target.dispatchEvent (CustomEvent.Create ("custom", CustomEventInit.Create (detail = "payload"))) |> ignore
check (detail = "payload") "CustomEvent detail"
let mutable number = 0.0
target.addEventListener ("custom-n", !^ (fun (e: Event) -> number <- (e :?> CustomEvent<float>).detail))
target.dispatchEvent (CustomEvent.Create ("custom-n", CustomEventInit.Create (detail = 42.5))) |> ignore
check (number = 42.5) "CustomEvent numeric detail"

// An AbortSignal removes a listener when aborted.
clear ()
let controller = AbortController.Create ()
target.addEventListener ("abortable", !^ (note "abortable"), AddEventListenerOptions.Create (signal = controller.signal))
target.dispatchEvent (Event.Create "abortable") |> ignore
check (not controller.signal.aborted) "signal not aborted"
controller.abort ()
check controller.signal.aborted "signal aborted"
target.dispatchEvent (Event.Create "abortable") |> ignore
check (logged () = "abortable") "aborted signal removed the listener"

// The signal itself is an event target.
let mutable abortEvents = 0
let c2 = AbortController.Create ()
c2.signal.addEventListener ("abort", fun (_: Event) -> abortEvents <- abortEvents + 1)
c2.abort ()
c2.abort ()
check (abortEvents = 1) "abort event fires once"

// A MouseEvent made with an init dictionary, listened for by a typed key.
let button: HTMLButtonElement = document.createElement HTMLElementTagNameMapKeys.button
target.appendChild button |> ignore
let mutable x = 0.0
let mutable y = 0.0
let mutable shift = false
let mutable buttonNo = -1.0
let mutable clicks = 0
let onClick: MouseEvent -> unit =
    fun e ->
        clicks <- clicks + 1
        x <- e.clientX
        y <- e.clientY
        shift <- e.shiftKey
        buttonNo <- e.button
button.addEventListener (
    "click",
    !^ (fun (e: Event) ->
        let m = e :?> MouseEvent
        onClick m)
)
button.dispatchEvent (MouseEvent.Create ("click", MouseEventInit.Create (clientX = 12.0, clientY = 34.0, shiftKey = true, button = 0.0, bubbles = true))) |> ignore
check (clicks = 1) "synthetic click"
check (x = 12.0 && y = 34.0) "MouseEvent coordinates"
check shift "MouseEvent shiftKey"
check (buttonNo = 0.0) "MouseEvent button"
button.click ()
check (clicks = 2) "click() fires the listener"

// A typed listener for the click key, with options.
let mutable typedClicks = 0
// FINDING 3: a typed listener is an F# function; a let-bound lambda compiles to a named JS
// function and every use passes a fresh wrapper (`arg => typed(arg)`), so removeEventListener
// never matches it. Held as a closure value (as tests/dom/test.fs does), the same object is passed.
let typed: PointerEvent -> unit =
    let f = fun (_: PointerEvent) -> typedClicks <- typedClicks + 1
    f
button.addEventListener (HTMLElementEventMapKeys.click, typed)
button.click ()
button.click ()
check (typedClicks = 2) "typed listener"
button.removeEventListener (HTMLElementEventMapKeys.click, typed)
button.click ()
check (typedClicks = 2) "typed listener removed"

// A KeyboardEvent made with an init dictionary.
let mutable key = ""
let mutable code = ""
let mutable ctrl = false
let mutable repeat = true
target.addEventListener (
    "keydown",
    !^ (fun (e: Event) ->
        let k = e :?> KeyboardEvent
        key <- k.key
        code <- k.code
        ctrl <- k.ctrlKey
        repeat <- k.repeat)
)
target.dispatchEvent (KeyboardEvent.Create ("keydown", KeyboardEventInit.Create (key = "a", code = "KeyA", ctrlKey = true))) |> ignore
check (key = "a") "KeyboardEvent key"
check (code = "KeyA") "KeyboardEvent code"
check ctrl "KeyboardEvent ctrlKey"
check (not repeat) "KeyboardEvent repeat defaults to false"

// An event object listener.
clear ()
let listenerObject =
    { new EventListenerObject with
        member _.handleEvent (_: Event) = log.Add "object" }
target.addEventListener ("obj", !^ listenerObject)
target.dispatchEvent (Event.Create "obj") |> ignore
check (logged () = "object") "EventListenerObject"

// The event's own properties.
let e = Event.Create ("props", EventInit.Create (bubbles = true, cancelable = true))
check (e.``type`` = "props" && e.bubbles && e.cancelable && not e.composed) "event properties"
check (not e.isTrusted) "synthetic events are untrusted"

printfn "%d checks passed" passed
