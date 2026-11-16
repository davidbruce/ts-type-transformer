// ProseMirror from C++, in a page: a schema defined here, documents built and
// read back, an editor state with transactions, marks, selections and a plugin
// with state, then an EditorView on the page's element. Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

using Sch = Schema<ts::Value, ts::Value>;

// A generated object where a ts::Value is wanted (an object given as data).
static ts::Value val_of(const ts::Handle& h) {
    ts::Value v;
    static_cast<ts::Handle&>(v) = h;
    return v;
}
static ts::Value str(std::string_view s) { return ts::Value::string(s); }

// FINDING 22: Node.fromJSON, SchemaSpec.nodes, marks... take `ts::Value`/`object`
// and there is no way to make a plain JavaScript object or array of values;
// `Attrs` (index signature, `{}` with set_at) stands in.
static Attrs object() { return Attrs::create(); }

static Sch make_schema() {
    // doc: paragraph+ ; paragraph: text* ; text; and a strong mark.
    NodeSpec doc = NodeSpec::create();
    doc.content("paragraph+");
    NodeSpec paragraph = NodeSpec::create();
    paragraph.content("inline*");
    paragraph.group("block");
    paragraph.toDOM(Node_to_DOMOutputSpec_fn([](Node) {
        ts::Array<> spec = ts::Array<>::create();
        spec.push(str("p"));
        spec.push(ts::Value::number(0));
        return DOMOutputSpec(spec);
    }));
    NodeSpec text = NodeSpec::create();
    text.group("inline");
    NodeSpec nodes = NodeSpec::create();
    nodes.set_at("doc", val_of(doc));
    nodes.set_at("paragraph", val_of(paragraph));
    nodes.set_at("text", val_of(text));

    MarkSpec strong = MarkSpec::create();
    strong.toDOM(Mark_boolean_to_DOMOutputSpec_fn([](Mark, bool) {
        ts::Array<> spec = ts::Array<>::create();
        spec.push(str("strong"));
        spec.push(ts::Value::number(0));
        return DOMOutputSpec(spec);
    }));
    MarkSpec marks = MarkSpec::create();
    marks.set_at("strong", val_of(strong));

    SchemaSpec<ts::Value, ts::Value> spec = SchemaSpec<ts::Value, ts::Value>::create();
    spec.nodes(val_of(nodes));
    spec.marks(val_of(marks));
    return Sch::create(spec);
}

static Node para(const Sch& schema, std::string_view text) {
    return schema.node(string_or_NodeType(std::string("paragraph")), std::nullopt, Fragment_or_Node_or_Node_array(schema.text(text)));
}

extern "C" __attribute__((export_name("run"))) void run() {
    Sch schema = make_schema();
    CHECK(schema.topNodeType().name() == "doc");
    CHECK(schema.nodes().h != 0);
    CHECK(!schema.marks().h == false);

    // ---- model: nodes and sizes ----
    Node hello = schema.text("Hello");
    CHECK(hello.isText() && hello.text() == "Hello");
    CHECK(hello.nodeSize() == 5);
    Node p1 = para(schema, "Hello");
    Node p2 = para(schema, "World!");
    CHECK(p1.nodeSize() == 7);  // text + the two sides of the paragraph
    CHECK(p1.childCount() == 1 && p1.textContent() == "Hello");
    CHECK(p1.isBlock() && p1.isTextblock() && !p1.isInline());
    CHECK(p1.type().name() == "paragraph");

    ts::Array<Node> two = ts::Array<Node>::create();
    two.push(p1);
    two.push(p2);
    Node doc = schema.node(string_or_NodeType(std::string("doc")), std::nullopt, Fragment_or_Node_or_Node_array(two));
    CHECK(doc.childCount() == 2);
    CHECK(doc.nodeSize() == 7 + 8 + 2);
    CHECK(doc.content().size() == 15);
    CHECK(doc.textContent() == "HelloWorld!");
    CHECK(doc.textBetween(0, doc.content().size(), std::string_view("|")) == "Hello|World!");
    CHECK(doc.child(1).textContent() == "World!");
    CHECK(doc.firstChild()->eq(p1));
    CHECK(doc.lastChild()->eq(p2));
    CHECK(!p1.eq(p2));
    CHECK(doc.nodeAt(1)->text() == "Hello");
    CHECK(!doc.nodeAt(15).has_value());
    CHECK(doc.toString().find("paragraph") != std::string::npos);
    doc.check();
    CHECK(!ts::take_error());

    // Positions: resolve pos 3 (inside "Hello", after "He").
    ResolvedPos at = doc.resolve(3);
    CHECK(at.pos() == 3 && at.depth() == 1 && at.parentOffset() == 2);
    CHECK(at.parent().type().name() == "paragraph");
    CHECK(at.start() == 1 && at.end() == 6);
    CHECK(doc.resolve(0).depth() == 0);

    // A slice and a cut of the document.
    Node cut = doc.cut(0, 7);
    CHECK(cut.childCount() == 1 && cut.textContent() == "Hello");
    Slice slice = doc.slice(2, 5);
    CHECK(slice.content().textBetween(0, slice.content().size()) == "ell");

    // Marks: strong on part of the text.
    Mark strong = schema.mark(string_or_MarkType(std::string("strong")));
    CHECK(strong.type().name() == "strong");
    Node bold = schema.text("bold", ts::Array<Mark>{strong});
    CHECK(bold.marks().size() == 1);
    CHECK(bold.marks()[0].eq(strong));
    CHECK(strong.isInSet(bold.marks()));
    CHECK(!hello.marks().size());

    // From JSON and back.
    Attrs t = object();
    t.set_at("type", str("text"));
    t.set_at("text", str("from json"));
    Attrs jp = object();
    jp.set_at("type", str("paragraph"));
    ts::Array<> jcontent = ts::Array<>::create();
    jcontent.push(val_of(t));
    jp.set_at("content", val_of(jcontent));
    Attrs jd = object();
    jd.set_at("type", str("doc"));
    ts::Array<> jdocc = ts::Array<>::create();
    jdocc.push(val_of(jp));
    jd.set_at("content", val_of(jdocc));
    Node parsed = Node::fromJSON(schema, val_of(jd));
    CHECK(parsed.textContent() == "from json");
    CHECK(parsed.childCount() == 1);
    CHECK(parsed.type().name() == "doc");

    // ---- state ----
    EditorStateConfig config = EditorStateConfig::create();
    config.schema(schema);
    config.doc(doc);
    EditorState state = EditorState::create(config);
    CHECK(state.doc().eq(doc));
    CHECK(state.selection().from() == 1 && state.selection().to() == 1);  // at the start
    CHECK(state.selection().empty());
    CHECK(state.schema().topNodeType().name() == "doc");

    // A transaction inserts text at a place; applying makes a new state.
    Transaction tr = state.tr();
    tr.insertText("Oh, ", 1);
    CHECK(tr.docChanged());
    CHECK(tr.doc().textContent() == "Oh, HelloWorld!");
    CHECK(state.doc().textContent() == "HelloWorld!");  // the old state is as it was
    EditorState next = state.apply(tr);
    CHECK(next.doc().textContent() == "Oh, HelloWorld!");
    CHECK(next.doc().child(0).textContent() == "Oh, Hello");
    CHECK(tr.steps().size() == 1);
    CHECK(tr.mapping().h != 0);

    // Selections: a text selection from 2 to 6.
    TextSelection sel = TextSelection::create(next.doc(), 2, 6);
    CHECK(sel.anchor() == 2 && sel.head() == 6 && sel.from() == 2 && sel.to() == 6);
    CHECK(!sel.empty());
    Transaction pick = next.tr();
    pick.setSelection(sel);
    CHECK(pick.selectionSet());
    EditorState picked = next.apply(pick);
    CHECK(picked.selection().from() == 2 && picked.selection().to() == 6);
    CHECK(picked.doc().textBetween(picked.selection().from(), picked.selection().to()) == "h, H");

    // Replacing the selection with typed text.
    Transaction typed = picked.tr();
    typed.insertText("X");
    EditorState after = picked.apply(typed);
    CHECK(after.doc().textContent() == "OXelloWorld!");
    CHECK(after.selection().empty() && after.selection().from() == 3);

    // Marks: make part of the first paragraph strong.
    Transaction mark = after.tr();
    mark.addMark(1, 4, strong);
    EditorState marked = after.apply(mark);
    CHECK(marked.doc().child(0).child(0).marks().size() == 1);
    CHECK(marked.doc().child(0).child(0).text() == "OXe");
    CHECK(marked.doc().child(0).child(1).marks().size() == 0);
    CHECK(marked.doc().rangeHasMark(1, 4, Mark_or_MarkType(strong)));
    Transaction unmark = marked.tr();
    unmark.removeMark(1, 4, Mark_or_MarkType(strong));
    CHECK(!marked.apply(unmark).doc().rangeHasMark(1, 4, Mark_or_MarkType(strong)));

    // Deleting a range, and the state's JSON.
    Transaction del = state.tr();
    del.delete_(1, 6);
    CHECK(state.apply(del).doc().child(0).childCount() == 0);
    CHECK(state.toJSON().h != 0);

    // ---- a plugin with state: counts the transactions that changed the doc ----
    int applied = 0;
    StateField<ts::Value> field = StateField<ts::Value>::create();
    field.init(StateField_init<ts::Value>([](EditorStateConfig, EditorState) { return ts::Value::number(0); }));
    field.apply(StateField_apply<ts::Value>([&](Transaction t, ts::Value old, EditorState, EditorState) {
        applied++;
        return ts::Value::number(old.to_number() + (t.docChanged() ? 1 : 0));
    }));
    PluginSpec<ts::Value> plugin_spec = PluginSpec<ts::Value>::create();
    plugin_spec.state(field);
    Plugin<ts::Value> counter = Plugin<ts::Value>::create(plugin_spec);
    EditorStateConfig with_plugin = EditorStateConfig::create();
    with_plugin.schema(schema);
    with_plugin.doc(doc);
    with_plugin.plugins(ts::Array<Plugin<ts::Value>>{counter});
    EditorState counted = EditorState::create(with_plugin);
    CHECK(counted.plugins().size() == 1);
    CHECK(counter.getState(counted)->to_number() == 0);
    Transaction one = counted.tr();
    one.insertText("a", 1);
    counted = counted.apply(one);
    CHECK(counter.getState(counted)->to_number() == 1);
    Transaction noop = counted.tr();
    counted = counted.apply(noop);
    CHECK(counter.getState(counted)->to_number() == 1);
    Transaction two_edits = counted.tr();
    two_edits.insertText("b", 1);
    counted = counted.apply(two_edits);
    CHECK(counter.getState(counted)->to_number() == 2);
    CHECK(applied == 3);
    CHECK(counted.doc().textContent() == "baHelloWorld!");

    // ---- a view on the page's #stage, dispatching to a function here ----
    int dispatched = 0;
    EditorState current = state;
    EditorView* view_ref = nullptr;
    DirectEditorProps props = DirectEditorProps::create();
    props.state(state);
    props.dispatchTransaction(Transaction_fn([&](Transaction t) {
        dispatched++;
        current = current.apply(t);
        if (view_ref) view_ref->updateState(current);
    }));
    HTMLElement stage = test_page::element("stage");
    // FINDING 23: HTMLElement is an empty struct in this package set (lib.dom is
    // not bound here), so the rendered DOM cannot be read from C++; and the
    // place the view mounts in, `dist_DOMNode` (also empty), takes no conversion
    // from an HTMLElement: the handle is rewrapped.
    dist_DOMNode place;
    static_cast<ts::Handle&>(place) = stage;
    EditorView view = EditorView::create(EditorView_new_place_or_dist_DOMNode(place), props);
    view_ref = &view;
    CHECK(!view.isDestroyed());
    CHECK(view.state().doc().eq(doc));
    CHECK(view.dom().h != 0);
    CHECK(view.editable());
    CHECK(!view.composing());

    // FINDING 2 again: `view.dispatch` is a bare Callback (Transaction_fn) that
    // cannot be called. pasteText makes the view dispatch a transaction itself,
    // which reaches dispatchTransaction.
    CHECK(dispatched == 0);
    view.pasteText("pasted");
    CHECK(dispatched >= 1);
    CHECK(current.doc().textContent().find("pasted") != std::string::npos);
    CHECK(view.state().doc().textContent().find("pasted") != std::string::npos);
    view_ref = nullptr;
    view.destroy();
    CHECK(view.isDestroyed());
}
