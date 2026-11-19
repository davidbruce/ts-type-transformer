module Program

open Fable.Core
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
title.textContent <- "Custom Macros (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- @"\RR, \NN, \dd, \norm"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let macros = JsRecord<string, StringOrObjectOrFn>.Create ()
macros.["\\RR"] <- U3.Case1 @"\mathbb{R}"
macros.["\\NN"] <- U3.Case1 @"\mathbb{N}"
macros.["\\dd"] <- U3.Case1 @"\mathrm{d}"
macros.["\\norm"] <- U3.Case1 @"\left\| #1 \right\|"

let opts = KatexOptions.Create (displayMode = true, macros = macros)

let legend = div "card"
let legLbl = div "label"
legLbl.textContent <- "Defined Macros"
let legPre = document.createElement "pre"
legPre.className <- "tex"
legPre.textContent <- "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"
legend.appendChild legLbl |> ignore
legend.appendChild legPre |> ignore
app.appendChild legend |> ignore

let grid = div "grid"
app.appendChild grid |> ignore

let exprs =
    [ ("Euclidean Space", @"v \in \RR^n,\quad k \in \NN")
      ("Vector Norm", @"\norm{x + y} \le \norm{x} + \norm{y}")
      ("Differential Form", @"\int_a^b f(x)\,\dd x = F(b) - F(a)")
      ("Inner Product Norm", @"\norm{\phi}_2 = \sqrt{\int_{\RR} |\phi(t)|^2\,\dd t}") ]

for (name, tex) in exprs do
    let card = div "card"
    let lbl = div "label"
    lbl.textContent <- name
    let src = document.createElement "pre"
    src.className <- "tex"
    src.textContent <- tex
    let slot = div "slot"
    slot.innerHTML <- renderToString (tex, opts)
    card.appendChild lbl |> ignore
    card.appendChild src |> ignore
    card.appendChild slot |> ignore
    grid.appendChild card |> ignore
