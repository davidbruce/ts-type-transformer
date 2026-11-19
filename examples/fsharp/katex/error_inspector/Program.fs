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
title.textContent <- "ParseError Inspector (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- "throwOnError: false vs true"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let lenientOpts = KatexOptions.Create (throwOnError = false, errorColor = "#555555")
let strictOpts = KatexOptions.Create (throwOnError = true)

let badInputs =
    [ @"\frac{1}{"
      @"\unknowncmd{x}"
      @"x^{2 + "
      @"\sqrt{a + b" ]

for tex in badInputs do
    let row = div "row"
    let left = div "cell mode-html"
    let lLbl = div "label"
    lLbl.textContent <- "throwOnError: false (#555555)"
    let lOut = div "slot"
    lOut.innerHTML <- renderToString (tex, lenientOpts)
    left.appendChild lLbl |> ignore
    left.appendChild lOut |> ignore

    let right = div "cell"
    let rLbl = div "label"
    rLbl.textContent <- "throwOnError: true (caught ParseError)"
    let rOut = div "err"
    let msg =
        try
            renderToString (tex, strictOpts) |> ignore
            "(did not throw)"
        with ex -> ex.Message
    rOut.textContent <- msg
    right.appendChild rLbl |> ignore
    right.appendChild rOut |> ignore

    row.appendChild left |> ignore
    row.appendChild right |> ignore
    app.appendChild row |> ignore
