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

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Marked Document Preview (F#)"; el "span" "badge" "marked" ]) |> ignore

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

let mdOut = el "div" "md-out" ""
mdOut.innerHTML <- unwrapHtml (marked markdownSrc)

app.appendChild
    (wrap "div" "grid" [
        wrap "div" "card" [ el "div" "label" "Markdown Source"; el "pre" "src" markdownSrc ]
        wrap "div" "card" [ el "div" "label" "Rendered HTML"; mdOut ]
    ])
|> ignore
