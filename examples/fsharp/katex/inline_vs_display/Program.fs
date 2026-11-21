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

let private app = (document.getElementById "app").Value

app.appendChild (wrap "head" [ el "h1" "" "Inline vs Display Mode (F#)"; el "span" "badge" "displayMode: false vs true" ]) |> ignore

let inlineOpts = KatexOptions.Create (displayMode = false)
let displayOpts = KatexOptions.Create (displayMode = true)

let items =
    [ ("Basel Series", @"\sum_{k=1}^{n} \frac{1}{k^2} = \frac{\pi^2}{6}")
      ("Dilogarithm", @"\int_0^1 \frac{\ln(1+x)}{x}\,dx")
      ("Sinc Limit", @"\lim_{x \to 0} \frac{\sin x}{x} = 1")
      ("Binomial", @"\binom{n}{k} = \frac{n!}{k!(n-k)!}") ]

for (name, tex) in items do
    let cells =
        [ ($"{name} — inline", inlineOpts); ($"{name} — display", displayOpts) ]
        |> List.map (fun (modeName, opts) ->
            let out = el "div" "slot" ""
            out.innerHTML <- renderToString (tex, opts)
            wrap "cell" [ el "div" "label" modeName; out ])
    app.appendChild (wrap "row" cells) |> ignore
