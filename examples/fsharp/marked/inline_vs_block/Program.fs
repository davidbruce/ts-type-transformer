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

let app = document.getElementById "app"

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Marked Inline vs Block (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "parse vs parseInline")) |> ignore
app.appendChild head |> ignore

let snippets =
    [ "Hello **world** and *italics*"
      "# Heading stays literal inline"
      "Inline `code` and [link](https://example.com)"
      "~~strike~~ and **nested *em***" ]

for s in snippets do
    let row = el "div" "row-card" None
    row.appendChild (el "div" "label" (Some "Markdown Snippet")) |> ignore
    row.appendChild (el "pre" "src" (Some s)) |> ignore

    let cols = el "div" "grid" None

    let blockHtml = unwrapHtml (marked s)
    let blockCard = el "div" "card" None
    blockCard.appendChild (el "div" "label" (Some "Block (parse)")) |> ignore
    let blockOut = el "div" "md-out" None
    blockOut.innerHTML <- blockHtml
    blockCard.appendChild blockOut |> ignore
    blockCard.appendChild (el "pre" "html-raw" (Some blockHtml)) |> ignore

    let inlHtml = unwrapHtml (parseInline.Invoke s)
    let inlCard = el "div" "card" None
    inlCard.appendChild (el "div" "label" (Some "Inline (parseInline)")) |> ignore
    let inlOut = el "div" "md-out" None
    inlOut.innerHTML <- inlHtml
    inlCard.appendChild inlOut |> ignore
    inlCard.appendChild (el "pre" "html-raw" (Some inlHtml)) |> ignore

    cols.appendChild blockCard |> ignore
    cols.appendChild inlCard |> ignore
    row.appendChild cols |> ignore
    app.appendChild row |> ignore
