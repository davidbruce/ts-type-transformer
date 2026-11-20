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

let private sanitizeRaw (raw: string) =
    raw.Replace("\n", "↵")

let private app = document.getElementById "app"

let sample =
    "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n"

let tokens = Lexer.lex sample

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked Token Inspector (F#)"; el "span" "badge" $"{int tokens.length} tokens" ]) |> ignore
app.appendChild (wrap "div" "card" [ el "div" "label" "Markdown Input"; el "pre" "src" sample ]) |> ignore

let tbl =
    wrap "table" "tok-table" [
        wrap "thead" "" [ wrap "tr" "" [ for col in [ "#"; "Type"; "Bytes"; "Raw Snippet" ] -> el "th" "" col ] ]
        wrap "tbody" "" [
            for i, tok in Seq.indexed tokens ->
                let g = unbox<Tokens.Generic> tok
                wrap "tr" "" [ el "td" "" (string i); el "td" "" g.``type``; el "td" "" (string g.raw.Length); el "td" "" (sanitizeRaw g.raw) ]
        ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Lexer Token Stream"; tbl ]) |> ignore
