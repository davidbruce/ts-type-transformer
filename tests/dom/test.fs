// Drives a real page through the generated DOM bindings; every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings

let mutable passed = 0
let failed = ResizeArray<string>()

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failed.Add what

[<Emit("window.__fsharp = { passed: $0, failed: $1 }")>]
let report (passed: int) (failed: ResizeArray<string>) : unit = jsNative

let body = document.body

// Create an element and put it in the page.
// One method per tag, returning the tag's own element type.
let div: HTMLDivElement = document.createElement_div ()
let box: HTMLElement = div
box.id <- "made-by-fsharp"
box.textContent <- "hello from fsharp"
box.classList.add "box"
body.appendChild box |> ignore
check (obj.ReferenceEquals(document.getElementById "made-by-fsharp", box)) "getElementById"
check (box.classList.contains "box") "classList"

// Read an element that was already in the page, and copy its text.
let seed = document.getElementById "seed"
check (seed.textContent = "seed text") "textContent"
check (seed.getAttribute "data-x" = "42") "getAttribute"
check (isNull (seed.getAttribute "data-missing")) "missing attribute is null"
let copy = document.createElement "p"
copy.textContent <- seed.textContent
box.appendChild copy |> ignore

// A listener called by real clicks, then removed. Held as a value, so that
// removing it passes the same function.
let mutable clicks = 0
let button = document.createElement "button"
button.textContent <- "press"
box.appendChild button |> ignore

// One method per event, with the event's own type.
let onClick: PointerEvent -> obj =
    let f =
        fun (_: PointerEvent) ->
            clicks <- clicks + 1
            box.setAttribute ("data-count", string clicks)
            null

    f

button.addEventListener_click onClick
button.click ()
button.click ()
check (clicks = 2) "listener called"
button.removeEventListener_click onClick
button.click ()
check (clicks = 2) "listener removed"

// An options object, { once: true }, and events made and dispatched here.
let mutable pings = 0

let onPing: EventListener =
    unbox (fun (_: Event) ->
        pings <- pings + 1
        box.setAttribute ("data-pings", string pings))

let once = jsOptions<AddEventListenerOptions> (fun o -> o.once <- Some true)
box.addEventListener ("ping", U2.Case1 onPing, U2.Case2 once)
box.dispatchEvent (Event.Create "ping") |> ignore
box.dispatchEvent (Event.Create "ping") |> ignore
check (pings = 1) "once listener"

// Style, layout and the window.
box.style.setProperty ("color", "red")
check (box.style.color = "red") "style"
check (box.getBoundingClientRect().width > 0.0) "layout"
check (box.children.length = 2.0) "children"
check (innerWidth > 0.0) "window"
document.title <- "driven by fsharp"

report passed failed
