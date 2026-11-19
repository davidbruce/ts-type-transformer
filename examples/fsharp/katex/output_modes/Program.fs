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
title.textContent <- "Output & Layout Options (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- "html | mathml | leqno | fleqn | maxSize | strict"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let grid = div "grid"
app.appendChild grid |> ignore

let addCard (name: string) (cls: string) (html: string) =
    let card = div ("card " + cls)
    let row = div "head"
    let lbl = div "label"
    lbl.textContent <- name
    let sz = document.createElement "span"
    sz.className <- "badge"
    sz.textContent <- $"{html.Length} chars"
    row.appendChild lbl |> ignore
    row.appendChild sz |> ignore
    let slot = div "slot"
    slot.innerHTML <- html
    card.appendChild row |> ignore
    card.appendChild slot |> ignore
    grid.appendChild card |> ignore

let oHtml = KatexOptions.Create (output = KatexOptionsOutput.Html)
addCard "output: html" "mode-html" (renderToString (@"a^2 + b^2 = c^2", oHtml))

let oMath = KatexOptions.Create (output = KatexOptionsOutput.Mathml)
addCard "output: mathml" "" (renderToString (@"\frac{1}{1 + x^2}", oMath))

let oBoth = KatexOptions.Create (output = KatexOptionsOutput.HtmlAndMathml)
addCard "output: htmlAndMathml" "" (renderToString (@"\sqrt{\alpha + \beta}", oBoth))

let oLeqno = KatexOptions.Create (displayMode = true, leqno = true)
addCard "displayMode + leqno" "" (renderToString (@"\int_0^1 x^2\,dx = \frac{1}{3} \tag{1}", oLeqno))

let oFleqn =
    KatexOptions.Create (
        displayMode = true,
        fleqn = true,
        maxSize = 2.0,
        strict = StringOrBooleanOrStrictFunction.Bool false
    )
addCard "fleqn + maxSize(2) + strict(false)" "" (renderToString (@"\text{café} + \rule{10em}{0.4em} + é", oFleqn))
