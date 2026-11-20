#include <stdio.h>
#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

static JsValue dom_hole(const char* tag) {
    JsArray a = JsArray_new();
    js_push_str(a, tag);
    js_push_num(a, 0);
    return js_value(a);
}
static JsValue heading_dom(void* ctx, Node_3 n) {
    (void)ctx;
    js_scope_open();
    int lvl = (int)js_to_number(js_get_prop(Node_3_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    js_scope_close();
    return res;
}
static JsValue tag_dom(void* ctx, Node_3 n) { (void)n; return dom_hole((const char*)ctx); }
static JsValue code_block_dom(void* ctx, Node_3 n) {
    (void)ctx; (void)n;
    js_scope_open();
    JsArray inner = JsArray_new();
    js_push_str(inner, "code");
    js_push_num(inner, 0);
    JsArray outer = JsArray_new();
    js_push_str(outer, "pre");
    js_push_val(outer, inner);
    js_retain(outer);
    js_scope_close();
    return js_value(outer);
}
static JsValue hr_dom(void* ctx, Node_3 n) {
    (void)ctx; (void)n;
    JsArray a = JsArray_new();
    js_push_str(a, "hr");
    return js_value(a);
}
static JsValue mark_dom(void* ctx, Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Schema g_schema;
static EditorView g_view;
static int g_dispatches = 0;
static const char* g_last_action = "Initialized schema with 7 node types & 3 marks";

static Schema make_schema(void) {
    NodeSpec doc = NodeSpec_create();
    NodeSpec_set_content(doc, "block+");

    AttributeSpec lvl = AttributeSpec_create();
    AttributeSpec_set_default(lvl, js_number(2));
    MarkSpec_attrs hattrs = MarkSpec_attrs_create();
    MarkSpec_attrs_set_at(hattrs, "level", lvl);

    NodeSpec heading = NodeSpec_create();
    NodeSpec_set_attrs(heading, hattrs);
    NodeSpec_set_content(heading, "inline*");
    NodeSpec_set_group(heading, "block");
    NodeSpec_set_defining(heading, true);
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"p"));

    NodeSpec quote = NodeSpec_create();
    NodeSpec_set_content(quote, "block+");
    NodeSpec_set_group(quote, "block");
    NodeSpec_set_defining(quote, true);
    NodeSpec_set_toDOM(quote, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"blockquote"));

    NodeSpec cblock = NodeSpec_create();
    NodeSpec_set_content(cblock, "text*");
    NodeSpec_set_marks(cblock, "");
    NodeSpec_set_group(cblock, "block");
    NodeSpec_set_code(cblock, true);
    NodeSpec_set_defining(cblock, true);
    NodeSpec_set_toDOM(cblock, Node_to_DOMOutputSpec_fn_from(code_block_dom, NULL));

    NodeSpec hr = NodeSpec_create();
    NodeSpec_set_group(hr, "block");
    NodeSpec_set_atom(hr, true);
    NodeSpec_set_toDOM(hr, Node_to_DOMOutputSpec_fn_from(hr_dom, NULL));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "blockquote", quote);
    js_set_val(nodes, "code_block", cblock);
    js_set_val(nodes, "horizontal_rule", hr);
    js_set_val(nodes, "text", text);

    MarkSpec strong = MarkSpec_create();
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    MarkSpec em = MarkSpec_create();
    MarkSpec_set_toDOM(em, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"em"));
    MarkSpec code = MarkSpec_create();
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));

    Attrs marks = Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "em", em);
    js_set_val(marks, "code", code);

    SchemaSpec spec = SchemaSpec_create();
    js_set_val(spec, "nodes", nodes);
    js_set_val(spec, "marks", marks);
    return Schema_new(spec);
}

static Node_3 t_node(const char* t) {
    return Schema_text(g_schema, t, (JsArray_Mark){ 0 });
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Node_3 h = Schema_node_with_string(g_schema, "heading", h_attrs, t_node("ProseMirror Document Schema"));

    Mark m_strong = Schema_mark(g_schema, "strong", (Attrs){ 0 });
    Mark m_em = Schema_mark(g_schema, "em", (Attrs){ 0 });
    Mark m_code = Schema_mark(g_schema, "code", (Attrs){ 0 });

    Node_3 p1_items[7] = {
        t_node("Inline marks compose cleanly: "),
        Schema_text(g_schema, "bold", JsArray_Mark_from(&m_strong, 1)),
        t_node(", "),
        Schema_text(g_schema, "italic", JsArray_Mark_from(&m_em, 1)),
        t_node(", and "),
        Schema_text(g_schema, "inline_code", JsArray_Mark_from(&m_code, 1)),
        t_node(" spans.")
    };
    Node_3 p_marks = Schema_node_with_string_and_Node_array(
        g_schema, "paragraph", (Attrs){ 0 }, JsArray_Node_3_from(p1_items, 7)
    );

    Node_3 q_inner = Schema_node_with_string(
        g_schema, "paragraph", (Attrs){ 0 },
        t_node("Every document conforms to a strict schema enforced on every transaction.")
    );
    Node_3 quote = Schema_node_with_string(g_schema, "blockquote", (Attrs){ 0 }, q_inner);

    Node_3 code_block = Schema_node_with_string(
        g_schema, "code_block", (Attrs){ 0 },
        t_node("schema.node(\"paragraph\", null, [schema.text(\"typed\")])")
    );

    Node_3 outro = Schema_node_with_string(
        g_schema, "paragraph", (Attrs){ 0 },
        t_node("Select any text in this editor and click a toolbar action to toggle marks or switch block types.")
    );

    Node_3 blocks[5] = { h, p_marks, quote, code_block, outro };
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, JsArray_Node_3_from(blocks, 5));
}

static EditorState make_state(void) {
    EditorStateConfig cfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(cfg, g_schema);
    EditorStateConfig_set_doc(cfg, make_initial_doc());
    return EditorState_create(cfg);
}

static void target_range(EditorState st, double* out_from, double* out_to) {
    Selection_2 sel = EditorState_get_selection(st);
    if (!Selection_2_get_empty(sel)) {
        *out_from = Selection_2_get_from(sel);
        *out_to = Selection_2_get_to(sel);
        return;
    }
    Node_3 doc = EditorState_get_doc(st);
    if (Node_3_get_childCount(doc) >= 2.0) {
        Node_3 c0 = Node_3_child(doc, 0);
        Node_3 c1 = Node_3_child(doc, 1);
        double start_pos = Node_3_get_nodeSize(c0) + 1.0;
        double len = Fragment_get_size(Node_3_get_content(c1));
        if (len > 0.0) {
            *out_from = start_pos;
            *out_to = start_pos + (len > 12.0 ? 12.0 : len);
            return;
        }
    }
    double sz = Fragment_get_size(Node_3_get_content(doc));
    *out_from = 1.0;
    *out_to = sz > 2.0 ? 2.0 : sz;
}

static void update_meta(EditorState st) {
    js_scope_open();
    double f = 0, t = 0;
    target_range(st, &f, &t);
    Node_3 doc = EditorState_get_doc(st);
    Schema_marks marks = Schema_get_marks(g_schema);
    bool has_b = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "strong"));
    bool has_e = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "em"));
    bool has_c = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "code"));

    HTMLElement_set_className(Document_getElementById(g_doc, "btn-0"), has_b ? "active" : "");
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-1"), has_e ? "active" : "");
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-2"), has_c ? "active" : "");

    Selection_2 sel = EditorState_get_selection(st);
    JsString bname = NodeType_get_name(Node_3_get_type(ResolvedPos_get_parent(Selection_2_get__from(sel))));
    char block_str[64];
    uint32_t blen = JsString_copy(bname, block_str, sizeof block_str - 1);
    block_str[blen < sizeof block_str ? blen : sizeof block_str - 1] = 0;

    char meta[256];
    snprintf(
        meta, sizeof meta,
        "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 range=[%d..%d] \xC2\xB7 block=%s \xC2\xB7 dispatches=%d",
        g_last_action, (int)Node_3_get_childCount(doc), (int)Node_3_get_nodeSize(doc),
        (int)f, (int)t, block_str, g_dispatches
    );
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    g_dispatches++;
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    update_meta(next);
    js_scope_close();
}

static void toggle_mark(const char* name, const char* added_msg, const char* removed_msg) {
    js_scope_open();
    EditorState st = EditorView_get_state(g_view);
    double f = 0, t = 0;
    target_range(st, &f, &t);
    if (t > f) {
        MarkType mt = Schema_marks_get_at(Schema_get_marks(g_schema), name);
        Transaction tr = EditorState_get_tr(st);
        if (Node_3_rangeHasMark_with_MarkType(EditorState_get_doc(st), f, t, mt)) {
            Transaction_removeMark_with_opt_MarkType(tr, f, t, mt);
            g_last_action = removed_msg;
        } else {
            Mark m = Schema_mark(g_schema, mt, (Attrs){ 0 });
            Transaction_addMark(tr, f, t, m);
            g_last_action = added_msg;
        }
        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    }
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    if (idx == 0) toggle_mark("strong", "Added bold mark", "Removed bold mark");
    else if (idx == 1) toggle_mark("em", "Added italic mark", "Removed italic mark");
    else if (idx == 2) toggle_mark("code", "Added code mark", "Removed code mark");
    else if (idx == 3) {
        EditorState st = EditorView_get_state(g_view);
        Selection_2 sel = EditorState_get_selection(st);
        JsString pname = NodeType_get_name(Node_3_get_type(ResolvedPos_get_parent(Selection_2_get__from(sel))));
        Transaction tr = EditorState_get_tr(st);
        Schema_nodes nodes = Schema_get_nodes(g_schema);
        if (JsString_equals(pname, "heading")) {
            Transaction_setBlockType(tr, Selection_2_get_from(sel), Selection_2_get_to(sel), Schema_nodes_get_at(nodes, "paragraph"), (Attrs){ 0 });
            g_last_action = "Converted heading -> paragraph";
        } else {
            Attrs ha = Attrs_create();
            js_set_num(ha, "level", 2);
            Transaction_setBlockType(tr, Selection_2_get_from(sel), Selection_2_get_to(sel), Schema_nodes_get_at(nodes, "heading"), ha);
            g_last_action = "Converted block -> heading[level=2]";
        }
        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    } else if (idx == 4) {
        g_dispatches = 0;
        g_last_action = "Reset initial schema document";
        EditorState fresh = make_state();
        EditorView_updateState(g_view, fresh);
        update_meta(fresh);
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, make_state());
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    update_meta(EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
