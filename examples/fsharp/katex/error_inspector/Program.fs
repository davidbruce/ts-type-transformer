module Program

open Dom
open type Dom.Globals
open Katex
open type Katex.Globals

let private el (tag: string) (cls: string) (text: string) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    if text <> "" then node.textContent <- text
    node

let private wrap (cls: string) (children: Dom.HTMLElement list) =
    let node = el "div" cls ""
    for ch in children do
        node.appendChild ch |> ignore
    node

let private app = document.getElementById "app"

app.appendChild (wrap "head" [ el "h1" "" "ParseError Inspector (F#)"; el "span" "badge" "throwOnError: false vs true" ]) |> ignore

let lenientOpts = KatexOptions.Create (throwOnError = false, errorColor = "#555555")
let strictOpts = KatexOptions.Create (throwOnError = true)

let badInputs =
    [ @"\frac{1}{"
      @"\unknowncmd{x}"
      @"x^{2 + "
      @"\sqrt{a + b" ]

for tex in badInputs do
    let lOut = el "div" "slot" ""
    lOut.innerHTML <- renderToString (tex, lenientOpts)
    let left = wrap "cell mode-html" [ el "div" "label" "throwOnError: false (#555555)"; lOut ]

    let msg =
        try
            renderToString (tex, strictOpts) |> ignore
            "(did not throw)"
        with ex -> ex.Message
    let right = wrap "cell" [ el "div" "label" "throwOnError: true (caught ParseError)"; el "div" "err" msg ]

    app.appendChild (wrap "row" [ left; right ]) |> ignore
