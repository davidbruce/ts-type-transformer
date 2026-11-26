module Demos.Marked.GfmAndBreaks

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

let private renderWith (src: string) (ext: MarkedExtension) =
    let m = Marked.Create ext
    m.parse src

let private makePane (title: string) (html: string) =
    let rendered = el "div" "md-out" ""
    rendered.innerHTML <- html
    wrap "div" "card" [ el "div" "label" title; rendered; el "pre" "html-raw" html ]

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked GFM & Line Breaks (F#)"; el "span" "badge" "MarkedExtension" ]) |> ignore

let gfmSrc = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n"
app.appendChild
    (wrap "div" "section" [
        el "div" "label" "GitHub Flavored Markdown (gfm)"
        wrap "div" "grid" [
            makePane "gfm: true" (renderWith gfmSrc (MarkedExtension.Create (gfm = true)))
            makePane "gfm: false" (renderWith gfmSrc (MarkedExtension.Create (gfm = false)))
        ]
    ])
|> ignore

let breaksSrc = "Line one\nLine two\nLine three"
app.appendChild
    (wrap "div" "section" [
        el "div" "label" "Soft Line Breaks (breaks)"
        wrap "div" "grid" [
            makePane "breaks: false" (renderWith breaksSrc (MarkedExtension.Create (breaks = false)))
            makePane "breaks: true" (renderWith breaksSrc (MarkedExtension.Create (breaks = true)))
        ]
    ])
|> ignore
