// ProseMirror from F#, in a page: a schema, documents from JSON, editor state,
// transactions, a plugin with state, and an EditorView on the page's element.
// Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open Bindings.ProsemirrorModel
open Bindings.ProsemirrorState
open Bindings.ProsemirrorView

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A schema: doc of paragraphs of text, with a strong mark. Specs are options objects with settable fields.
let docSpec = createEmpty<NodeSpec>
docSpec.content <- Some "paragraph+"
let paragraphSpec = createEmpty<NodeSpec>
paragraphSpec.content <- Some "inline*"
paragraphSpec.group <- Some "block"
paragraphSpec.toDOM <- Some (fun (_: Node) -> DOMOutputSpec.Tuple (ResizeArray<obj> [ box "p"; box 0 ]))
let textSpec = createEmpty<NodeSpec>
textSpec.group <- Some "inline"
let strongSpec = createEmpty<MarkSpec>
strongSpec.toDOM <- Some (fun (_: Mark) (_: bool) -> DOMOutputSpec.Tuple (ResizeArray<obj> [ box "strong"; box 0 ]))

let schema =
    Schema.Create<obj, obj> (
        SchemaSpec.Create (
            nodes = U2.Case1 (createObj [ "doc" ==> docSpec; "paragraph" ==> paragraphSpec; "text" ==> textSpec ]),
            marks = U2.Case1 (createObj [ "strong" ==> strongSpec ])
        )
    )
check (schema.nodes.["doc"].name = "doc") "the schema's doc type"
check (schema.nodes.["paragraph"].isTextblock && schema.nodes.["text"].isText) "node type flags"
check (schema.topNodeType.name = "doc") "topNodeType"
check (schema.marks.["strong"].name = "strong") "a mark type"
check (schema.nodes.["paragraph"].inlineContent) "a paragraph has inline content"

// Documents: made by the schema, and from JSON.
let hello = schema.text "Hello"
check (hello.isText && hello.text = Some "Hello" && hello.nodeSize = 5.0) "a text node"
let para = schema.node (StringOrNodeType.String "paragraph", content = FragmentOrNodeOrNodeArray.Node hello)
check (para.nodeSize = 7.0 && para.childCount = 1.0) "a paragraph's size is its text plus two"
check (para.textContent = "Hello") "textContent"
let json: obj =
    createObj [
        "type" ==> "doc"
        "content" ==> [| createObj [ "type" ==> "paragraph"
                                     "content" ==> [| createObj [ "type" ==> "text"; "text" ==> "one" ] |] ]
                         createObj [ "type" ==> "paragraph"
                                     "content" ==> [| createObj [ "type" ==> "text"; "text" ==> "two words" ] |] ] |]
    ]
let doc = schema.nodeFromJSON json
check (doc.``type``.name = "doc" && doc.childCount = 2.0) "nodeFromJSON"
check (doc.textContent = "onetwo words") "the doc's text"
check (doc.nodeSize = 18.0 && doc.content.size = 16.0) "nodeSize"
check (doc.child(1).textContent = "two words") "child"
check (doc.textBetween (0.0, doc.content.size, " ") = "one two words") "textBetween"
check ((doc.nodeAt 1.0).Value.isText) "nodeAt"
check (doc.eq (schema.nodeFromJSON json)) "equal documents"
check (doc.resolve(3.0).parent.``type``.name = "paragraph") "resolve"
check (doc.resolve(3.0).parentOffset = 2.0) "parentOffset"

// A strong mark.
let strong = schema.mark (StringOrMarkType.String "strong")
check (strong.``type``.name = "strong") "a mark"
let bold = schema.text ("bold", ResizeArray [ strong ])
check (bold.marks.Count = 1 && strong.isInSet bold.marks) "marks on a text node"
check (strong.eq (schema.mark (StringOrMarkType.String "strong"))) "marks compare"

// State: a document and a selection at its start.
let state = EditorState.create (EditorStateConfig.Create (schema = schema, doc = doc))
check (state.doc.eq doc) "EditorState.create"
check (state.selection.from = 1.0 && state.selection.``to`` = 1.0 && state.selection.empty) "the initial selection"

// A transaction: text inserted at the cursor, and the state it makes.
let tr = state.tr.insertText ("X", 1.0)
check (tr.docChanged && tr.steps.Count = 1) "insertText makes a step"
let next = state.apply tr
check (next.doc.child(0).textContent = "Xone") "apply"
check (state.doc.child(0).textContent = "one") "the old state is unchanged"
check (next.selection.from = 2.0) "the selection moves after the insertion"

// More: delete, addMark, setSelection.
// FINDING 28: the Transform methods that return `this` (addMark, delete, replace ...) are typed to return Transform,
// not Transaction: `state.apply (state.tr.addMark (...))` does not compile. They are called as statements on the transaction.
let tr2 = next.tr
tr2.addMark (1.0, 4.0, strong) |> ignore
let marked = next.apply tr2
check (marked.doc.child(0).child(0).marks.Count = 1) "addMark"
check (marked.doc.rangeHasMark (1.0, 4.0, MarkOrMarkType.Mark strong)) "rangeHasMark"
let tr3 = marked.tr
tr3.delete (1.0, 3.0) |> ignore
check ((marked.apply tr3).doc.child(0).textContent = "ne") "delete"
let sel = TextSelection.create (marked.doc, 2.0, 4.0)
let selected = marked.apply (marked.tr.setSelection sel)
check (selected.selection.from = 2.0 && selected.selection.``to`` = 4.0 && not selected.selection.empty) "setSelection"
check (selected.doc.textBetween (selected.selection.from, selected.selection.``to``) = "on") "the selected text"
let replaced = selected.apply (selected.tr.replaceSelectionWith (schema.text "ZZ", false))
check (replaced.doc.child(0).textContent = "XZZe") "replaceSelectionWith"

// A plugin with state: counts the transactions that changed the document.
let key = PluginKey.Create<int> "counter"
let field =
    StateField.Create (
        (fun (_: EditorStateConfig) (_: EditorState) -> 0),
        (fun (tr: Transaction) (count: int) (_: EditorState) (_: EditorState) -> if tr.docChanged then count + 1 else count)
    )
let pluginSpec = createEmpty<PluginSpec<int>>
pluginSpec.key <- Some (unbox key)
pluginSpec.state <- Some field
let counter = Plugin.Create pluginSpec
let withPlugin = EditorState.create (EditorStateConfig.Create (schema = schema, doc = doc, plugins = ResizeArray [ unbox counter ]))
check (key.getState withPlugin = Some 0) "the plugin's initial state"
let s1 = withPlugin.apply (withPlugin.tr.insertText ("a", 1.0))
let s2 = s1.apply (s1.tr.setSelection (TextSelection.create (s1.doc, 1.0, 2.0)))
let s3 = s2.apply (s2.tr.insertText ("b", 2.0))
check (key.getState s1 = Some 1) "one change counted"
check (key.getState s2 = Some 1) "a selection change does not count"
check (key.getState s3 = Some 2) "two changes counted"
check (withPlugin.plugins.Count = 1) "plugins"

// Metadata on a transaction.
let tm = withPlugin.tr.setMeta (StringOrPluginOrPluginKey.String "why", box "test")
check (unbox<string> (tm.getMeta (StringOrPluginOrPluginKey.String "why")) = "test") "setMeta and getMeta"

// A view on the page's element, dispatching through our function.
let mutable dispatched = 0
let mutable view: EditorView = null
let props = createEmpty<DirectEditorProps>
props.state <- state
props.dispatchTransaction <-
    Some (fun (tr: Transaction) ->
        dispatched <- dispatched + 1
        view.updateState (view.state.apply tr))
view <- EditorView.Create (EditorViewNewPlace.Mount (EditorViewNewPlaceMount.Create (TestPage.element "stage")), props)
check (view.state.doc.eq doc) "the view shows the document"
check (not view.isDestroyed) "not destroyed"
view.dispatch (view.state.tr.insertText ("Q", 1.0))
check (dispatched = 1) "dispatchTransaction was called"
check (view.state.doc.child(0).textContent = "Qone") "the view's state was updated"
check (view.editable) "editable"
view.dispatch (view.state.tr.insertText ("R", 2.0))
check (dispatched = 2 && view.state.doc.child(0).textContent = "QRone") "dispatch again"
view.destroy ()
check view.isDestroyed "destroy"

printfn "%d checks passed" passed
