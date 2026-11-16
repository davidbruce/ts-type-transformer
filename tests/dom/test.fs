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
// A typed key selects the element type: createElement "div" returns an HTMLDivElement.
let div: HTMLDivElement = document.createElement HTMLElementTagNameMapKeys.div
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

// A typed key selects the event type: the listener of "click" takes a PointerEvent.
let onClick: PointerEvent -> unit =
    let f =
        fun (_: PointerEvent) ->
            clicks <- clicks + 1
            box.setAttribute ("data-count", string clicks)

    f

button.addEventListener (HTMLElementEventMapKeys.click, onClick)
button.click ()
button.click ()
check (clicks = 2) "listener called"
button.removeEventListener (HTMLElementEventMapKeys.click, onClick)
button.click ()
check (clicks = 2) "listener removed"

// An options object, { once: true }, and events made and dispatched here.
let mutable pings = 0

let onPing: EventListener =
    fun (_: Event) ->
        pings <- pings + 1
        box.setAttribute ("data-pings", string pings)

// An options bag is made with named arguments.
let once = AddEventListenerOptions.Create (once = true)
// Union arguments take a value of any of their types; a function, by `!^`.
box.addEventListener ("ping", !^ onPing, once)
box.dispatchEvent (Event.Create "ping") |> ignore
box.dispatchEvent (Event.Create "ping") |> ignore
check (pings = 1) "once listener"

// Style, layout and the window.
box.style.setProperty ("color", "red")
check (box.style.color = "red") "style"
check (box.getBoundingClientRect().width > 0.0) "layout"
check (box.children.length = 2.0) "children"
// A subtype where `Node | string` is wanted: `!^`.
let holder = document.createElement "section"
holder.append (!^ (document.createElement "span"), "tail")
check (holder.childNodes.length = 2.0) "append a subtype and a string"
// Collections are seqs.
let tags = [ for child in box.children -> child.tagName ]
check (tags.Length = 2) "children iterate"
check (innerWidth > 0.0) "window"
document.title <- "driven by fsharp"

report passed failed
