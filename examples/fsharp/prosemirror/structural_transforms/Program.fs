module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Prosemirror
open Prosemirror.ProsemirrorModel
open Prosemirror.ProsemirrorTransform
open Prosemirror.ProsemirrorState
open Prosemirror.ProsemirrorView

let private tagSpec (tag: string) : DOMOutputSpec =
    DOMOutputSpec.Tuple (ResizeArray<obj> [ box tag; box 0 ])

let private buildSchema () : Schema<obj, obj> =
    let docSpec = createEmpty<NodeSpec>
    docSpec.content <- Some "block+"

    let levelAttr = createEmpty<AttributeSpec>
    levelAttr.``default`` <- Some (box 2)
    let headingSpec = createEmpty<NodeSpec>
    headingSpec.attrs <- Some (unbox (createObj [ "level" ==> levelAttr ]))
    headingSpec.content <- Some "inline*"
    headingSpec.group <- Some "block"
    headingSpec.defining <- Some true
    headingSpec.toDOM <- Some (fun (n: ProsemirrorModel.Node) ->
        let lvl = unbox<float> n.attrs.["level"] |> int
        tagSpec (sprintf "h%d" lvl)
    )

    let paraSpec = createEmpty<NodeSpec>
    paraSpec.content <- Some "inline*"
    paraSpec.group <- Some "block"
    paraSpec.toDOM <- Some (fun _ -> tagSpec "p")

    let quoteSpec = createEmpty<NodeSpec>
    quoteSpec.content <- Some "block+"
    quoteSpec.group <- Some "block"
    quoteSpec.defining <- Some true
    quoteSpec.toDOM <- Some (fun _ -> tagSpec "blockquote")

    let listSpec = createEmpty<NodeSpec>
    listSpec.content <- Some "list_item+"
    listSpec.group <- Some "block"
    listSpec.toDOM <- Some (fun _ -> tagSpec "ul")

    let itemSpec = createEmpty<NodeSpec>
    itemSpec.content <- Some "paragraph block*"
    itemSpec.defining <- Some true
    itemSpec.toDOM <- Some (fun _ -> tagSpec "li")

    let hrSpec = createEmpty<NodeSpec>
    hrSpec.group <- Some "block"
    hrSpec.atom <- Some true
    hrSpec.toDOM <- Some (fun _ -> DOMOutputSpec.Tuple (ResizeArray<obj> [ box "hr" ]))

    let textSpec = createEmpty<NodeSpec>
    textSpec.group <- Some "inline"

    let strongSpec = createEmpty<MarkSpec>
    strongSpec.toDOM <- Some (fun _ _ -> tagSpec "strong")

    let codeSpec = createEmpty<MarkSpec>
    codeSpec.toDOM <- Some (fun _ _ -> tagSpec "code")

    Schema.Create<obj, obj> (
        SchemaSpec.Create (
            nodes = U2.Case1 (createObj [
                "doc" ==> docSpec
                "heading" ==> headingSpec
                "paragraph" ==> paraSpec
                "blockquote" ==> quoteSpec
                "bullet_list" ==> listSpec
                "list_item" ==> itemSpec
                "horizontal_rule" ==> hrSpec
                "text" ==> textSpec
            ]),
            marks = U2.Case1 (createObj [
                "strong" ==> strongSpec
                "code" ==> codeSpec
            ])
        )
    )

let private schema = buildSchema ()

let private p (t: string) =
    schema.node (StringOrNodeType.String "paragraph", content = FragmentOrNodeOrNodeArray.Node (schema.text t))

let private li (t: string) =
    schema.node (StringOrNodeType.String "list_item", content = FragmentOrNodeOrNodeArray.Node (p t))

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            StringOrNodeType.String "heading",
            attrs = unbox (createObj [ "level" ==> 2 ]),
            content = FragmentOrNodeOrNodeArray.Node (schema.text "Structural Transform Pipeline")
        )
    let p1 = p "ProseMirror transforms express structural edits as invertible steps."
    let p2 = p "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."
    let quote =
        schema.node (
            StringOrNodeType.String "blockquote",
            content = FragmentOrNodeOrNodeArray.Node (p "Blockquote content can be lifted back to top-level document depth.")
        )
    let list =
        schema.node (
            StringOrNodeType.String "bullet_list",
            content = FragmentOrNodeOrNodeArray.Array (ResizeArray [
                li "Split at cursor position"
                li "Wrap range with blockquote"
                li "Lift nested block to root"
            ])
        )
    schema.node (
        StringOrNodeType.String "doc",
        content = FragmentOrNodeOrNodeArray.Array (ResizeArray [ h; p1; p2; quote; list ])
    )

let mutable private totalSteps = 0
let mutable private noteCounter = 0
let mutable private lastOp = "Initialized document structure"

let private updateMeta (doc: ProsemirrorModel.Node) =
    let names =
        [ for i in 0 .. (int doc.childCount - 1) -> doc.child(i).``type``.name ]
    let meta = unbox<Dom.HTMLElement> (document.getElementById "meta")
    meta.textContent <-
        sprintf "%s · blocks=%d · outline=[%s] · totalSteps=%d · nodeSize=%d"
            lastOp (int doc.childCount) (String.concat " · " names) totalSteps (int doc.nodeSize)

let private createState () =
    EditorState.create (EditorStateConfig.Create (schema = schema, doc = buildInitialDoc ()))

let main () =
    let mountEl = unbox<Dom.HTMLElement> (document.getElementById "editor")
    let mutable view: EditorView = null

    let props = createEmpty<DirectEditorProps>
    props.state <- createState ()
    props.dispatchTransaction <- Some (fun (tr: Transaction) ->
        if not (isNull view) then
            totalSteps <- totalSteps + tr.steps.Count
            let next = view.state.apply tr
            view.updateState next
            updateMeta next.doc
    )

    view <- EditorView.Create (unbox<dist_DOMNode> mountEl, props)
    updateMeta view.state.doc

    let wire (id: string) (fn: unit -> unit) =
        let el = unbox<Dom.HTMLElement> (document.getElementById id)
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        let st = view.state
        let doc = st.doc
        let selFrom = st.selection.from
        let splitPos =
            if ProsemirrorTransform.canSplit (doc, selFrom) then Some selFrom
            else
                let mutable pos = 0.0
                let mutable found: float option = None
                for i in 0 .. (int doc.childCount - 1) do
                    let ch = doc.child i
                    if found.IsNone && ch.``type``.name = "paragraph" && ch.textContent.Length >= 8 then
                        let mid = pos + 1.0 + float (ch.textContent.Length / 2)
                        if ProsemirrorTransform.canSplit (doc, mid) then found <- Some mid
                    pos <- pos + ch.nodeSize
                found
        match splitPos with
        | Some pos ->
            let tr = st.tr
            tr.split pos |> ignore
            lastOp <- sprintf "Split paragraph at pos %d" (int pos)
            view.dispatch tr
        | None ->
            lastOp <- "No splittable paragraph found"
            updateMeta doc
    )

    wire "btn-1" (fun () ->
        let st = view.state
        let doc = st.doc
        let quoteType = schema.nodes.["blockquote"]
        let wrapRange =
            let rSel = st.selection.``$from``.blockRange st.selection.``$to``
            if not (isNull (box rSel)) && not (isNull (ProsemirrorTransform.findWrapping (rSel, quoteType))) && st.selection.``$from``.depth = 1.0 && st.selection.``$from``.parent.``type``.name = "paragraph" then
                Some rSel
            else
                let mutable pos = 0.0
                let mutable found: NodeRange option = None
                for i in 0 .. (int doc.childCount - 1) do
                    let ch = doc.child i
                    if found.IsNone && ch.``type``.name = "paragraph" then
                        let rp = doc.resolve (pos + 1.0)
                        let r = rp.blockRange rp
                        if not (isNull (box r)) && not (isNull (ProsemirrorTransform.findWrapping (r, quoteType))) then
                            found <- Some r
                    pos <- pos + ch.nodeSize
                found
        match wrapRange with
        | Some range ->
            let wrappers = ProsemirrorTransform.findWrapping (range, quoteType)
            if not (isNull wrappers) then
                let tr = st.tr
                tr.wrap (range, unbox wrappers) |> ignore
                lastOp <- sprintf "Wrapped block [%d..%d] in blockquote" (int range.start) (int range.``end``)
                view.dispatch tr
        | None ->
            lastOp <- "No top-level paragraph left to wrap"
            updateMeta doc
    )

    wire "btn-2" (fun () ->
        let st = view.state
        let doc = st.doc
        let mutable pos = 0.0
        let mutable lifted = false
        for i in 0 .. (int doc.childCount - 1) do
            let ch = doc.child i
            if not lifted && ch.``type``.name = "blockquote" && ch.childCount > 0.0 then
                let rp = doc.resolve (pos + 2.0)
                let range = rp.blockRange rp
                if not (isNull (box range)) then
                    let target = ProsemirrorTransform.liftTarget range
                    if target.HasValue then
                        let tr = st.tr
                        tr.lift (range, target.Value) |> ignore
                        lastOp <- sprintf "Lifted blockquote child to target depth %d" (int target.Value)
                        view.dispatch tr
                        lifted <- true
            pos <- pos + ch.nodeSize
        if not lifted then
            lastOp <- "No blockquote left to lift"
            updateMeta doc
    )

    wire "btn-3" (fun () ->
        noteCounter <- noteCounter + 1
        let st = view.state
        let hr = schema.node (StringOrNodeType.String "horizontal_rule")
        let note = p (sprintf "Appended structural section #%d at doc offset %d." noteCounter (int st.doc.content.size))
        let tr = st.tr
        tr.insert (st.doc.content.size, FragmentOrNodeOrNodeArray.Array (ResizeArray [ hr; note ])) |> ignore
        lastOp <- sprintf "Inserted horizontal_rule + note #%d" noteCounter
        view.dispatch tr
    )

    wire "btn-4" (fun () ->
        totalSteps <- 0
        noteCounter <- 0
        lastOp <- "Reset structural sample"
        let fresh = createState ()
        view.updateState fresh
        updateMeta view.state.doc
    )

main ()
