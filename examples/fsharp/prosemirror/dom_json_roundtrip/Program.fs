module Program

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Prosemirror
open Prosemirror.ProsemirrorModel
open Prosemirror.ProsemirrorState
open Prosemirror.ProsemirrorView

let private tagSpec (tag: string) : DOMOutputSpec =
    !^(ResizeArray<obj> [ box tag; box 0 ])

let private parseTag (tag: string) : TagParseRule =
    TagParseRule.Create (tag = tag)

let private parseTagWithLevel (tag: string) (level: int) : TagParseRule =
    TagParseRule.Create (tag = tag, attrs = JsRecord.Create [ "level", box level ])

let private buildSchema () : Schema<obj, obj> =
    let docSpec = NodeSpec.Create (content = "block+")

    let levelAttr = AttributeSpec.Create (``default`` = box 2)
    let headingSpec =
        NodeSpec.Create (
            attrs = JsRecord.Create [ "level", levelAttr ],
            content = "inline*",
            group = "block",
            defining = true,
            parseDOM = ResizeArray [ parseTagWithLevel "h1" 1; parseTagWithLevel "h2" 2 ],
            toDOM = fun (n: ProsemirrorModel.Node) ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )

    let paraSpec =
        NodeSpec.Create (
            content = "inline*",
            group = "block",
            parseDOM = ResizeArray [ parseTag "p" ],
            toDOM = fun _ -> tagSpec "p"
        )

    let quoteSpec =
        NodeSpec.Create (
            content = "block+",
            group = "block",
            defining = true,
            parseDOM = ResizeArray [ parseTag "blockquote" ],
            toDOM = fun _ -> tagSpec "blockquote"
        )

    let textSpec = NodeSpec.Create (group = "inline")
    let strongSpec = MarkSpec.Create (parseDOM = ResizeArray [ !^(parseTag "strong") ], toDOM = fun _ _ -> tagSpec "strong")
    let emSpec = MarkSpec.Create (parseDOM = ResizeArray [ !^(parseTag "em") ], toDOM = fun _ _ -> tagSpec "em")
    let codeSpec = MarkSpec.Create (parseDOM = ResizeArray [ !^(parseTag "code") ], toDOM = fun _ _ -> tagSpec "code")

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
                "blockquote", quoteSpec
                "text", textSpec
            ]),
            marks = !^(JsRecord.Create [
                "strong", strongSpec
                "em", emSpec
                "code", codeSpec
            ])
        )
    )

let private schema = buildSchema ()
let private parser = DOMParser.fromSchema schema
let private serializer = DOMSerializer.fromSchema schema

let private specHtml =
    "<h2>DOMParser &amp; JSON Serialization</h2>" +
    "<p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p>" +
    "<blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote>" +
    "<p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>"

let private changelogHtml =
    "<h2>Release 1.4 Notes</h2>" +
    "<p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p>" +
    "<blockquote><p>All round-trips verified with Node.eq().</p></blockquote>" +
    "<p>Edit directly above to watch HTML output update live.</p>"

let private parseHtml (html: string) : ProsemirrorModel.Node =
    let scratch = document.createElement HTMLElementTagNameMapKeys.div
    scratch.innerHTML <- html
    parser.parse (scratch :> Dom.Node)

let mutable private roundTripStatus = "JSON Node.eq() pending"

let private describeDoc (doc: ProsemirrorModel.Node) : string =
    [
        yield sprintf "doc(blocks=%d, nodeSize=%d)" (int doc.childCount) (int doc.nodeSize)
        for i in 0 .. (int doc.childCount - 1) do
            let b = doc.child i
            let info =
                if b.``type``.name = "heading" then sprintf "heading[level=%d]" (int (unbox<float> b.attrs.["level"]))
                else b.``type``.name
            yield sprintf "  [%d] %s (%d bytes): %s" i info (int b.nodeSize) b.textContent
    ]
    |> String.concat "\n"

let private refreshOutput (doc: ProsemirrorModel.Node) =
    let scratch = document.createElement HTMLElementTagNameMapKeys.div
    serializer.serializeFragment (doc.content, target = !^(scratch :> Dom.HTMLElement)) |> ignore
    let htmlOut = scratch.innerHTML
    let outEl = document.getElementById "output"
    outEl.textContent <-
        sprintf "SERIALIZED HTML (DOMSerializer):\n%s\n\nDOCUMENT AST & JSON ROUND-TRIP:\n%s\n\nROUND-TRIP STATUS: %s"
            htmlOut (describeDoc doc) roundTripStatus
    let meta = document.getElementById "meta"
    meta.textContent <-
        sprintf "%s · blocks=%d · nodeSize=%d · htmlBytes=%d"
            roundTripStatus (int doc.childCount) (int doc.nodeSize) htmlOut.Length

let main () =
    let mountEl = document.getElementById "editor"
    let mutable view: EditorView option = None

    let initialState =
        EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml specHtml))

    let props =
        DirectEditorProps.Create (
            state = initialState,
            dispatchTransaction = fun (tr: Transaction) ->
                match view with
                | Some v ->
                    let next = v.state.apply tr
                    v.updateState next
                    refreshOutput next.doc
                | None -> ()
        )

    let v = EditorView.Create ((mountEl :> Dom.Node), props)
    view <- Some v
    roundTripStatus <- "Parsed Spec HTML via DOMParser"
    refreshOutput v.state.doc

    let wire (id: string) (fn: unit -> unit) =
        let el = document.getElementById id
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        roundTripStatus <- "Parsed Spec HTML via DOMParser"
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml specHtml))
        v.updateState st
        refreshOutput st.doc
    )

    wire "btn-1" (fun () ->
        roundTripStatus <- "Parsed Changelog HTML via DOMParser"
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml changelogHtml))
        v.updateState st
        refreshOutput st.doc
    )

    wire "btn-2" (fun () ->
        let doc = v.state.doc
        let json = doc.toJSON ()
        let rebuilt = schema.nodeFromJSON json
        let ok = doc.eq rebuilt
        roundTripStatus <- sprintf "JSON toJSON->nodeFromJSON eq() = %b (size=%d)" ok (int rebuilt.nodeSize)
        refreshOutput doc
    )

    wire "btn-3" (fun () ->
        let st = v.state
        let doc = st.doc
        if doc.childCount >= 2.0 then
            let insertPos = doc.child(0).nodeSize + doc.child(1).nodeSize - 1.0
            let codeMark = schema.mark "code"
            let span = schema.text (" roundtrip()", ResizeArray [ codeMark ])
            let tr = st.tr
            tr.insert (insertPos, span) |> ignore
            roundTripStatus <- sprintf "Inserted code span at pos %d" (int insertPos)
            v.dispatch tr
    )

    wire "btn-4" (fun () ->
        let doc = v.state.doc
        let scratch = document.createElement HTMLElementTagNameMapKeys.div
        serializer.serializeFragment (doc.content, target = !^(scratch :> Dom.HTMLElement)) |> ignore
        let reparsed = parser.parse (scratch :> Dom.Node)
        let ok = doc.eq reparsed
        roundTripStatus <- sprintf "DOM serializeFragment->parse eq() = %b" ok
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = reparsed))
        v.updateState st
        refreshOutput st.doc
    )

main ()
