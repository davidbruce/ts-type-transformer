module Program

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

let private makePane (title: string) (html: string) =
    let rendered = el "div" "md-out" ""
    rendered.innerHTML <- html
    wrap "div" "card" [ el "div" "label" title; rendered; el "pre" "html-raw" html ]

let private app = document.getElementById "app"

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked GFM & Line Breaks (F#)"; el "span" "badge" "MarkedOptions" ]) |> ignore

let gfmSrc = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n"
app.appendChild
    (wrap "div" "section" [
        el "div" "label" "GitHub Flavored Markdown (gfm)"
        wrap "div" "grid" [
            makePane "gfm: true" (unwrapHtml (marked (gfmSrc, MarkedOptions.Create (gfm = true))))
            makePane "gfm: false" (unwrapHtml (marked (gfmSrc, MarkedOptions.Create (gfm = false))))
        ]
    ])
|> ignore

let breaksSrc = "Line one\nLine two\nLine three"
app.appendChild
    (wrap "div" "section" [
        el "div" "label" "Soft Line Breaks (breaks)"
        wrap "div" "grid" [
            makePane "breaks: false" (unwrapHtml (marked (breaksSrc, MarkedOptions.Create (breaks = false))))
            makePane "breaks: true" (unwrapHtml (marked (breaksSrc, MarkedOptions.Create (breaks = true))))
        ]
    ])
|> ignore
