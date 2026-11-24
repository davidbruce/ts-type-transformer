module Demos.Prosemirror.StructuralTransforms

open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open Prosemirror
open Prosemirror.ProsemirrorModel
open Prosemirror.ProsemirrorTransform
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
            defining = true,
            toDOM = fun (n: ProsemirrorModel.Node) ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )

    let paraSpec = NodeSpec.Create (content = "inline*", group = "block", toDOM = fun _ -> tagSpec "p")
    let quoteSpec = NodeSpec.Create (content = "block+", group = "block", defining = true, toDOM = fun _ -> tagSpec "blockquote")
    let listSpec = NodeSpec.Create (content = "list_item+", group = "block", toDOM = fun _ -> tagSpec "ul")
    let itemSpec = NodeSpec.Create (content = "paragraph block*", defining = true, toDOM = fun _ -> tagSpec "li")
    let hrSpec = NodeSpec.Create (group = "block", atom = true, toDOM = fun _ -> !^(ResizeArray<obj> [ box "hr" ]))
    let textSpec = NodeSpec.Create (group = "inline")

    let strongSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "strong")
    let codeSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "code")

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
                "blockquote", quoteSpec
                "bullet_list", listSpec
                "list_item", itemSpec
                "horizontal_rule", hrSpec
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
    schema.node ("paragraph", content = schema.text t)

let private li (t: string) =
    schema.node ("list_item", content = p t)

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            "heading",
            attrs = JsRecord.Create [ "level", box 2 ],
            content = schema.text "Structural Transform Pipeline"
        )
    let p1 = p "ProseMirror transforms express structural edits as invertible steps."
    let p2 = p "Wrap blocks in quotes, lift nested content, or split paragraphs cleanly."
    let quote =
        schema.node (
            "blockquote",
            content = p "Blockquote content can be lifted back to top-level document depth."
        )
    let list =
        schema.node (
            "bullet_list",
            content = !^(ResizeArray [
                li "Split at cursor position"
                li "Wrap range with blockquote"
                li "Lift nested block to root"
            ])
        )
    schema.node ("doc", content = !^(ResizeArray [ h; p1; p2; quote; list ]))

let mutable private totalSteps = 0
let mutable private noteCounter = 0
let mutable private lastOp = "Initialized document structure"

let private updateMeta (doc: ProsemirrorModel.Node) =
    let names =
        [ for i in 0 .. (int doc.childCount - 1) -> doc.child(i).``type``.name ]
    let meta = (document.getElementById "meta").Value
    meta.textContent <-
        sprintf "%s · blocks=%d · outline=[%s] · totalSteps=%d · nodeSize=%d"
            lastOp (int doc.childCount) (String.concat " · " names) totalSteps (int doc.nodeSize)

let private createState () =
    EditorState.create (EditorStateConfig.Create (schema = schema, doc = buildInitialDoc ()))

let main () =
    let mountEl = (document.getElementById "editor").Value
    let mutable view: EditorView option = None

    let props =
        DirectEditorProps.Create (
            state = createState (),
            dispatchTransaction = fun (tr: Transaction) ->
                match view with
                | Some v ->
                    totalSteps <- totalSteps + tr.steps.Count
                    let next = v.state.apply tr
                    v.updateState next
                    updateMeta next.doc
                | None -> ()
        )

    let v = EditorView.Create ((mountEl :> Dom.Node), props)
    view <- Some v
    updateMeta v.state.doc

    let wire (id: string) (fn: unit -> unit) =
        let el = (document.getElementById id).Value
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        let st = v.state
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
            v.dispatch tr
        | None ->
            lastOp <- "No splittable paragraph found"
            updateMeta doc
    )

    wire "btn-1" (fun () ->
        let st = v.state
        let doc = st.doc
        let quoteType = schema.nodes.["blockquote"]
        let wrapRange =
            match st.selection.``$from``.blockRange st.selection.``$to`` with
            | Some rSel when (ProsemirrorTransform.findWrapping (rSel, quoteType)).IsSome && st.selection.``$from``.depth = 1.0 && st.selection.``$from``.parent.``type``.name = "paragraph" ->
                Some rSel
            | _ ->
                let mutable pos = 0.0
                let mutable found: NodeRange option = None
                for i in 0 .. (int doc.childCount - 1) do
                    let ch = doc.child i
                    if found.IsNone && ch.``type``.name = "paragraph" then
                        let rp = doc.resolve (pos + 1.0)
                        match rp.blockRange rp with
                        | Some r when (ProsemirrorTransform.findWrapping (r, quoteType)).IsSome ->
                            found <- Some r
                        | _ -> ()
                    pos <- pos + ch.nodeSize
                found
        match wrapRange with
        | Some range ->
            match ProsemirrorTransform.findWrapping (range, quoteType) with
            | Some wrappers ->
                let wrapSpecs =
                    ResizeArray [
                        for w in wrappers -> {| ``type`` = w.``type``; attrs = if isNull w.attrs then None else Some w.attrs |}
                    ]
                let tr = st.tr
                tr.wrap (range, wrapSpecs) |> ignore
                lastOp <- sprintf "Wrapped block [%d..%d] in blockquote" (int range.start) (int range.``end``)
                v.dispatch tr
            | None -> ()
        | None ->
            lastOp <- "No top-level paragraph left to wrap"
            updateMeta doc
    )

    wire "btn-2" (fun () ->
        let st = v.state
        let doc = st.doc
        let mutable pos = 0.0
        let mutable lifted = false
        for i in 0 .. (int doc.childCount - 1) do
            let ch = doc.child i
            if not lifted && ch.``type``.name = "blockquote" && ch.childCount > 0.0 then
                let rp = doc.resolve (pos + 2.0)
                match rp.blockRange rp with
                | Some range ->
                    let target = ProsemirrorTransform.liftTarget range
                    if target.IsSome then
                        let tr = st.tr
                        tr.lift (range, target.Value) |> ignore
                        lastOp <- sprintf "Lifted blockquote child to target depth %d" (int target.Value)
                        v.dispatch tr
                        lifted <- true
                | None -> ()
            pos <- pos + ch.nodeSize
        if not lifted then
            lastOp <- "No blockquote left to lift"
            updateMeta doc
    )

    wire "btn-3" (fun () ->
        noteCounter <- noteCounter + 1
        let st = v.state
        let hr = schema.node "horizontal_rule"
        let note = p (sprintf "Appended structural section #%d at doc offset %d." noteCounter (int st.doc.content.size))
        let tr = st.tr
        tr.insert (st.doc.content.size, ResizeArray [ hr; note ]) |> ignore
        lastOp <- sprintf "Inserted horizontal_rule + note #%d" noteCounter
        v.dispatch tr
    )

    wire "btn-4" (fun () ->
        totalSteps <- 0
        noteCounter <- 0
        lastOp <- "Reset structural sample"
        let fresh = createState ()
        v.updateState fresh
        updateMeta v.state.doc
    )

main ()
