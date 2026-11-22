module Program

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

app.appendChild
    (wrap "head" [ el "h1" "" "Output & Layout Options (F#)"; el "span" "badge" "html | mathml | leqno | fleqn | maxSize | strict" ])
|> ignore

let grid = el "div" "grid" ""
app.appendChild grid |> ignore

let addCard (name: string) (cls: string) (html: string) =
    let slot = el "div" "slot" ""
    slot.innerHTML <- html
    grid.appendChild
        (wrap ("card " + cls) [
            wrap "head" [ el "div" "label" name; el "span" "badge" $"{html.Length} chars" ]
            slot
        ])
    |> ignore

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
        strict = false
    )
addCard "fleqn + maxSize(2) + strict(false)" "" (renderToString (@"\text{café} + \rule{10em}{0.4em} + é", oFleqn))
