module Program

open System
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Prosemirror
open Prosemirror.ProsemirrorModel
open Prosemirror.ProsemirrorState
open Prosemirror.ProsemirrorView

let private tagSpec (tag: string) : DOMOutputSpec =
    !^(ResizeArray<obj> [ box tag; box 0 ])

let private buildSchema () : Schema<obj, obj> =
    let docSpec = NodeSpec.Create (content = "block+")

    let levelAttr = AttributeSpec.Create (``default`` = box 2)
    let headingSpec =
        NodeSpec.Create (
            attrs = JsRecord.Create [ "level", levelAttr ],
            content = "inline*",
            group = "block",
            toDOM = fun (n: ProsemirrorModel.Node) ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )

    let paraSpec = NodeSpec.Create (content = "inline*", group = "block", toDOM = fun _ -> tagSpec "p")

    let codeBlockSpec =
        NodeSpec.Create (
            content = "text*",
            marks = "",
            group = "block",
            code = true,
            toDOM = fun _ -> !^(ResizeArray<obj> [ box "pre"; box [| box "code"; box 0 |] ])
        )

    let textSpec = NodeSpec.Create (group = "inline")
    let strongSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "strong")
    let codeSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "code")

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
                "code_block", codeBlockSpec
                "text", textSpec
            ]),
            marks = !^(JsRecord.Create [
                "strong", strongSpec
                "code", codeSpec
            ])
        )
    )

let private schema = buildSchema ()

let private p (t: string) =
    schema.node (!^"paragraph", content = !^(schema.text t))

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            !^"heading",
            attrs = JsRecord.Create [ "level", box 2 ],
            content = !^(schema.text "Immutable State & View Decorations")
        )
    let p1 = p "A ProseMirror schema validates every document tree before state updates commit to the view."
    let p2 = p "Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema."
    let cb =
        schema.node (
            !^"code_block",
            content = !^(schema.text "const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view")
        )
    let p3 = p "Inline decorations highlight matching tokens while node decorations outline matching blocks in the view."
    schema.node (!^"doc", content = !^(ResizeArray [ h; p1; p2; cb; p3 ]))

let private computeDecorations (doc: ProsemirrorModel.Node) (query: string) : DecorationSet * int * int =
    let q = query.Trim().ToLowerInvariant()
    if q.Length = 0 then
        (DecorationSet.empty, 0, 0)
    else
        let decos = ResizeArray<Decoration> ()
        let mutable inlineHits = 0
        let mutable blockHits = 0
        let inlineAttr = DecorationAttrs.Create (``class`` = "pm-match")
        let blockAttr = DecorationAttrs.Create (``class`` = "pm-hit-block")

        let mutable blockPos = 0.0
        for i in 0 .. (int doc.childCount - 1) do
            let block = doc.child i
            let mutable inlinePos = blockPos + 1.0
            let mutable blockMatched = false
            for j in 0 .. (int block.childCount - 1) do
                let child = block.child j
                if child.isText then
                    let text = child.textContent.ToLowerInvariant()
                    let mutable searchFrom = 0
                    let mutable foundIdx = text.IndexOf(q, searchFrom, StringComparison.Ordinal)
                    while foundIdx >= 0 do
                        let fromP = inlinePos + float foundIdx
                        let toP = fromP + float q.Length
                        decos.Add (Decoration.``inline`` (fromP, toP, inlineAttr))
                        inlineHits <- inlineHits + 1
                        blockMatched <- true
                        searchFrom <- foundIdx + q.Length
                        foundIdx <-
                            if searchFrom <= text.Length then text.IndexOf(q, searchFrom, StringComparison.Ordinal)
                            else -1
                inlinePos <- inlinePos + child.nodeSize
            if blockMatched then
                decos.Add (Decoration.node (blockPos, blockPos + block.nodeSize, blockAttr))
                blockHits <- blockHits + 1
            blockPos <- blockPos + block.nodeSize

        (DecorationSet.create (doc, decos), inlineHits, blockHits)

let private searchKey = PluginKey.Create<string> "search"

let private makeSearchPlugin () : Plugin<string> =
    let field =
        StateField.Create (
            (fun _ _ -> "state"),
            (fun (tr: Transaction) (prev: string) _ _ ->
                let metaVal = tr.getMeta "search"
                if isNull metaVal then prev else string metaVal
            )
        )
    let eProps =
        EditorProps.Create (
            decorations = fun (st: EditorState) ->
                let query = match searchKey.getState st with Some q -> q | None -> ""
                let (dset, _, _) = computeDecorations st.doc query
                Some (dset :> DecorationSource)
        )
    Plugin.Create (PluginSpec.Create (key = unbox searchKey, state = field, props = eProps))

let private syncMeta (st: EditorState) (activeBtnId: string) =
    let query = match searchKey.getState st with Some q -> q | None -> ""
    let (dset, inlineHits, blockHits) = computeDecorations st.doc query
    let decoTotal = dset.find().Count
    let qDisp = if query.Length = 0 then "(none)" else sprintf "\"%s\"" query
    let meta = document.getElementById "meta"
    meta.textContent <-
        sprintf "Query: %s · inline matches=%d · highlighted blocks=%d · decorations=%d"
            qDisp inlineHits blockHits decoTotal
    for id in [ "btn-0"; "btn-1"; "btn-2"; "btn-3"; "btn-4" ] do
        let b = document.getElementById id
        if id = activeBtnId then b.classList.add [| "active" |] else b.classList.remove [| "active" |]

let main () =
    let mountEl = document.getElementById "editor"
    let mutable view: EditorView option = None
    let mutable activeId = "btn-0"

    let state =
        EditorState.create (
            EditorStateConfig.Create (
                schema = schema,
                doc = buildInitialDoc (),
                plugins = ResizeArray [ unbox<Plugin<obj>> (makeSearchPlugin ()) ]
            )
        )

    let props =
        DirectEditorProps.Create (
            state = state,
            dispatchTransaction = fun (tr: Transaction) ->
                match view with
                | Some v ->
                    let next = v.state.apply tr
                    v.updateState next
                    syncMeta next activeId
                | None -> ()
        )

    let v = EditorView.Create ((mountEl :> Dom.Node), props)
    view <- Some v
    syncMeta v.state activeId

    let bindQuery (btnId: string) (q: string) =
        let btn = document.getElementById btnId
        btn.addEventListener (HTMLElementEventMapKeys.click, fun _ ->
            activeId <- btnId
            let tr = v.state.tr.setMeta ("search", box q)
            v.dispatch tr
        )

    bindQuery "btn-0" "state"
    bindQuery "btn-1" "schema"
    bindQuery "btn-2" "transaction"
    bindQuery "btn-3" "view"
    bindQuery "btn-4" ""

main ()
