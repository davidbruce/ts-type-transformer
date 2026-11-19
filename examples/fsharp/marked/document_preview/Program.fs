module Program

open Dom
open type Dom.Globals
open Marked
open type Marked.Globals

let div (cls: string) =
    let el = document.createElement "div"
    el.className <- cls
    el

let unwrapHtml (r: StringOrPromise) =
    match r with
    | StringOrPromise.String s -> s
    | StringOrPromise.Promise _ -> ""

let app = document.getElementById "app"

let head = div "head"
let title = document.createElement "h1"
title.textContent <- "Marked Document Preview (F#)"
let badge = document.createElement "span"
badge.className <- "badge"
badge.textContent <- "marked"
head.appendChild title |> ignore
head.appendChild badge |> ignore
app.appendChild head |> ignore

let markdownSrc =
    """# Marked Wireframe Spec

## Features

Typed bindings with *emphasis*, **strong**, `inline code`, and [reference](https://example.com "Spec").

- Lexer AST
- Custom renderer
- GFM tables

1. Parse
2. Walk
3. Render

> Typed bindings from TypeScript definitions.

```ts
const ok: boolean = true;
```

---
"""

let grid = div "grid"

let srcCard = div "card"
let srcLbl = div "label"
srcLbl.textContent <- "Markdown Source"
let srcPre = document.createElement "pre"
srcPre.className <- "src"
srcPre.textContent <- markdownSrc
srcCard.appendChild srcLbl |> ignore
srcCard.appendChild srcPre |> ignore

let outCard = div "card"
let outLbl = div "label"
outLbl.textContent <- "Rendered HTML"
let mdOut = div "md-out"
mdOut.innerHTML <- unwrapHtml (marked markdownSrc)
outCard.appendChild outLbl |> ignore
outCard.appendChild mdOut |> ignore

grid.appendChild srcCard |> ignore
grid.appendChild outCard |> ignore
app.appendChild grid |> ignore
