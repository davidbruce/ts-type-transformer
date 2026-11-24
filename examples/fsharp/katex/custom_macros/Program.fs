module Demos.Katex.CustomMacros

open Fable.Core.JsInterop
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

let private app = (document.getElementById "app").Value

app.appendChild (wrap "head" [ el "h1" "" "Custom Macros (F#)"; el "span" "badge" @"\RR, \NN, \dd, \norm" ]) |> ignore

let macros =
    JsRecord<string, StringOrObjectOrFn>.Create [
        "\\RR", !^ @"\mathbb{R}"
        "\\NN", !^ @"\mathbb{N}"
        "\\dd", !^ @"\mathrm{d}"
        "\\norm", !^ @"\left\| #1 \right\|"
    ]

let opts = KatexOptions.Create (displayMode = true, macros = macros)

app.appendChild
    (wrap "card" [
        el "div" "label" "Defined Macros"
        el "pre" "tex" "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"
    ])
|> ignore

let grid = el "div" "grid" ""
app.appendChild grid |> ignore

let exprs =
    [ ("Euclidean Space", @"v \in \RR^n,\quad k \in \NN")
      ("Vector Norm", @"\norm{x + y} \le \norm{x} + \norm{y}")
      ("Differential Form", @"\int_a^b f(x)\,\dd x = F(b) - F(a)")
      ("Inner Product Norm", @"\norm{\phi}_2 = \sqrt{\int_{\RR} |\phi(t)|^2\,\dd t}") ]

for (name, tex) in exprs do
    let slot = el "div" "slot" ""
    slot.innerHTML <- renderToString (tex, opts)
    grid.appendChild (wrap "card" [ el "div" "label" name; el "pre" "tex" tex; slot ]) |> ignore
