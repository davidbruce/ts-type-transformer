module Demos.Marked.InlineVsBlock

open Dom
open type Dom.Globals
open Marked
open type Marked.Globals

let private el (tag: string) (cls: string) (text: string) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    if text <> "" then node.textContent <- text
    node

let private wrap (tag: string) (cls: string) (children: Dom.HTMLElement list) =
    let node = el tag cls ""
    for ch in children do
        node.appendChild ch |> ignore
    node

let private unwrapHtml (r: StringOrPromise) =
    match r with
    | StringOrPromise.String s -> s
    | StringOrPromise.Promise _ -> ""

let private makeModeCard (title: string) (html: string) =
    let slot = el "div" "md-out" ""
    slot.innerHTML <- html
    wrap "div" "card" [ el "div" "label" title; slot; el "pre" "html-raw" html ]

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked Inline vs Block (F#)"; el "span" "badge" "parse vs parseInline" ]) |> ignore

let snippets =
    [ "Hello **world** and *italics*"
      "# Heading stays literal inline"
      "Inline `code` and [link](https://example.com)"
      "~~strike~~ and **nested *em***" ]

for s in snippets do
    app.appendChild
        (wrap "div" "row-card" [
            el "div" "label" "Markdown Snippet"
            el "pre" "src" s
            wrap "div" "grid" [
                makeModeCard "Block (parse)" (unwrapHtml (marked s))
                makeModeCard "Inline (parseInline)" (unwrapHtml (parseInline.Invoke s))
            ]
        ])
    |> ignore
