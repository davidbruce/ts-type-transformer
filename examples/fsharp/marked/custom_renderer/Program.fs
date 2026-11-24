module Program

open Fable.Core.JsInterop
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

let private app = (document.getElementById "app").Value

let md =
    "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n"

let mutable seen: string list = []

let ext =
    MarkedExtension.Create (
        renderer =
            MarkedExtensionRenderer.Create (
                heading =
                    (fun (h: Tokens.Heading) ->
                        !^ $"<h{int h.depth} class=\"wire-h\">§ {h.text}</h{int h.depth}>\n")
            )
    )

let customInstance = Marked.Create ext
let customTokens = customInstance.lexer md
let private tokenType = function
    | Token.Blockquote _ -> "blockquote"
    | Token.Br _ -> "br"
    | Token.Checkbox _ -> "checkbox"
    | Token.Code _ -> "code"
    | Token.Codespan _ -> "codespan"
    | Token.Def _ -> "def"
    | Token.Del _ -> "del"
    | Token.Em _ -> "em"
    | Token.Escape _ -> "escape"
    | Token.Heading _ -> "heading"
    | Token.Hr _ -> "hr"
    | Token.HTML _ -> "html"
    | Token.Image _ -> "image"
    | Token.Link _ -> "link"
    | Token.List _ -> "list"
    | Token.ListItem _ -> "list_item"
    | Token.Paragraph _ -> "paragraph"
    | Token.Space _ -> "space"
    | Token.Strong _ -> "strong"
    | Token.Table _ -> "table"
    | Token.Tag _ -> "html"
    | Token.Text _ -> "text"
    | Token.Generic g -> g.``type``

customInstance.walkTokens (
    !^customTokens,
    fun t ->
        seen <- tokenType t :: seen
        !^()
) |> ignore
let visited = List.rev seen
let customHtml = customInstance.parser (ResizeArray customTokens)

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked Custom Renderer & Walker (F#)"; el "span" "badge" $"{visited.Length} tokens visited" ]) |> ignore

let mkOutCard (lbl: string) (html: string) =
    let slot = el "div" "md-out" ""
    slot.innerHTML <- html
    wrap "div" "card" [ el "div" "label" lbl; slot ]

app.appendChild (wrap "div" "grid" [ mkOutCard "Default Renderer" (unwrapHtml (marked md)); mkOutCard "Custom Heading Renderer" customHtml ]) |> ignore

app.appendChild
    (wrap "div" "card" [
        el "div" "label" "Visited Tokens (walkTokens)"
        wrap "div" "tok-strip" [ for kind in visited -> el "span" "tok-pill" kind ]
    ])
|> ignore
