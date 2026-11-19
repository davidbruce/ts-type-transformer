module Program

open Fable.Core
open Fable.Core.JsInterop
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

let md =
    "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n"

let seen = ResizeArray<string>()

let ext = createEmpty<MarkedExtension<string, string>>
ext.renderer <- Some (
    MarkedExtensionRenderer.Create (
        heading = RendererSpace.Create (fun args ->
            let h = unbox<Tokens.Heading> args.[0]
            U2.Case2 $"<h{int h.depth} class=\"wire-h\">§ {h.text}</h{int h.depth}>\n")
    )
)

let customInstance = Marked.Create ext
let customTokens = customInstance.lexer md
customInstance.walkTokens (
    TokenArrayOrTokensList.TokensList customTokens,
    fun t ->
        seen.Add (unbox<Tokens.Generic> t).``type``
        U3.Case1 ()
) |> ignore
let customHtml =
    customInstance.parser (ResizeArray [ for i in 0 .. int customTokens.length - 1 -> customTokens.[float i] ])

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Marked Custom Renderer & Walker (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some $"{seen.Count} tokens visited")) |> ignore
app.appendChild head |> ignore

let grid = el "div" "grid" None

let defCard = el "div" "card" None
defCard.appendChild (el "div" "label" (Some "Default Renderer")) |> ignore
let defOut = el "div" "md-out" None
defOut.innerHTML <- unwrapHtml (marked md)
defCard.appendChild defOut |> ignore

let custCard = el "div" "card" None
custCard.appendChild (el "div" "label" (Some "Custom Heading Renderer")) |> ignore
let custOut = el "div" "md-out" None
custOut.innerHTML <- customHtml
custCard.appendChild custOut |> ignore

grid.appendChild defCard |> ignore
grid.appendChild custCard |> ignore
app.appendChild grid |> ignore

let walkCard = el "div" "card" None
walkCard.appendChild (el "div" "label" (Some "Visited Tokens (walkTokens)")) |> ignore
let strip = el "div" "tok-strip" None
for kind in seen do
    strip.appendChild (el "span" "tok-pill" (Some kind)) |> ignore
walkCard.appendChild strip |> ignore
app.appendChild walkCard |> ignore
