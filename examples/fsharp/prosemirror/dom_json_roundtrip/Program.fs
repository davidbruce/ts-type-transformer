module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Prosemirror
open Prosemirror.ProsemirrorModel
open Prosemirror.ProsemirrorState
open Prosemirror.ProsemirrorView

let private tagSpec (tag: string) : DOMOutputSpec =
    DOMOutputSpec.Tuple (ResizeArray<obj> [ box tag; box 0 ])

let private parseTag (tag: string) : TagParseRule =
    let r = createEmpty<TagParseRule>
    r.tag <- tag
    r

let private buildSchema () : Schema<obj, obj> =
    let docSpec = createEmpty<NodeSpec>
    docSpec.content <- Some "block+"

    let levelAttr = createEmpty<AttributeSpec>
    levelAttr.``default`` <- Some (box 2)
    let h1Rule = parseTag "h1"
    h1Rule.attrs <- Some (unbox (createObj [ "level" ==> 1 ]))
    let h2Rule = parseTag "h2"
    h2Rule.attrs <- Some (unbox (createObj [ "level" ==> 2 ]))

    let headingSpec = createEmpty<NodeSpec>
    headingSpec.attrs <- Some (unbox (createObj [ "level" ==> levelAttr ]))
    headingSpec.content <- Some "inline*"
    headingSpec.group <- Some "block"
    headingSpec.defining <- Some true
    headingSpec.parseDOM <- Some (ResizeArray [ h1Rule; h2Rule ])
    headingSpec.toDOM <- Some (fun (n: ProsemirrorModel.Node) ->
        let lvl = unbox<float> n.attrs.["level"] |> int
        tagSpec (sprintf "h%d" lvl)
    )

    let paraSpec = createEmpty<NodeSpec>
    paraSpec.content <- Some "inline*"
    paraSpec.group <- Some "block"
    paraSpec.parseDOM <- Some (ResizeArray [ parseTag "p" ])
    paraSpec.toDOM <- Some (fun _ -> tagSpec "p")

    let quoteSpec = createEmpty<NodeSpec>
    quoteSpec.content <- Some "block+"
    quoteSpec.group <- Some "block"
    quoteSpec.defining <- Some true
    quoteSpec.parseDOM <- Some (ResizeArray [ parseTag "blockquote" ])
    quoteSpec.toDOM <- Some (fun _ -> tagSpec "blockquote")

    let textSpec = createEmpty<NodeSpec>
    textSpec.group <- Some "inline"

    let strongSpec = createEmpty<MarkSpec>
    strongSpec.parseDOM <- Some (ResizeArray [ ParseRule.TagParseRule (parseTag "strong") ])
    strongSpec.toDOM <- Some (fun _ _ -> tagSpec "strong")

    let emSpec = createEmpty<MarkSpec>
    emSpec.parseDOM <- Some (ResizeArray [ ParseRule.TagParseRule (parseTag "em") ])
    emSpec.toDOM <- Some (fun _ _ -> tagSpec "em")

    let codeSpec = createEmpty<MarkSpec>
    codeSpec.parseDOM <- Some (ResizeArray [ ParseRule.TagParseRule (parseTag "code") ])
    codeSpec.toDOM <- Some (fun _ _ -> tagSpec "code")

    Schema.Create<obj, obj> (
        SchemaSpec.Create (
            nodes = U2.Case1 (createObj [
                "doc" ==> docSpec
                "heading" ==> headingSpec
                "paragraph" ==> paraSpec
                "blockquote" ==> quoteSpec
                "text" ==> textSpec
            ]),
            marks = U2.Case1 (createObj [
                "strong" ==> strongSpec
                "em" ==> emSpec
                "code" ==> codeSpec
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
    let scratch = unbox<Dom.HTMLElement> (document.createElement "div")
    scratch.innerHTML <- html
    parser.parse (unbox<DOMNode> scratch)

let mutable private roundTripStatus = "JSON Node.eq() pending"

let private describeDoc (doc: ProsemirrorModel.Node) : string =
    let lines = ResizeArray<string> ()
    lines.Add (sprintf "doc(blocks=%d, nodeSize=%d)" (int doc.childCount) (int doc.nodeSize))
    for i in 0 .. (int doc.childCount - 1) do
        let b = doc.child i
        let info =
            if b.``type``.name = "heading" then sprintf "heading[level=%d]" (int (unbox<float> b.attrs.["level"]))
            else b.``type``.name
        lines.Add (sprintf "  [%d] %s (%d bytes): %s" i info (int b.nodeSize) b.textContent)
    String.concat "\n" lines

let private refreshOutput (doc: ProsemirrorModel.Node) =
    let scratch = unbox<Dom.HTMLElement> (document.createElement "div")
    serializer.serializeFragment (doc.content, target = HTMLElementOrDocumentFragment.HTMLElement scratch) |> ignore
    let htmlOut = scratch.innerHTML
    let outEl = unbox<Dom.HTMLElement> (document.getElementById "output")
    outEl.textContent <-
        sprintf "SERIALIZED HTML (DOMSerializer):\n%s\n\nDOCUMENT AST & JSON ROUND-TRIP:\n%s\n\nROUND-TRIP STATUS: %s"
            htmlOut (describeDoc doc) roundTripStatus
    let meta = unbox<Dom.HTMLElement> (document.getElementById "meta")
    meta.textContent <-
        sprintf "%s · blocks=%d · nodeSize=%d · htmlBytes=%d"
            roundTripStatus (int doc.childCount) (int doc.nodeSize) htmlOut.Length

let main () =
    let mountEl = unbox<Dom.HTMLElement> (document.getElementById "editor")
    let mutable view: EditorView = null

    let initialState =
        EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml specHtml))

    let props = createEmpty<DirectEditorProps>
    props.state <- initialState
    props.dispatchTransaction <- Some (fun (tr: Transaction) ->
        if not (isNull view) then
            let next = view.state.apply tr
            view.updateState next
            refreshOutput next.doc
    )

    view <- EditorView.Create (unbox<dist_DOMNode> mountEl, props)
    roundTripStatus <- "Parsed Spec HTML via DOMParser"
    refreshOutput view.state.doc

    let wire (id: string) (fn: unit -> unit) =
        let el = unbox<Dom.HTMLElement> (document.getElementById id)
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        roundTripStatus <- "Parsed Spec HTML via DOMParser"
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml specHtml))
        view.updateState st
        refreshOutput st.doc
    )

    wire "btn-1" (fun () ->
        roundTripStatus <- "Parsed Changelog HTML via DOMParser"
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = parseHtml changelogHtml))
        view.updateState st
        refreshOutput st.doc
    )

    wire "btn-2" (fun () ->
        let doc = view.state.doc
        let json = doc.toJSON ()
        let rebuilt = schema.nodeFromJSON json
        let ok = doc.eq rebuilt
        roundTripStatus <- sprintf "JSON toJSON->nodeFromJSON eq() = %b (size=%d)" ok (int rebuilt.nodeSize)
        refreshOutput doc
    )

    wire "btn-3" (fun () ->
        let st = view.state
        let doc = st.doc
        if doc.childCount >= 2.0 then
            let insertPos = doc.child(0).nodeSize + doc.child(1).nodeSize - 1.0
            let codeMark = schema.mark (StringOrMarkType.String "code")
            let span = schema.text (" roundtrip()", ResizeArray [ codeMark ])
            let tr = st.tr
            tr.insert (insertPos, FragmentOrNodeOrNodeArray.Node span) |> ignore
            roundTripStatus <- sprintf "Inserted code span at pos %d" (int insertPos)
            view.dispatch tr
    )

    wire "btn-4" (fun () ->
        let doc = view.state.doc
        let scratch = unbox<Dom.HTMLElement> (document.createElement "div")
        serializer.serializeFragment (doc.content, target = HTMLElementOrDocumentFragment.HTMLElement scratch) |> ignore
        let reparsed = parser.parse (unbox<DOMNode> scratch)
        let ok = doc.eq reparsed
        roundTripStatus <- sprintf "DOM serializeFragment->parse eq() = %b" ok
        let st = EditorState.create (EditorStateConfig.Create (schema = schema, doc = reparsed))
        view.updateState st
        refreshOutput st.doc
    )

main ()
