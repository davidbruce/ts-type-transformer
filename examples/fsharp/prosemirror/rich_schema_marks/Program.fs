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

    let codeBlockSpec = createEmpty<NodeSpec>
    codeBlockSpec.content <- Some "text*"
    codeBlockSpec.marks <- Some ""
    codeBlockSpec.group <- Some "block"
    codeBlockSpec.code <- Some true
    codeBlockSpec.defining <- Some true
    codeBlockSpec.toDOM <- Some (fun _ ->
        DOMOutputSpec.Tuple (ResizeArray<obj> [ box "pre"; box [| box "code"; box 0 |] ])
    )

    let hrSpec = createEmpty<NodeSpec>
    hrSpec.group <- Some "block"
    hrSpec.atom <- Some true
    hrSpec.toDOM <- Some (fun _ -> DOMOutputSpec.Tuple (ResizeArray<obj> [ box "hr" ]))

    let textSpec = createEmpty<NodeSpec>
    textSpec.group <- Some "inline"

    let strongSpec = createEmpty<MarkSpec>
    strongSpec.toDOM <- Some (fun _ _ -> tagSpec "strong")

    let emSpec = createEmpty<MarkSpec>
    emSpec.toDOM <- Some (fun _ _ -> tagSpec "em")

    let codeSpec = createEmpty<MarkSpec>
    codeSpec.toDOM <- Some (fun _ _ -> tagSpec "code")

    Schema.Create<obj, obj> (
        SchemaSpec.Create (
            nodes = U2.Case1 (createObj [
                "doc" ==> docSpec
                "heading" ==> headingSpec
                "paragraph" ==> paraSpec
                "blockquote" ==> quoteSpec
                "code_block" ==> codeBlockSpec
                "horizontal_rule" ==> hrSpec
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

let private p (children: ProsemirrorModel.Node list) =
    schema.node (StringOrNodeType.String "paragraph", content = FragmentOrNodeOrNodeArray.Array (ResizeArray children))

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            StringOrNodeType.String "heading",
            attrs = unbox (createObj [ "level" ==> 2 ]),
            content = FragmentOrNodeOrNodeArray.Node (schema.text "ProseMirror Document Schema")
        )
    let mStrong = schema.mark (StringOrMarkType.String "strong")
    let mEm = schema.mark (StringOrMarkType.String "em")
    let mCode = schema.mark (StringOrMarkType.String "code")
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
            StringOrNodeType.String "blockquote",
            content = FragmentOrNodeOrNodeArray.Node (p [ schema.text "Every document conforms to a strict schema enforced on every transaction." ])
        )
    let codeBlock =
        schema.node (
            StringOrNodeType.String "code_block",
            content = FragmentOrNodeOrNodeArray.Node (schema.text "schema.node(\"paragraph\", null, [schema.text(\"typed\")])")
        )
    let pOutro =
        p [ schema.text "Select any text in this editor and click a toolbar action to toggle marks or switch block types." ]

    schema.node (
        StringOrNodeType.String "doc",
        content = FragmentOrNodeOrNodeArray.Array (ResizeArray [ h; pMarks; quote; codeBlock; pOutro ])
    )

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

let private setActiveButton (markName: string) (id: string) (hasIt: bool) =
    let btn = unbox<Dom.HTMLElement> (document.getElementById id)
    if hasIt then btn.classList.add [| "active" |] else btn.classList.remove [| "active" |]
    ignore markName

let private updateMeta (st: EditorState) =
    let (fromPos, toPos) = targetRange st
    let doc = st.doc
    let hasBold = doc.rangeHasMark (fromPos, toPos, MarkOrMarkType.MarkType schema.marks.["strong"])
    let hasEm = doc.rangeHasMark (fromPos, toPos, MarkOrMarkType.MarkType schema.marks.["em"])
    let hasCode = doc.rangeHasMark (fromPos, toPos, MarkOrMarkType.MarkType schema.marks.["code"])
    setActiveButton "strong" "btn-0" hasBold
    setActiveButton "em" "btn-1" hasEm
    setActiveButton "code" "btn-2" hasCode

    let blockName = st.selection.``$from``.parent.``type``.name
    let activeMarks =
        [ if hasBold then yield "strong"
          if hasEm then yield "em"
          if hasCode then yield "code" ]
    let marksText = if activeMarks.IsEmpty then "none" else String.concat ", " activeMarks
    let meta = unbox<Dom.HTMLElement> (document.getElementById "meta")
    meta.textContent <-
        sprintf "%s · blocks=%d · nodeSize=%d · range=[%d..%d] · block=%s · marks=[%s] · dispatches=%d"
            lastAction (int doc.childCount) (int doc.nodeSize) (int fromPos) (int toPos) blockName marksText dispatches

let private createState () =
    EditorState.create (EditorStateConfig.Create (schema = schema, doc = buildInitialDoc ()))

let main () =
    let mountEl = unbox<Dom.HTMLElement> (document.getElementById "editor")
    let mutable view: EditorView = null

    let props = createEmpty<DirectEditorProps>
    props.state <- createState ()
    props.dispatchTransaction <- Some (fun (tr: Transaction) ->
        if not (isNull view) then
            dispatches <- dispatches + 1
            let next = view.state.apply tr
            view.updateState next
            updateMeta view.state
    )

    view <- EditorView.Create (unbox<dist_DOMNode> mountEl, props)
    updateMeta view.state

    let toggleMark (markName: string) (label: string) =
        let st = view.state
        let (f, t) = targetRange st
        if t > f then
            let mType = schema.marks.[markName]
            let tr = st.tr
            if st.doc.rangeHasMark (f, t, MarkOrMarkType.MarkType mType) then
                tr.removeMark (f, t, MarkOrMarkType.MarkType mType) |> ignore
                lastAction <- sprintf "Removed %s on [%d..%d]" label (int f) (int t)
            else
                let m = schema.mark (StringOrMarkType.MarkType mType)
                tr.addMark (f, t, m) |> ignore
                lastAction <- sprintf "Added %s on [%d..%d]" label (int f) (int t)
            view.dispatch tr

    let wire (id: string) (fn: unit -> unit) =
        let el = unbox<Dom.HTMLElement> (document.getElementById id)
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () -> toggleMark "strong" "bold")
    wire "btn-1" (fun () -> toggleMark "em" "italic")
    wire "btn-2" (fun () -> toggleMark "code" "code")
    wire "btn-3" (fun () ->
        let st = view.state
        let sel = st.selection
        let parentType = sel.``$from``.parent.``type``.name
        let tr = st.tr
        if parentType = "heading" then
            tr.setBlockType (sel.from, Some sel.``to``, schema.nodes.["paragraph"]) |> ignore
            lastAction <- "Converted heading -> paragraph"
        else
            tr.setBlockType (sel.from, Some sel.``to``, schema.nodes.["heading"], AttrsOrFn.Attrs (unbox (createObj [ "level" ==> 2 ]))) |> ignore
            lastAction <- "Converted block -> heading[level=2]"
        view.dispatch tr
    )
    wire "btn-4" (fun () ->
        dispatches <- 0
        lastAction <- "Reset initial schema document"
        let fresh = createState ()
        view.updateState fresh
        updateMeta view.state
    )

main ()
