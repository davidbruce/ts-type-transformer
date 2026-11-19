module Program

open Dom
open type Dom.Globals
open Marked
open type Marked.Globals

let el (tag: string) (cls: string) (txt: string option) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    match txt with
    | Some t -> node.textContent <- t
    | None -> ()
    node

let unwrapHtml (r: StringOrPromise) =
    match r with
    | StringOrPromise.String s -> s
    | StringOrPromise.Promise _ -> ""

let makePane (title: string) (html: string) =
    let card = el "div" "card" None
    card.appendChild (el "div" "label" (Some title)) |> ignore
    let rendered = el "div" "md-out" None
    rendered.innerHTML <- html
    card.appendChild rendered |> ignore
    card.appendChild (el "pre" "html-raw" (Some html)) |> ignore
    card

let app = document.getElementById "app"

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Marked GFM & Line Breaks (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "MarkedOptions")) |> ignore
app.appendChild head |> ignore

let gfmSrc = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n"
let gfmTrueOpts = getDefaults<string, string> ()
gfmTrueOpts.gfm <- Some true
let gfmFalseOpts = getDefaults<string, string> ()
gfmFalseOpts.gfm <- Some false

let gfmSec = el "div" "section" None
gfmSec.appendChild (el "div" "label" (Some "GitHub Flavored Markdown (gfm)")) |> ignore
let gfmGrid = el "div" "grid" None
gfmGrid.appendChild (makePane "gfm: true" (unwrapHtml (marked (gfmSrc, gfmTrueOpts)))) |> ignore
gfmGrid.appendChild (makePane "gfm: false" (unwrapHtml (marked (gfmSrc, gfmFalseOpts)))) |> ignore
gfmSec.appendChild gfmGrid |> ignore
app.appendChild gfmSec |> ignore

let breaksSrc = "Line one\nLine two\nLine three"
let brFalseOpts = getDefaults<string, string> ()
brFalseOpts.breaks <- Some false
let brTrueOpts = getDefaults<string, string> ()
brTrueOpts.breaks <- Some true

let brSec = el "div" "section" None
brSec.appendChild (el "div" "label" (Some "Soft Line Breaks (breaks)")) |> ignore
let brGrid = el "div" "grid" None
brGrid.appendChild (makePane "breaks: false" (unwrapHtml (marked (breaksSrc, brFalseOpts)))) |> ignore
brGrid.appendChild (makePane "breaks: true" (unwrapHtml (marked (breaksSrc, brTrueOpts)))) |> ignore
brSec.appendChild brGrid |> ignore
app.appendChild brSec |> ignore
