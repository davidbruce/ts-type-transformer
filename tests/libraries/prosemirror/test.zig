// ProseMirror from Zig, in a page: a schema, documents, transactions, selections, a plugin with state and
// an editor view on the page's #stage. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");
const pm = b.prosemirror_model;
const ps = b.prosemirror_state;
const pv = b.prosemirror_view;

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

fn str(s: []const u8) b.JsValue {
    return b.JsValue.string(s);
}

fn num(x: f64) b.JsValue {
    return b.JsValue.number(x);
}

// A schema's `nodes` and `marks` are dictionaries of specs; the bindings type them as JsValue, so a
// dictionary is built with an index-signature type as the holder and its handle passed on.
// FINDING 19: SchemaSpec.nodes and .marks (Record<string, NodeSpec> | OrderedMap) are plain JsValue, and the
// same goes for any JSON written by hand (Node.fromJSON takes a JsValue): there is no way to make an object
// or array literal, so NodeSpec/MarkSpec/Attrs `create()` objects with setAt stand in as dictionaries.
fn dict(comptime T: type, entries: []const struct { []const u8, b.JsValue }) b.JsValue {
    const d = T.create();
    for (entries) |e| d.setAt(e[0], e[1]);
    return .{ .h = d.h };
}

// toDOM for a node or mark: [tag, 0], the 0 being the hole for the content.
fn tagWithHole(name: []const u8) pm.DOMOutputSpec {
    const spec = b.JsArray.new();
    spec.push(b.JsValue.string(name));
    spec.push(b.JsValue.number(0));
    return .{ .array = spec };
}
fn docToDOM(_: ?*anyopaque, _: pm.Node) pm.DOMOutputSpec {
    return tagWithHole("div");
}
fn paragraphToDOM(_: ?*anyopaque, _: pm.Node) pm.DOMOutputSpec {
    return tagWithHole("p");
}
fn strongToDOM(_: ?*anyopaque, _: pm.Mark, _: bool) pm.DOMOutputSpec {
    return tagWithHole("strong");
}

fn makeSchema() pm.Schema {
    const doc = pm.NodeSpec.create();
    doc.setContent("block+");
    doc.setToDOM(b.NodeSpecToDOMFn.from(docToDOM, null));
    const paragraph = pm.NodeSpec.create();
    paragraph.setContent("inline*");
    paragraph.setGroup("block");
    paragraph.setToDOM(b.NodeSpecToDOMFn.from(paragraphToDOM, null));
    const text = pm.NodeSpec.create();
    text.setGroup("inline");
    const nodes = dict(pm.NodeSpec, &.{
        .{ "doc", .{ .h = doc.h } },
        .{ "paragraph", .{ .h = paragraph.h } },
        .{ "text", .{ .h = text.h } },
    });
    const strong = pm.MarkSpec.create();
    strong.setToDOM(b.MarkSpecToDOMFn.from(strongToDOM, null));
    const marks = dict(pm.MarkSpec, &.{.{ "strong", .{ .h = strong.h } }});
    const spec = pm.SchemaSpec.create();
    spec.setNodes(nodes);
    spec.setMarks(marks);
    return pm.Schema.new(spec);
}

fn paragraphOf(schema: pm.Schema, text: []const u8) pm.Node {
    const inner = schema.text(text);
    return schema.nodeWith(.{ .string = b.JsString.from("paragraph") }, .{ .content = .{ .Node = inner } });
}

fn docOf(schema: pm.Schema, paragraphs: []const pm.Node) pm.Node {
    return schema.nodeWith(.{ .string = b.JsString.from("doc") }, .{ .content = .{ .array = b.JsArrayOf(pm.Node).of(paragraphs) } });
}

// A plugin's state: how many transactions changed the document.
var inits: u32 = 0;
fn stateInit(_: ?*anyopaque, _: ps.EditorStateConfig, _: ps.EditorState) b.JsValue {
    inits += 1;
    return num(0);
}
fn stateApply(_: ?*anyopaque, tr: ps.Transaction, value: b.JsValue, _: ps.EditorState, _: ps.EditorState) b.JsValue {
    // What a callback returns is given to JavaScript: a lent handle (the old value) must be retained.
    return if (tr.asTransform().getDocChanged()) num(value.toNumber() + 1) else b.jsRetain(value);
}

var view: pv.EditorView = undefined;
var dispatched: u32 = 0;
fn onDispatch(_: ?*anyopaque, tr: ps.Transaction) void {
    dispatched += 1;
    view.updateState(view.getState().apply(tr));
}

export fn run() void {
    const schema = makeSchema();
    expect(schema.getTopNodeType().getName().eql("doc"), @src());
    expect(schema.getTopNodeType().getSpec().getContent().?.eql("block+"), @src());

    // Nodes: text, paragraphs and a document, with their sizes.
    const hello = schema.text("Hello");
    expect(hello.getIsText() and hello.getText().?.eql("Hello") and hello.getNodeSize() == 5, @src());
    const p1 = paragraphOf(schema, "Hello");
    const p2 = paragraphOf(schema, "world");
    const doc = docOf(schema, &.{ p1, p2 });
    expect(doc.getChildCount() == 2 and doc.child(0).getType().getName().eql("paragraph"), @src());
    expect(doc.getTextContent().eql("Helloworld"), @src());
    expect(doc.getNodeSize() == 2 + 7 + 7, @src());
    expect(doc.getContent().getSize() == 14, @src());
    expect(doc.textBetweenWith(0, doc.getContent().getSize(), .{ .blockSeparator = "|" }).eql("Hello|world"), @src());
    expect(doc.child(1).getIsTextblock() and doc.getIsBlock() and !doc.getIsText(), @src());
    expect(doc.nodeAt(1).?.getIsText() and doc.nodeAt(1).?.getText().?.eql("Hello"), @src());
    expect(doc.eq(docOf(schema, &.{ paragraphOf(schema, "Hello"), paragraphOf(schema, "world") })), @src());
    expect(!doc.eq(docOf(schema, &.{p1})), @src());
    doc.check();

    // Positions inside the document.
    const at = doc.resolve(3);
    expect(at.getParent().getType().getName().eql("paragraph") and at.getDepth() == 1, @src());
    expect(at.getParentOffset() == 2 and at.start(null) == 1 and at.end(null) == 6, @src());

    // Nodes from JSON, as the editor stores them.
    const text_json = dict(b.Attrs, &.{ .{ "type", str("text") }, .{ "text", str("From JSON") } });
    const items = b.JsArray.new();
    items.push(text_json);
    const para_json = dict(b.Attrs, &.{ .{ "type", str("paragraph") }, .{ "content", .{ .h = items.h } } });
    const parsed = pm.Node.fromJSON(schema, para_json);
    expect(parsed.getType().getName().eql("paragraph") and parsed.getTextContent().eql("From JSON"), @src());
    expect(parsed.toJSON().h != 0, @src());
    // A node's JSON round trips.
    expect(pm.Node.fromJSON(schema, doc.toJSON()).eq(doc), @src());

    // A document that breaks the schema is refused when checked: a doc cannot hold text directly.
    const bad = schema.nodeWith(.{ .string = b.JsString.from("doc") }, .{ .content = .{ .Node = hello } });
    expect(b.jsTakeError() != null, @src());
    expect(bad.h == 0, @src());

    // Marks: a strong mark on a text.
    const strong = schema.mark(.{ .string = b.JsString.from("strong") });
    expect(strong.getType().getName().eql("strong"), @src());
    const bold = schema.textWith("bold", b.JsArrayOf(pm.Mark).of(&.{strong}));
    expect(bold.getMarks().len() == 1 and strong.isInSet(bold.getMarks()), @src());
    expect(!strong.isInSet(hello.getMarks()), @src());
    expect(pm.Mark.sameSet(bold.getMarks(), b.JsArrayOf(pm.Mark).of(&.{strong})), @src());

    // An editor state with a plugin that counts changing transactions.
    const key = ps.PluginKey.new("counter");
    const field = ps.StateField.create();
    field.setInit(b.StateFieldInitFn.from(stateInit, null));
    field.setApply(b.StateFieldApplyFn.from(stateApply, null));
    const plugin_spec = ps.PluginSpec.create();
    plugin_spec.setKey(key);
    plugin_spec.setState(field);
    const counter = ps.Plugin.new(plugin_spec);
    const state_config = ps.EditorStateConfig.create();
    state_config.setDoc(doc);
    state_config.setPlugins(b.JsArrayOf(ps.Plugin).of(&.{counter}));
    const state0 = ps.EditorState.create(state_config);
    expect(state0.getDoc().eq(doc), @src());
    expect(b.jsSame(state0.getSchema(), schema), @src());
    expect(inits == 1 and key.getState(state0).toNumber() == 0, @src());
    expect(state0.getPlugins().len() == 1, @src());

    // The selection starts at the start of the document.
    const sel0 = state0.getSelection();
    expect(sel0.getFrom() == 1 and sel0.getTo() == 1 and sel0.getEmpty(), @src());

    // A transaction: type text at the cursor, and apply it.
    const tr1 = state0.getTr().insertText("Oh, ");
    expect(tr1.asTransform().getDocChanged(), @src());
    expect(tr1.asTransform().getDoc().getTextContent().eql("Oh, Helloworld"), @src());
    const state1 = state0.apply(tr1);
    expect(state1.getDoc().child(0).getTextContent().eql("Oh, Hello"), @src());
    expect(state0.getDoc().child(0).getTextContent().eql("Hello"), @src());
    expect(key.getState(state1).toNumber() == 1, @src());
    // The cursor moved after the inserted text.
    expect(state1.getSelection().getFrom() == 5, @src());

    // A selection range, then a mark added over it.
    const range = ps.TextSelection.createWith(state1.getDoc(), 1, .{ .head = 3 });
    expect(range.asSelection().getFrom() == 1 and range.asSelection().getTo() == 3 and !range.asSelection().getEmpty(), @src());
    const tr2 = state1.getTr().setSelection(range.asSelection());
    _ = tr2.asTransform().addMark(1, 3, strong);
    const state2 = state1.apply(tr2);
    expect(state2.getSelection().getFrom() == 1 and state2.getSelection().getTo() == 3, @src());
    expect(state2.getDoc().rangeHasMark(1, 3, .{ .Mark = strong }), @src());
    expect(!state2.getDoc().rangeHasMark(4, 6, .{ .Mark = strong }), @src());
    expect(key.getState(state2).toNumber() == 2, @src());

    // A transaction that changes nothing does not count: selection only.
    const state3 = state2.apply(state2.getTr().setSelection(ps.Selection.atEnd(state2.getDoc())));
    expect(key.getState(state3).toNumber() == 2, @src());
    expect(state3.getSelection().getFrom() == state3.getDoc().getContent().getSize() - 1, @src());

    // Deleting a range.
    const state4 = state3.apply(state3.getTr().delete(1, 5));
    expect(state4.getDoc().child(0).getTextContent().eql("Hello"), @src());
    expect(state4.getDoc().getChildCount() == 2, @src());
    expect(key.getState(state4).toNumber() == 3, @src());

    // Replacing the whole document with new content, in one step.
    const tr5 = state4.getTr().replaceWith(0, state4.getDoc().getContent().getSize(), .{ .Node = paragraphOf(schema, "only") });
    expect(tr5.asTransform().getDoc().getChildCount() == 1, @src());
    expect(tr5.asTransform().getSteps().len() == 1, @src());
    expect(tr5.asTransform().getDocs().len() == 1, @src());

    // An editor view on the page's #stage. Its dispatchTransaction applies each transaction.
    const props = pv.DirectEditorProps.create();
    props.setState(state0);
    props.setDispatchTransaction(b.DispatchTransactionFn.from(onDispatch, null));
    view = pv.EditorView.new(.{ .DomNode = .{ .h = b.test_page.element("stage").h } }, props);
    expect(view.getState().getDoc().eq(doc), @src());
    expect(view.getEditable() and !view.getComposing(), @src());

    // Updating the state by hand.
    view.updateState(state1);
    expect(view.getState().getDoc().child(0).getTextContent().eql("Oh, Hello"), @src());
    // The document is rendered in the page: the first paragraph's position has a DOM node.
    expect(view.nodeDOM(0) != null, @src());
    // Coordinates exist for a position in the text.
    expect(view.coordsAtPos(3).h != 0, @src());

    // The editor's own typing goes through dispatchTransaction: dispatch here as the view would.
    const typed = view.getState().getTr().insertTextWith("!", .{ .from = 14, .to = 14 });
    props.setState(view.getState());
    view.update(props);
    view.dispatch(typed);
    expect(dispatched == 1, @src());
    expect(view.getState().getDoc().child(1).getTextContent().eql("wo!rld"), @src());

    // Destroying the view detaches it from the page.
    view.destroy();
}
