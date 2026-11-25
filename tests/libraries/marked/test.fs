// marked from F#, in Node: markdown to HTML, the lexer's tokens, options and
// extensions. Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// Block level: headings, emphasis, code, lists.
let doc = marked "# Title\n\nSome *emphasis* and **strong** and `code`.\n"
check (doc.Contains "<h1>Title</h1>") "a heading"
check (doc.Contains "<em>emphasis</em>") "emphasis"
check (doc.Contains "<strong>strong</strong>") "strong"
check (doc.Contains "<code>code</code>") "code span"
check (doc.Contains "<p>Some") "a paragraph"

let list = marked "- one\n- two\n- three\n"
check (list.Contains "<ul>" && list.Contains "<li>two</li>") "a list"
let numbered = marked "1. a\n2. b\n"
check (numbered.Contains "<ol>") "an ordered list"

let link = marked "[site](https://example.com \"T\")"
check (link.Contains "<a href=\"https://example.com\" title=\"T\">site</a>") "a link"
let fenced = marked "```js\nlet x = 1 < 2;\n```\n"
check (fenced.Contains "language-js" && fenced.Contains "&lt;") "a fenced block, escaped"
let table = marked "| a | b |\n|---|:-:|\n| 1 | 2 |\n"
check (table.Contains "<table>" && table.Contains "<th align=\"center\">b</th>") "a GFM table"
check ((marked "~~gone~~").Contains "<del>gone</del>") "strikethrough"
check ((marked "> quoted").Contains "<blockquote>") "a blockquote"
check ((marked "---").Contains "<hr>") "a rule"

// Inline only: no paragraph around it.
let inl = parseInline "just *inline*"
check (inl = "just <em>inline</em>") "parseInline"

// Options, given an instance with use: breaks turns a newline into <br>; gfm off drops strikethrough.
let defaults = getDefaults<string, string> ()
check (defaults.breaks = Some false || defaults.breaks = None) "the default is no breaks"
check (not ((marked "a\nb").Contains "<br>")) "newline without breaks"
let breaks = createEmpty<MarkedExtension>
breaks.breaks <- Some true
let soft = Marked.Create ()
soft.``use`` breaks |> ignore
check ((soft.parse "a\nb").Contains "<br>") "breaks"
let nogfm = createEmpty<MarkedExtension>
nogfm.gfm <- Some false
let flat = Marked.Create ()
flat.``use`` nogfm |> ignore
check (not ((flat.parse "~~gone~~").Contains "<del>")) "gfm false"

// The lexer: tokens with their types and raw text.
let tokens = Lexer.lex "# Hi\n\npara *x*\n\n- a\n- b\n"
let kinds = [ for i in 0 .. int tokens.length - 1 -> tokens.[float i].``type`` ]
check (kinds = [ "heading"; "space"; "paragraph"; "space"; "list" ]) "token types"
let heading = match tokens.[0.0] with Token.Heading h -> h | _ -> failwith "expected heading"
check (heading.depth = 1.0 && heading.text = "Hi") "heading token"
check (heading.tokens.Count = 1) "its inline tokens"
let para = match tokens.[2.0] with Token.Paragraph p -> p | _ -> failwith "expected paragraph"
check (para.text = "para *x*") "paragraph token"
check (para.tokens.[1].``type`` = "em") "inline em token"
let lst = match tokens.[4.0] with Token.List l -> l | _ -> failwith "expected list"
check (not lst.ordered && lst.items.Count = 2) "list token"
check (lst.items.[1].text = "b") "list item"

let htmlToks = Lexer.lex "<div>hi</div>\n"
check (match htmlToks.[0.0] with Token.HTML h -> h.block | _ -> false) "html block token case test"
let tagToks = Lexer.lexInline "<span>hi</span>"
check (match tagToks.[0] with Token.Tag t -> t.``type`` = "html" | _ -> false) "html tag token case test"

let inlTokens = Lexer.lexInline "a `b`"
check (inlTokens.Count = 2 && inlTokens.[1].``type`` = "codespan") "lexInline"

let tbl: Tokens.Table = unbox (Lexer.lex "| a | b |\n|:--|--:|\n| 1 | 2 |\n").[0.0]
check (tbl.header.Count = 2 && tbl.rows.Count = 1) "table token"
check (tbl.align.[0] = TokensTableAlign.Left && tbl.align.[1] = TokensTableAlign.Right) "table alignment"
check (tbl.rows.[0].[1].text = "2") "table cell"

// Tokens parsed back to HTML.
let back: string = Parser.parse (ResizeArray [ for i in 0 .. int tokens.length - 1 -> tokens.[float i] ])
check (back.Contains "<h1>Hi</h1>" && back.Contains "<li>b</li>") "Parser.parse"

// Links defined by reference are collected by the lexer.
let refs = Lexer.lex "[x][a]\n\n[a]: https://a.example \"A\"\n"
check (refs.links.["a"].href = "https://a.example") "reference links"

// A Marked instance has its own options.
let m = Marked.Create ()
check (not (isNull (box m)) && not (isNull (box m.defaults))) "Marked instance"
check (m.lexer("# T").length = 1.0) "instance lexer"
let tl = m.lexer "x *y*"
check (m.parser (ResizeArray [ tl.[0.0] ]) = "<p>x <em>y</em></p>\n") "instance parser"

// FINDING 5: `marked.parse` is typed `obj`: not callable.
// Custom renderer and walkTokens extension.
let seen = ResizeArray<string>()
let ext = createEmpty<MarkedExtension>
ext.renderer <- Some (
    MarkedExtensionRenderer.Create (
        heading = (fun (h: Tokens.Heading) ->
            !^ (sprintf "<h%d class=\"custom\">%s</h%d>\n" (int h.depth) h.text (int h.depth)))
    )
)
ext.walkTokens <- Some (fun (t: Token) ->
    seen.Add (unbox<Tokens.Generic> t).``type``
    U3.Case1 ())
let m2 = Marked.Create ext
check (not (isNull (box m2)) && seen.Count = 0) "an instance with an extension"
let customTokens = m2.lexer "# Hi"
let customHtml = m2.parser (ResizeArray [ customTokens.[0.0] ])
check (customHtml = "<h1 class=\"custom\">Hi</h1>\n") "custom renderer override"
m2.walkTokens (TokenArrayOrTokensList.TokensList (Lexer.lex "# a *b*"), fun t -> seen.Add ((unbox<Tokens.Generic> t).``type``); U3.Case1 ()) |> ignore
check (seen.Contains "heading" && seen.Contains "em") "walkTokens visits nested tokens"

printfn "%d checks passed" passed
