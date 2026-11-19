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

let sanitizeRaw (raw: string) =
    raw.Replace("\n", "↵")

let app = document.getElementById "app"

let sample =
    "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n"

let tokens = Lexer.lex sample

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Marked Token Inspector (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some $"{int tokens.length} tokens")) |> ignore
app.appendChild head |> ignore

let srcCard = el "div" "card" None
srcCard.appendChild (el "div" "label" (Some "Markdown Input")) |> ignore
srcCard.appendChild (el "pre" "src" (Some sample)) |> ignore
app.appendChild srcCard |> ignore

let tblCard = el "div" "card" None
tblCard.appendChild (el "div" "label" (Some "Lexer Token Stream")) |> ignore

let table = el "table" "tok-table" None
let thead = el "thead" "" None
let hrow = el "tr" "" None
for col in [ "#"; "Type"; "Bytes"; "Raw Snippet" ] do
    hrow.appendChild (el "th" "" (Some col)) |> ignore
thead.appendChild hrow |> ignore
table.appendChild thead |> ignore

let tbody = el "tbody" "" None
for i in 0 .. int tokens.length - 1 do
    let g = unbox<Tokens.Generic> tokens.[float i]
    let tr = el "tr" "" None
    tr.appendChild (el "td" "" (Some (string i))) |> ignore
    tr.appendChild (el "td" "" (Some g.``type``)) |> ignore
    tr.appendChild (el "td" "" (Some (string g.raw.Length))) |> ignore
    tr.appendChild (el "td" "" (Some (sanitizeRaw g.raw))) |> ignore
    tbody.appendChild tr |> ignore

table.appendChild tbody |> ignore
tblCard.appendChild table |> ignore
app.appendChild tblCard |> ignore
