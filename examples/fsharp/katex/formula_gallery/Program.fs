module Program

open Dom
open type Dom.Globals
open Katex
open type Katex.Globals

let div (cls: string) =
    let el = document.createElement "div"
    el.className <- cls
    el

let app = document.getElementById "app"

let head = div "head"
let title = document.createElement "h1"
title.textContent <- "KaTeX Formula Gallery (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- $"katex v{version}"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let grid = div "grid"
app.appendChild grid |> ignore

let opts = KatexOptions.Create (displayMode = true)

let formulas =
    [ ("Euler's Identity", @"e^{i\pi} + 1 = 0")
      ("Quadratic Formula", @"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}")
      ("Gaussian Integral", @"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}")
      ("Maxwell-Faraday", @"\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}")
      ("Fourier Transform", @"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)e^{-2\pi i x \xi}\,dx") ]

for (name, tex) in formulas do
    let card = div "card"
    let lbl = div "label"
    lbl.textContent <- name
    let src = document.createElement "pre"
    src.className <- "tex"
    src.textContent <- tex
    let slot = div "slot"
    render (tex, slot, opts)
    card.appendChild lbl |> ignore
    card.appendChild src |> ignore
    card.appendChild slot |> ignore
    grid.appendChild card |> ignore
