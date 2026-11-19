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
title.textContent <- "Inline vs Display Mode (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- "displayMode: false vs true"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let inlineOpts = KatexOptions.Create (displayMode = false)
let displayOpts = KatexOptions.Create (displayMode = true)

let items =
    [ ("Basel Series", @"\sum_{k=1}^{n} \frac{1}{k^2} = \frac{\pi^2}{6}")
      ("Dilogarithm", @"\int_0^1 \frac{\ln(1+x)}{x}\,dx")
      ("Sinc Limit", @"\lim_{x \to 0} \frac{\sin x}{x} = 1")
      ("Binomial", @"\binom{n}{k} = \frac{n!}{k!(n-k)!}") ]

for (name, tex) in items do
    let row = div "row"
    for (modeName, opts) in [ ($"{name} — inline", inlineOpts); ($"{name} — display", displayOpts) ] do
        let cell = div "cell"
        let lbl = div "label"
        lbl.textContent <- modeName
        let out = div "slot"
        out.innerHTML <- renderToString (tex, opts)
        cell.appendChild lbl |> ignore
        cell.appendChild out |> ignore
        row.appendChild cell |> ignore
    app.appendChild row |> ignore
