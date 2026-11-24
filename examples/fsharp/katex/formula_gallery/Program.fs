module Demos.Katex.FormulaGallery

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

app.appendChild (wrap "head" [ el "h1" "" "KaTeX Formula Gallery (F#)"; el "span" "badge" $"katex v{version}" ]) |> ignore

let grid = el "div" "grid" ""
app.appendChild grid |> ignore

let opts = KatexOptions.Create (displayMode = true)

let formulas =
    [ ("Euler's Identity", @"e^{i\pi} + 1 = 0")
      ("Quadratic Formula", @"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")
      ("Gaussian Integral", @"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}")
      ("Maxwell-Faraday", @"\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}")
      ("Fourier Transform", @"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)e^{-2\pi i x \xi}\,dx") ]

for (name, tex) in formulas do
    let slot = el "div" "slot" ""
    render (tex, slot, opts)
    grid.appendChild (wrap "card" [ el "div" "label" name; el "pre" "tex" tex; slot ]) |> ignore
