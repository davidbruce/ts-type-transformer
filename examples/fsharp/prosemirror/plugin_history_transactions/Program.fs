module Program

open System.Collections.Generic
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
            toDOM = fun (n: ProsemirrorModel.Node) ->
                let lvl = unbox<float> n.attrs.["level"] |> int
                tagSpec (sprintf "h%d" lvl)
        )

    let paraSpec = NodeSpec.Create (content = "inline*", group = "block", toDOM = fun _ -> tagSpec "p")
    let textSpec = NodeSpec.Create (group = "inline")
    let strongSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "strong")
    let codeSpec = MarkSpec.Create (toDOM = fun _ _ -> tagSpec "code")

    Schema.Create (
        SchemaSpec.Create (
            nodes = !^(JsRecord.Create [
                "doc", docSpec
                "heading", headingSpec
                "paragraph", paraSpec
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
            content = !^(schema.text "Step-Inversion Undo & Transaction Filter")
        )
    let p1 = p "Every document change produces atomic Transform steps that can be inverted against prior snapshots."
    let p2 = p "A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."
    let p3 = p "Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."
    schema.node (!^"doc", content = !^(ResizeArray [ h; p1; p2; p3 ]))

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
    let meta = (document.getElementById "meta").Value
    meta.textContent <-
        sprintf "%s · edits=%d · undoDepth=%d · redoDepth=%d · lock=%s · blocked=%d · blocks=%d"
            lastAction edits undoStack.Count redoStack.Count lockLabel blockedCount (int st.doc.childCount)
    let lockBtn = (document.getElementById "btn-4").Value
    if readOnlyLocked then lockBtn.classList.add [| "active" |] else lockBtn.classList.remove [| "active" |]

let main () =
    let mountEl = (document.getElementById "editor").Value
    let mutable view: EditorView option = None

    let field =
        StateField.Create (
            (fun _ _ -> 0),
            (fun (tr: Transaction) (count: int) _ _ -> if tr.docChanged then count + 1 else count)
        )
    let auditPlugin =
        Plugin.Create (
            PluginSpec.Create (
                key = unbox auditKey,
                state = field,
                filterTransaction = fun (tr: Transaction) _ ->
                    if readOnlyLocked && tr.docChanged then
                        blockedCount <- blockedCount + 1
                        lastAction <- "Blocked document mutation (filterTransaction=false)"
                        false
                    else
                        true
            )
        )

    let state =
        EditorState.create (
            EditorStateConfig.Create (
                schema = schema,
                doc = buildInitialDoc (),
                plugins = ResizeArray [ unbox<Plugin<obj>> auditPlugin ]
            )
        )

    let props =
        DirectEditorProps.Create (
            state = state,
            dispatchTransaction = fun (tr: Transaction) ->
                match view with
                | Some v ->
                    let res = v.state.applyTransaction tr
                    if res.transactions.Count > 0 then
                        if tr.docChanged then
                            let inv =
                                [| for i in (tr.steps.Count - 1) .. -1 .. 0 ->
                                    tr.steps.[i].invert tr.docs.[i] |]
                            let histMeta = tr.getMeta "history"
                            if isNull histMeta then
                                undoStack.Push inv
                                redoStack.Clear ()
                            elif string histMeta = "undo" then
                                redoStack.Push inv
                            elif string histMeta = "redo" then
                                undoStack.Push inv
                        v.updateState res.state
                    updateMeta v.state
                | None -> ()
        )

    let v = EditorView.Create ((mountEl :> Dom.Node), props)
    view <- Some v
    updateMeta v.state

    let wire (id: string) (fn: unit -> unit) =
        let el = (document.getElementById id).Value
        el.addEventListener (HTMLElementEventMapKeys.click, fun _ -> fn ())

    wire "btn-0" (fun () ->
        revNum <- revNum + 1
        let st = v.state
        let node = p (sprintf "Revision #%d recorded via Transform step." revNum)
        let tr = st.tr
        tr.insert (st.doc.content.size, node) |> ignore
        if not readOnlyLocked then lastAction <- sprintf "Appended Revision #%d" revNum
        v.dispatch tr
    )

    wire "btn-1" (fun () ->
        let st = v.state
        if st.doc.childCount >= 2.0 then
            let fromP = st.doc.child(0).nodeSize + 1.0
            let toP = fromP + st.doc.child(1).content.size
            if toP > fromP then
                let strong = schema.mark "strong"
                let tr = st.tr
                tr.addMark (fromP, toP, strong) |> ignore
                if not readOnlyLocked then lastAction <- sprintf "Bolded first paragraph [%d..%d]" (int fromP) (int toP)
                v.dispatch tr
    )

    wire "btn-2" (fun () ->
        if undoStack.Count = 0 then
            lastAction <- "Undo stack empty"
            updateMeta v.state
        elif not readOnlyLocked then
            let batch = undoStack.Pop ()
            let tr = v.state.tr.setMeta ("history", box "undo")
            for s in batch do tr.step s |> ignore
            lastAction <- sprintf "Undid %d step(s) via Step.invert" batch.Length
            v.dispatch tr
        else
            let probe = v.state.tr.setMeta ("history", box "undo")
            for s in undoStack.Peek () do probe.step s |> ignore
            v.dispatch probe
    )

    wire "btn-3" (fun () ->
        if redoStack.Count = 0 then
            lastAction <- "Redo stack empty"
            updateMeta v.state
        elif not readOnlyLocked then
            let batch = redoStack.Pop ()
            let tr = v.state.tr.setMeta ("history", box "redo")
            for s in batch do tr.step s |> ignore
            lastAction <- sprintf "Redid %d inverted step(s)" batch.Length
            v.dispatch tr
        else
            let probe = v.state.tr.setMeta ("history", box "redo")
            for s in redoStack.Peek () do probe.step s |> ignore
            v.dispatch probe
    )

    wire "btn-4" (fun () ->
        readOnlyLocked <- not readOnlyLocked
        lastAction <- if readOnlyLocked then "Enabled read-only lock" else "Disabled read-only lock"
        updateMeta v.state
    )

main ()
