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

let private app = (document.getElementById "app").Value

let sample =
    "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n"

let tokens = Lexer.lex sample

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked Token Inspector (F#)"; el "span" "badge" $"{int tokens.length} tokens" ]) |> ignore
app.appendChild (wrap "div" "card" [ el "div" "label" "Markdown Input"; el "pre" "src" sample ]) |> ignore

let private tokenKindRaw = function
    | Token.Blockquote x -> ("blockquote", x.raw)
    | Token.Br x -> ("br", x.raw)
    | Token.Checkbox x -> ("checkbox", x.raw)
    | Token.Code x -> ("code", x.raw)
    | Token.Codespan x -> ("codespan", x.raw)
    | Token.Def x -> ("def", x.raw)
    | Token.Del x -> ("del", x.raw)
    | Token.Em x -> ("em", x.raw)
    | Token.Escape x -> ("escape", x.raw)
    | Token.Heading x -> ("heading", x.raw)
    | Token.Hr x -> ("hr", x.raw)
    | Token.HTML x -> ("html", x.raw)
    | Token.Image x -> ("image", x.raw)
    | Token.Link x -> ("link", x.raw)
    | Token.List x -> ("list", x.raw)
    | Token.ListItem x -> ("list_item", x.raw)
    | Token.Paragraph x -> ("paragraph", x.raw)
    | Token.Space x -> ("space", x.raw)
    | Token.Strong x -> ("strong", x.raw)
    | Token.Table x -> ("table", x.raw)
    | Token.Tag x -> ("html", x.raw)
    | Token.Text x -> ("text", x.raw)
    | Token.Generic x -> (x.``type``, x.raw)

let tbl =
    wrap "table" "tok-table" [
        wrap "thead" "" [ wrap "tr" "" [ for col in [ "#"; "Type"; "Bytes"; "Raw Snippet" ] -> el "th" "" col ] ]
        wrap "tbody" "" [
            for i, tok in Seq.indexed tokens ->
                let kind, raw = tokenKindRaw tok
                wrap "tr" "" [ el "td" "" (string i); el "td" "" kind; el "td" "" (string raw.Length); el "td" "" (sanitizeRaw raw) ]
        ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Lexer Token Stream"; tbl ]) |> ignore
