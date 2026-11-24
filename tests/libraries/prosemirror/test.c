// ProseMirror from C, in a page: a schema defined here, documents from JSON,
// editor state, transactions (text and marks), selections, a plugin with
// state, and a view on the page's element. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool str_is(JsString s, const char* expected) {
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

// ---- building plain JavaScript objects and arrays ----
// An Attrs is a string-keyed object: all of these use it as one.
static JsValue obj(Attrs a) { return (JsValue){ a.h }; }
static JsValue str(const char* s) { return js_string(s); }

// A schema's `toDOM` for a paragraph and a strong mark: ["p", 0] and ["strong", 0].
static JsValue dom_hole(const char* tag) {
    JsArray spec = JsArray_new();
    JsArray_push(spec, str(tag));
    JsArray_push(spec, js_number(0));
    return (JsValue){ spec.h };
}
static JsValue paragraph_dom(void* ctx, Node node) { return dom_hole("p"); }
static JsValue strong_dom(void* ctx, Mark mark, bool inline_) { return dom_hole("strong"); }

static Schema make_schema(void) {
    NodeSpec doc = NodeSpec_create();
    NodeSpec_set_content(doc, "paragraph+");
    NodeSpec paragraph = NodeSpec_create();
    NodeSpec_set_content(paragraph, "text*");
    NodeSpec_set_group(paragraph, "block");
    NodeSpec_set_toDOM(paragraph, Node_to_DOMOutputSpec_fn_from(paragraph_dom, 0));
    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");
    SchemaSpec_nodes nodes = SchemaSpec_nodes_create();
    SchemaSpec_nodes_set_at(nodes, "doc", doc);
    SchemaSpec_nodes_set_at(nodes, "paragraph", paragraph);
    SchemaSpec_nodes_set_at(nodes, "text", text);
    MarkSpec strong = MarkSpec_create();
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(strong_dom, 0));
    SchemaSpec_marks marks = SchemaSpec_marks_create();
    SchemaSpec_marks_set_at(marks, "strong", strong);
    SchemaSpec spec = SchemaSpec_create();
    SchemaSpec_set_nodes(spec, nodes);
    SchemaSpec_set_marks(spec, marks);
    return Schema_new(spec);
}

// A document of one paragraph of `text`, as JSON.
static JsValue doc_json(const char* text) {
    Attrs leaf = Attrs_create();
    Attrs_set_at(leaf, "type", str("text"));
    Attrs_set_at(leaf, "text", str(text));
    JsArray leaves = JsArray_new();
    JsArray_push(leaves, obj(leaf));
    Attrs paragraph = Attrs_create();
    Attrs_set_at(paragraph, "type", str("paragraph"));
    Attrs_set_at(paragraph, "content", (JsValue){ leaves.h });
    JsArray blocks = JsArray_new();
    JsArray_push(blocks, obj(paragraph));
    Attrs doc = Attrs_create();
    Attrs_set_at(doc, "type", str("doc"));
    Attrs_set_at(doc, "content", (JsValue){ blocks.h });
    return obj(doc);
}

// ---- a plugin that counts the transactions that changed the document ----
static JsValue counter_init(void* ctx, EditorStateConfig config, EditorState instance) { return js_number(0); }
static JsValue counter_apply(void* ctx, Transaction tr, JsValue value, EditorState old_state, EditorState new_state) {
    return js_number(js_to_number(value) + (Transform_get_docChanged(tr) ? 1 : 0));
}

// ---- the view's dispatch: apply the transaction to the view's state ----
static EditorView view;
static int dispatched;
static void on_dispatch(void* ctx, Transaction tr) {
    dispatched++;
    EditorView_updateState(view, EditorState_apply(EditorView_get_state(view), tr));
}

static JsString text_of(Node n) { return Node_get_textContent(n); }

__attribute__((export_name("run"))) void run(void) {
    Schema schema = make_schema();

    // ---- schema ----
    NodeType paragraph_type = Schema_nodes_get_at(Schema_get_nodes(schema), "paragraph");
    CHECK(str_is(NodeType_get_name(paragraph_type), "paragraph"));
    CHECK(NodeType_get_isBlock(paragraph_type) && NodeType_get_isTextblock(paragraph_type));
    NodeType text_type = Schema_nodes_get_at(Schema_get_nodes(schema), "text");
    CHECK(NodeType_get_isText(text_type) && NodeType_get_isInline(text_type));
    MarkType strong_type = Schema_marks_get_at(Schema_get_marks(schema), "strong");
    CHECK(str_is(MarkType_get_name(strong_type), "strong"));
    CHECK(str_is(NodeType_get_name(Schema_get_topNodeType(schema)), "doc"));

    // ---- nodes ----
    Node doc = Node_fromJSON(schema, doc_json("Hello world"));
    CHECK(str_is(text_of(doc), "Hello world"));
    CHECK(Node_get_childCount(doc) == 1);
    CHECK(Node_get_nodeSize(doc) == 13 + 2);
    Node para = Node_child(doc, 0);
    CHECK(Node_get_nodeSize(para) == 13 && Node_get_isTextblock(para));
    CHECK(Node_get_childCount(para) == 1 && Node_get_isText(Node_child(para, 0)));
    CHECK(str_is(Node_get_text(Node_child(para, 0)), "Hello world"));
    CHECK(str_is(NodeType_get_name(Node_get_type(para)), "paragraph"));
    // Nodes made by the schema are equal to the ones from JSON.
    Node made = Schema_node(schema, "paragraph", (Attrs){ 0 }, (JsArray_Node){ 0 }, (JsArray_Mark){ 0 });
    CHECK(Node_get_childCount(made) == 0 && Node_get_nodeSize(made) == 2);
    JsArray_Node one_text = JsArray_Node_new();
    JsArray_Node_push(one_text, Schema_text(schema, "Hello world", (JsArray_Mark){ 0 }));
    Node same_para = Schema_node(schema, "paragraph", (Attrs){ 0 }, one_text, (JsArray_Mark){ 0 });
    CHECK(Node_eq(same_para, para));
    CHECK(!Node_eq(made, para));
    // Text between positions, and cutting.
    CHECK(str_is(Node_textBetween(doc, 1, 6, (const char*)0, (const char*)0), "Hello"));
    CHECK(str_is(text_of(Node_cut(doc, 1, js_some_double(6))), "Hello"));
    // An invalid node is rejected by the schema: a doc needs a paragraph.
    Node invalid_doc = Schema_node(schema, "doc", (Attrs){ 0 }, (JsArray_Node){ 0 }, (JsArray_Mark){ 0 });
    CHECK(invalid_doc.h == 0 && js_failed());
    JsValue invalid = js_take_error();
    JsString why = js_error_message(invalid);
    CHECK(str_is(why, "Invalid content for node doc: <>"));
    js_release(invalid);

    // ---- state and transactions ----
    EditorStateConfig config = EditorStateConfig_create();
    EditorStateConfig_set_doc(config, doc);
    EditorStateConfig_set_schema(config, schema);
    // The plugin: its state starts at 0.
    PluginKey key = PluginKey_new("counter");
    StateField field = StateField_create();
    StateField_set_init(field, StateField_init_cb_from(counter_init, 0));
    StateField_set_apply(field, StateField_apply_cb_from(counter_apply, 0));
    PluginSpec plugin_spec = PluginSpec_create();
    PluginSpec_set_state(plugin_spec, field);
    PluginSpec_set_key(plugin_spec, key);
    Plugin counter = Plugin_new(plugin_spec);
    JsArray_Plugin plugins = JsArray_Plugin_new();
    JsArray_Plugin_push(plugins, counter);
    EditorStateConfig_set_plugins(config, plugins);
    EditorState state = EditorState_create(config);
    CHECK(str_is(text_of(EditorState_get_doc(state)), "Hello world"));
    CHECK(js_same(EditorState_get_schema(state), schema));
    CHECK(JsArray_Plugin_length(EditorState_get_plugins(state)) == 1);
    CHECK(js_to_number(Plugin_getState(counter, state)) == 0);
    // The selection starts at the start of the text.
    CHECK(Selection_get_empty(EditorState_get_selection(state)));
    CHECK(Selection_get_from(EditorState_get_selection(state)) == 1);

    // insertText at a position.
    Transaction tr = EditorState_get_tr(state);
    Transaction_insertText(tr, "Oh, ", js_some_double(1), js_some_double(1));
    CHECK(Transform_get_docChanged(tr));
    EditorState inserted = EditorState_apply(state, tr);
    CHECK(str_is(text_of(EditorState_get_doc(inserted)), "Oh, Hello world"));
    // The old state is unchanged.
    CHECK(str_is(text_of(EditorState_get_doc(state)), "Hello world"));
    CHECK(js_to_number(Plugin_getState(counter, inserted)) == 1);
    CHECK(js_to_number(PluginKey_getState(key, inserted)) == 1);
    CHECK(js_to_number(Plugin_getState(counter, state)) == 0);

    // A transaction that does not change the document does not count.
    Transaction idle = EditorState_get_tr(inserted);
    Transaction_setMeta(idle, "note", js_number(1));
    EditorState idled = EditorState_apply(inserted, idle);
    CHECK(js_to_number(Plugin_getState(counter, idled)) == 1);
    CHECK(js_to_number(Transaction_getMeta(idle, "note")) == 1);

    // Deleting, and replacing.
    Transaction del = EditorState_get_tr(inserted);
    Transform_delete(del, 1, 5);
    CHECK(str_is(text_of(Transform_get_doc(del)), "Hello world"));
    Transaction swap = EditorState_get_tr(state);
    Transform_replaceWith(swap, 7, 12, Schema_text(schema, "there", (JsArray_Mark){ 0 }));
    CHECK(str_is(text_of(Transform_get_doc(swap)), "Hello there"));
    CHECK(JsArray_Step_length(Transform_get_steps(swap)) == 1);

    // ---- marks ----
    Mark bold = Schema_mark(schema, "strong", (Attrs){ 0 });
    CHECK(str_is(MarkType_get_name(Mark_get_type(bold)), "strong"));
    Transaction marking = EditorState_get_tr(state);
    Transform_addMark(marking, 1, 6, bold);
    Node marked = Transform_get_doc(marking);
    CHECK(Node_rangeHasMark(marked, 1, 6, bold));
    CHECK(!Node_rangeHasMark(marked, 7, 12, bold));
    // The paragraph is now two text nodes: "Hello" (bold) and " world".
    Node marked_para = Node_child(marked, 0);
    CHECK(Node_get_childCount(marked_para) == 2);
    Node first = Node_child(marked_para, 0);
    CHECK(str_is(Node_get_text(first), "Hello"));
    CHECK(JsArray_Mark_length(Node_get_marks(first)) == 1);
    CHECK(Mark_eq(JsArray_Mark_get(Node_get_marks(first), 0), bold));
    CHECK(JsArray_Mark_length(Node_get_marks(Node_child(marked_para, 1))) == 0);
    // Taking the mark off again gives the first document back.
    Transform_removeMark(marking, 1, 6, bold);
    CHECK(Node_eq(Transform_get_doc(marking), doc));
    // Stored marks: typing next goes in bold.
    Transaction storing = EditorState_get_tr(state);
    Transaction_addStoredMark(storing, bold);
    CHECK(Transaction_get_storedMarksSet(storing));
    EditorState with_stored = EditorState_apply(state, storing);
    CHECK(JsArray_Mark_length(EditorState_get_storedMarks(with_stored)) == 1);
    Transaction typing = EditorState_get_tr(with_stored);
    Transaction_insertText(typing, "!", js_some_double(12), js_some_double(12));
    Node typed = Transform_get_doc(typing);
    Node last = Node_child(Node_child(typed, 0), Node_get_childCount(Node_child(typed, 0)) - 1);
    CHECK(str_is(Node_get_text(last), "!") && JsArray_Mark_length(Node_get_marks(last)) == 1);

    // ---- selections ----
    TextSelection range = TextSelection_create(doc, 2, js_some_double(7));
    CHECK(Selection_get_anchor(range) == 2 && Selection_get_head(range) == 7);
    CHECK(Selection_get_from(range) == 2 && Selection_get_to(range) == 7 && !Selection_get_empty(range));
    Transaction selecting = EditorState_get_tr(state);
    Transaction_setSelection(selecting, range);
    CHECK(Transaction_get_selectionSet(selecting));
    EditorState selected = EditorState_apply(state, selecting);
    CHECK(Selection_get_from(EditorState_get_selection(selected)) == 2);
    CHECK(Selection_get_to(EditorState_get_selection(selected)) == 7);
    // insertText with no range replaces the selection.
    Transaction replacing = EditorState_get_tr(selected);
    Transaction_insertText(replacing, "J", js_none_double(), js_none_double());
    CHECK(str_is(text_of(Transform_get_doc(replacing)), "HJworld"));
    Selection at_end = Selection_atEnd(doc);
    CHECK(Selection_get_from(at_end) == 12);
    CHECK(Selection_get_from(Selection_atStart(doc)) == 1);
    ResolvedPos resolved = Node_resolve(doc, 4);
    CHECK(ResolvedPos_get_parentOffset(resolved) == 3 && ResolvedPos_get_depth(resolved) == 1);

    // ---- the view ----
    EditorStateConfig view_config = EditorStateConfig_create();
    EditorStateConfig_set_doc(view_config, doc);
    EditorStateConfig_set_schema(view_config, schema);
    EditorState view_state = EditorState_create(view_config);
    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, view_state);
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, 0));
    view = EditorView_new((Node_2){ element("stage").h }, props);
    CHECK(view.h != 0 && !EditorView_get_isDestroyed(view));
    CHECK(EditorView_get_editable(view));
    CHECK(EditorView_get_dom(view).h != 0);
    CHECK(str_is(text_of(EditorState_get_doc(EditorView_get_state(view))), "Hello world"));
    // The document is on the page: a position has coordinates, and they map back.
    EditorView_coordsAtPos_result start = EditorView_coordsAtPos(view, 1, js_none_double());
    CHECK(EditorView_coordsAtPos_result_get_bottom(start) > EditorView_coordsAtPos_result_get_top(start));
    EditorView_coordsAtPos_result later = EditorView_coordsAtPos(view, 6, js_none_double());
    CHECK(EditorView_coordsAtPos_result_get_left(later) > EditorView_coordsAtPos_result_get_left(start));
    // Dispatching: the view's own dispatch goes through ours, and the view updates.
    Transaction edit = EditorState_get_tr(EditorView_get_state(view));
    Transaction_insertText(edit, "Say: ", js_some_double(1), js_some_double(1));
    EditorView_dispatch(view, edit);
    CHECK(dispatched == 1);
    CHECK(str_is(text_of(EditorState_get_doc(EditorView_get_state(view))), "Say: Hello world"));

    // Destroying the view.
    EditorView_destroy(view);
    CHECK(EditorView_get_isDestroyed(view));
}
