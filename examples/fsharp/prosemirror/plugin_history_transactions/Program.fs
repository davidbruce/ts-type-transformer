module Program

open System.Collections.Generic
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
    headingSpec.toDOM <- Some (fun (n: ProsemirrorModel.Node) ->
        let lvl = unbox<float> n.attrs.["level"] |> int
        tagSpec (sprintf "h%d" lvl)
    )

    let paraSpec = createEmpty<NodeSpec>
    paraSpec.content <- Some "inline*"
    paraSpec.group <- Some "block"
    paraSpec.toDOM <- Some (fun _ -> tagSpec "p")

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

let private buildInitialDoc () : ProsemirrorModel.Node =
    let h =
        schema.node (
            StringOrNodeType.String "heading",
            attrs = unbox (createObj [ "level" ==> 2 ]),
            content = FragmentOrNodeOrNodeArray.Node (schema.text "Step-Inversion Undo & Transaction Filter")
        )
    let p1 = p "Every document change produces atomic Transform steps that can be inverted against prior snapshots."
    let p2 = p "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."
    let p3 = p "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."
    schema.node (
        StringOrNodeType.String "doc",
        content = FragmentOrNodeOrNodeArray.Array (ResizeArray [ h; p1; p2; p3 ])
    )

let private auditKey = PluginKey.Create<int> "audit"
let mutable private readOnlyLocked = false
let mutable private blockedCount = 0
let mutable private revNum = 0
let mutable private lastAction = "Ready — open write access"

let private undoStack = Stack<Step array> ()
let private redoStack = Stack<Step array> ()

let private updateMeta (st: EditorState) =
    let edits = match auditKey.getState st with Some n -> n | None -> 0
    let lockLabel = if readOnlyLocked then "LOCKED" else "OPEN"
    let meta = unbox<Dom.HTMLElement> (document.getElementById "meta")
    meta.textContent <-
        sprintf "%s · edits=%d · undoDepth=%d · redoDepth=%d · lock=%s · blocked=%d · blocks=%d"
            lastAction edits undoStack.Count redoStack.Count lockLabel blockedCount (int st.doc.childCount)
    let lockBtn = unbox<Dom.HTMLElement> (document.getElementById "btn-4")
    if readOnlyLocked then lockBtn.classList.add [| "active" |] else lockBtn.classList.remove [| "active" |]

let main () =
    let mountEl = unbox<Dom.HTMLElement> (document.getElementById "editor")
    let mutable view: EditorView = null

    let field =
        StateField.Create (
            (fun _ _ -> 0),
            (fun (tr: Transaction) (count: int) _ _ -> if tr.docChanged then count + 1 else count)
        )
    let pSpec = createEmpty<PluginSpec<int>>
    pSpec.key <- Some (unbox auditKey)
    pSpec.state <- Some field
    pSpec.filterTransaction <- Some (fun (tr: Transaction) _ ->
        if readOnlyLocked && tr.docChanged then
            blockedCount <- blockedCount + 1
            lastAction <- "Blocked document mutation (filterTransaction=false)"
            false
        else
            true
    )
    let auditPlugin = Plugin.Create pSpec

    let state =
        EditorState.create (
            EditorStateConfig.Create (
                schema = schema,
                doc = buildInitialDoc (),
                plugins = ResizeArray [ unbox<Plugin<obj>> auditPlugin ]
            )
        )

    let props = createEmpty<DirectEditorProps>
    props.state <- state
    props.dispatchTransaction <- Some (fun (tr: Transaction) ->
        if not (isNull view) then
            let res = view.state.applyTransaction tr
            if res.transactions.Count > 0 then
                if tr.docChanged then
                    let inv =
                        [| for i in (tr.steps.Count - 1) .. -1 .. 0 ->
                            tr.steps.[i].invert tr.docs.[i] |]
                    let histMeta = tr.getMeta (StringOrPluginOrPluginKey.String "history")
                    if isNull histMeta then
                        undoStack.Push inv
                        redoStack.Clear ()
                    elif unbox<string> histMeta = "undo" then
                        redoStack.Push inv
                    elif unbox<string> histMeta = "redo" then
                        undoStack.Push inv
                view.updateState res.state
            updateMeta view.state
    )

    view <- EditorView.Create (unbox<dist_DOMNode> mountEl, props)
    updateMeta view.state

    let wire (id: string) (fn: unit -> unit) =
        let el = unbox<Dom.HTMLElement> (document.getElementById id)
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        revNum <- revNum + 1
        let st = view.state
        let node = p (sprintf "Revision #%d recorded via Transform step." revNum)
        let tr = st.tr
        tr.insert (st.doc.content.size, FragmentOrNodeOrNodeArray.Node node) |> ignore
        if not readOnlyLocked then lastAction <- sprintf "Appended Revision #%d" revNum
        view.dispatch tr
    )

    wire "btn-1" (fun () ->
        let st = view.state
        if st.doc.childCount >= 2.0 then
            let fromP = st.doc.child(0).nodeSize + 1.0
            let toP = fromP + st.doc.child(1).content.size
            if toP > fromP then
                let strong = schema.mark (StringOrMarkType.String "strong")
                let tr = st.tr
                tr.addMark (fromP, toP, strong) |> ignore
                if not readOnlyLocked then lastAction <- sprintf "Bolded first paragraph [%d..%d]" (int fromP) (int toP)
                view.dispatch tr
    )

    wire "btn-2" (fun () ->
        if undoStack.Count = 0 then
            lastAction <- "Undo stack empty"
            updateMeta view.state
        elif not readOnlyLocked then
            let batch = undoStack.Pop ()
            let tr = view.state.tr.setMeta (StringOrPluginOrPluginKey.String "history", box "undo")
            for s in batch do tr.step s |> ignore
            lastAction <- sprintf "Undid %d step(s) via Step.invert" batch.Length
            view.dispatch tr
        else
            let probe = view.state.tr.setMeta (StringOrPluginOrPluginKey.String "history", box "undo")
            for s in undoStack.Peek () do probe.step s |> ignore
            view.dispatch probe
    )

    wire "btn-3" (fun () ->
        if redoStack.Count = 0 then
            lastAction <- "Redo stack empty"
            updateMeta view.state
        elif not readOnlyLocked then
            let batch = redoStack.Pop ()
            let tr = view.state.tr.setMeta (StringOrPluginOrPluginKey.String "history", box "redo")
            for s in batch do tr.step s |> ignore
            lastAction <- sprintf "Redid %d inverted step(s)" batch.Length
            view.dispatch tr
        else
            let probe = view.state.tr.setMeta (StringOrPluginOrPluginKey.String "history", box "redo")
            for s in redoStack.Peek () do probe.step s |> ignore
            view.dispatch probe
    )

    wire "btn-4" (fun () ->
        readOnlyLocked <- not readOnlyLocked
        lastAction <- if readOnlyLocked then "Enabled read-only lock" else "Disabled read-only lock"
        updateMeta view.state
    )

main ()
