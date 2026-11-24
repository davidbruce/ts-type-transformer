module Demos.Prosemirror.RichSchemaMarks

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
            defining = true,
            toDOM = fun (n: ProsemirrorModel.Node) ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )

    let paraSpec = NodeSpec.Create (content = "inline*", group = "block", toDOM = fun _ -> tagSpec "p")
    let quoteSpec = NodeSpec.Create (content = "block+", group = "block", defining = true, toDOM = fun _ -> tagSpec "blockquote")

    let codeBlockSpec =
        NodeSpec.Create (
            content = "text*",
            marks = "",
            group = "block",
            code = true,
            defining = true,
            toDOM = fun _ -> !^(ResizeArray<obj> [ box "pre"; box [| box "code"; box 0 |] ])
        )

    let hrSpec = NodeSpec.Create (group = "block", atom = true, toDOM = fun _ -> !^(ResizeArray<obj> [ box "hr" ]))
    let textSpec = NodeSpec.Create (group = "inline")

    let strongSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "strong")
    let emSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "em")
    let codeSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "code")

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
                "blockquote", quoteSpec
                "code_block", codeBlockSpec
                "horizontal_rule", hrSpec
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

let private p (children: ProsemirrorModel.Node list) =
    schema.node ("paragraph", content = !^(ResizeArray children))

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            "heading",
            attrs = JsRecord.Create [ "level", box 2 ],
            content = schema.text "ProseMirror Document Schema"
        )
    let mStrong = schema.mark "strong"
    let mEm = schema.mark "em"
    let mCode = schema.mark "code"
    let pMarks =
        p [
            schema.text "Inline marks compose cleanly: "
            schema.text ("bold", ResizeArray [ mStrong ])
            schema.text ", "
            schema.text ("italic", ResizeArray [ mEm ])
            schema.text ", and "
            schema.text ("inline_code", ResizeArray [ mCode ])
            schema.text " spans."
        ]
    let quote =
        schema.node (
            "blockquote",
            content = p [ schema.text "Every document conforms to a strict schema enforced on every transaction." ]
        )
    let codeBlock =
        schema.node (
            "code_block",
            content = schema.text "schema.node(\"paragraph\", null, [schema.text(\"typed\")])"
        )
    let pOutro =
        p [ schema.text "Select any text in this editor and click a toolbar action to toggle marks or switch block types." ]

    schema.node ("doc", content = !^(ResizeArray [ h; pMarks; quote; codeBlock; pOutro ]))

let mutable private dispatches = 0
let mutable private lastAction = "Initialized schema with 7 node types & 3 marks"

let private targetRange (st: EditorState) : float * float =
    let sel = st.selection
    if not sel.empty then
        (sel.from, sel.``to``)
    else
        let doc = st.doc
        if doc.childCount >= 2.0 then
            let c0 = doc.child 0
            let c1 = doc.child 1
            let startPos = c0.nodeSize + 1.0
            let len = c1.content.size
            if len > 0.0 then
                let endPos = startPos + (if len > 12.0 then 12.0 else len)
                (startPos, endPos)
            else
                (1.0, if doc.content.size > 2.0 then 2.0 else doc.content.size)
        else
            (1.0, doc.content.size - 1.0)

let private setActiveButton (id: string) (hasIt: bool) =
    let btn = (document.getElementById id).Value
    if hasIt then btn.classList.add [| "active" |] else btn.classList.remove [| "active" |]

let private updateMeta (st: EditorState) =
    let (fromPos, toPos) = targetRange st
    let doc = st.doc
    let hasBold = doc.rangeHasMark (fromPos, toPos, schema.marks.["strong"])
    let hasEm = doc.rangeHasMark (fromPos, toPos, schema.marks.["em"])
    let hasCode = doc.rangeHasMark (fromPos, toPos, schema.marks.["code"])
    setActiveButton "btn-0" hasBold
    setActiveButton "btn-1" hasEm
    setActiveButton "btn-2" hasCode

    let blockName = st.selection.``$from``.parent.``type``.name
    let activeMarks =
        [ if hasBold then yield "strong"
          if hasEm then yield "em"
          if hasCode then yield "code" ]
    let marksText = if activeMarks.IsEmpty then "none" else String.concat ", " activeMarks
    let meta = (document.getElementById "meta").Value
    meta.textContent <-
        sprintf "%s · blocks=%d · nodeSize=%d · range=[%d..%d] · block=%s · marks=[%s] · dispatches=%d"
            lastAction (int doc.childCount) (int doc.nodeSize) (int fromPos) (int toPos) blockName marksText dispatches

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
                    dispatches <- dispatches + 1
                    let next = v.state.apply tr
                    v.updateState next
                    updateMeta v.state
                | None -> ()
        )

    let v = EditorView.Create ((mountEl :> Dom.Node), props)
    view <- Some v
    updateMeta v.state

    let toggleMark (markName: string) (label: string) =
        let st = v.state
        let (f, t) = targetRange st
        if t > f then
            let mType = schema.marks.[markName]
            let tr = st.tr
            if st.doc.rangeHasMark (f, t, mType) then
                tr.removeMark (f, t, mType) |> ignore
                lastAction <- sprintf "Removed %s on [%d..%d]" label (int f) (int t)
            else
                let m = schema.mark mType
                tr.addMark (f, t, m) |> ignore
                lastAction <- sprintf "Added %s on [%d..%d]" label (int f) (int t)
            v.dispatch tr

    let wire (id: string) (fn: unit -> unit) =
        let el = (document.getElementById id).Value
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () -> toggleMark "strong" "bold")
    wire "btn-1" (fun () -> toggleMark "em" "italic")
    wire "btn-2" (fun () -> toggleMark "code" "code")
    wire "btn-3" (fun () ->
        let st = v.state
        let sel = st.selection
        let parentType = sel.``$from``.parent.``type``.name
        let tr = st.tr
        if parentType = "heading" then
            tr.setBlockType (sel.from, Some sel.``to``, schema.nodes.["paragraph"]) |> ignore
            lastAction <- "Converted heading -> paragraph"
        else
            tr.setBlockType (sel.from, Some sel.``to``, schema.nodes.["heading"], JsRecord.Create [ "level", box 2 ]) |> ignore
            lastAction <- "Converted block -> heading[level=2]"
        v.dispatch tr
    )
    wire "btn-4" (fun () ->
        dispatches <- 0
        lastAction <- "Reset initial schema document"
        let fresh = createState ()
        v.updateState fresh
        updateMeta v.state
    )

main ()
