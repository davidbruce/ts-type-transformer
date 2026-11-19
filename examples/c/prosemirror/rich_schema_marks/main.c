#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

typedef struct {
    char buf[16384];
    int pos;
} StrBuf;

static void sb_str(StrBuf* s, const char* txt) {
    while (*txt && s->pos < 16380) s->buf[s->pos++] = *txt++;
}
static void sb_int(StrBuf* s, int val) {
    char tmp[24];
    int n = 0;
    if (val < 0) {
        if (s->pos < 16380) s->buf[s->pos++] = '-';
        val = -val;
    }
    if (val == 0) tmp[n++] = '0';
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0 && s->pos < 16380; i--) s->buf[s->pos++] = tmp[i];
}
static void sb_js(StrBuf* s, JsString js) {
    char tmp[128];
    uint32_t len = JsString_copy(js, tmp, sizeof tmp - 1);
    for (uint32_t i = 0; i < len && s->pos < 16380; i++) s->buf[s->pos++] = tmp[i];
    js_release(js);
}
static bool js_str_eq(JsString s, const char* expected) {
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

static JsValue dom_hole(const char* tag) {
    JsArray a = JsArray_new();
    JsArray_push(a, js_string(tag));
    JsArray_push(a, js_number(0));
    return (JsValue){ a.h };
}
static JsValue heading_dom(void* ctx, Node_3 n) {
    (void)ctx;
    int lvl = (int)js_to_number(Attrs_get_at(Node_3_get_attrs(n), "level"));
    return dom_hole(lvl == 1 ? "h1" : "h2");
}
static JsValue para_dom(void* ctx, Node_3 n) { (void)ctx; (void)n; return dom_hole("p"); }
static JsValue quote_dom(void* ctx, Node_3 n) { (void)ctx; (void)n; return dom_hole("blockquote"); }
static JsValue code_block_dom(void* ctx, Node_3 n) {
    (void)ctx; (void)n;
    JsArray inner = JsArray_new();
    JsArray_push(inner, js_string("code"));
    JsArray_push(inner, js_number(0));
    JsArray outer = JsArray_new();
    JsArray_push(outer, js_string("pre"));
    JsArray_push(outer, (JsValue){ inner.h });
    return (JsValue){ outer.h };
}
static JsValue hr_dom(void* ctx, Node_3 n) {
    (void)ctx; (void)n;
    JsArray a = JsArray_new();
    JsArray_push(a, js_string("hr"));
    return (JsValue){ a.h };
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
    NodeSpec_set_defining(heading, js_some_bool(true));
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, 0));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(para_dom, 0));

    NodeSpec quote = NodeSpec_create();
    NodeSpec_set_content(quote, "block+");
    NodeSpec_set_group(quote, "block");
    NodeSpec_set_defining(quote, js_some_bool(true));
    NodeSpec_set_toDOM(quote, Node_to_DOMOutputSpec_fn_from(quote_dom, 0));

    NodeSpec cblock = NodeSpec_create();
    NodeSpec_set_content(cblock, "text*");
    NodeSpec_set_marks(cblock, "");
    NodeSpec_set_group(cblock, "block");
    NodeSpec_set_code(cblock, js_some_bool(true));
    NodeSpec_set_defining(cblock, js_some_bool(true));
    NodeSpec_set_toDOM(cblock, Node_to_DOMOutputSpec_fn_from(code_block_dom, 0));

    NodeSpec hr = NodeSpec_create();
    NodeSpec_set_group(hr, "block");
    NodeSpec_set_atom(hr, js_some_bool(true));
    NodeSpec_set_toDOM(hr, Node_to_DOMOutputSpec_fn_from(hr_dom, 0));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    Attrs_set_at(nodes, "doc", (JsValue){ doc.h });
    Attrs_set_at(nodes, "heading", (JsValue){ heading.h });
    Attrs_set_at(nodes, "paragraph", (JsValue){ para.h });
    Attrs_set_at(nodes, "blockquote", (JsValue){ quote.h });
    Attrs_set_at(nodes, "code_block", (JsValue){ cblock.h });
    Attrs_set_at(nodes, "horizontal_rule", (JsValue){ hr.h });
    Attrs_set_at(nodes, "text", (JsValue){ text.h });

    MarkSpec strong = MarkSpec_create();
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    MarkSpec em = MarkSpec_create();
    MarkSpec_set_toDOM(em, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"em"));
    MarkSpec code = MarkSpec_create();
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));

    Attrs marks = Attrs_create();
    Attrs_set_at(marks, "strong", (JsValue){ strong.h });
    Attrs_set_at(marks, "em", (JsValue){ em.h });
    Attrs_set_at(marks, "code", (JsValue){ code.h });

    SchemaSpec spec = SchemaSpec_create();
    SchemaSpec_set_nodes(spec, (JsValue){ nodes.h });
    SchemaSpec_set_marks(spec, (JsValue){ marks.h });
    return Schema_new(spec);
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    Attrs_set_at(h_attrs, "level", js_number(2));
    Node_3 h = Schema_node_with_string(
        g_schema, "heading", h_attrs, Schema_text(g_schema, "ProseMirror Document Schema", (JsArray_Mark){ 0 })
    );

    Mark m_strong = Schema_mark(g_schema, "strong", (Attrs){ 0 });
    Mark m_em = Schema_mark(g_schema, "em", (Attrs){ 0 });
    Mark m_code = Schema_mark(g_schema, "code", (Attrs){ 0 });
    JsArray_Mark ms = JsArray_Mark_new(); JsArray_Mark_push(ms, m_strong);
    JsArray_Mark me = JsArray_Mark_new(); JsArray_Mark_push(me, m_em);
    JsArray_Mark mc = JsArray_Mark_new(); JsArray_Mark_push(mc, m_code);

    JsArray_Node_3 p1_items = JsArray_Node_3_new();
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, "Inline marks compose cleanly: ", (JsArray_Mark){ 0 }));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, "bold", ms));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, ", ", (JsArray_Mark){ 0 }));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, "italic", me));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, ", and ", (JsArray_Mark){ 0 }));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, "inline_code", mc));
    JsArray_Node_3_push(p1_items, Schema_text(g_schema, " spans.", (JsArray_Mark){ 0 }));
    Node_3 p_marks = Schema_node_with_string_and_Node_array(g_schema, "paragraph", (Attrs){ 0 }, p1_items);

    Node_3 q_inner = Schema_node_with_string(
        g_schema, "paragraph", (Attrs){ 0 },
        Schema_text(g_schema, "Every document conforms to a strict schema enforced on every transaction.", (JsArray_Mark){ 0 })
    );
    Node_3 quote = Schema_node_with_string(g_schema, "blockquote", (Attrs){ 0 }, q_inner);

    Node_3 code_block = Schema_node_with_string(
        g_schema, "code_block", (Attrs){ 0 },
        Schema_text(g_schema, "schema.node(\"paragraph\", null, [schema.text(\"typed\")])", (JsArray_Mark){ 0 })
    );

    Node_3 outro = Schema_node_with_string(
        g_schema, "paragraph", (Attrs){ 0 },
        Schema_text(g_schema, "Select any text in this editor and click a toolbar action to toggle marks or switch block types.", (JsArray_Mark){ 0 })
    );

    JsArray_Node_3 blocks = JsArray_Node_3_new();
    JsArray_Node_3_push(blocks, h);
    JsArray_Node_3_push(blocks, p_marks);
    JsArray_Node_3_push(blocks, quote);
    JsArray_Node_3_push(blocks, code_block);
    JsArray_Node_3_push(blocks, outro);
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, blocks);
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
    double f = 0, t = 0;
    target_range(st, &f, &t);
    Node_3 doc = EditorState_get_doc(st);
    Schema_marks marks = Schema_get_marks(g_schema);
    bool has_b = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "strong"));
    bool has_e = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "em"));
    bool has_c = Node_3_rangeHasMark_with_MarkType(doc, f, t, Schema_marks_get_at(marks, "code"));

    Element_set_className(Document_getElementById(g_doc, "btn-0"), has_b ? "active" : "");
    Element_set_className(Document_getElementById(g_doc, "btn-1"), has_e ? "active" : "");
    Element_set_className(Document_getElementById(g_doc, "btn-2"), has_c ? "active" : "");

    Selection_2 sel = EditorState_get_selection(st);
    JsString bname = NodeType_get_name(Node_3_get_type(ResolvedPos_get_parent(Selection_2_get__from(sel))));

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, g_last_action);
    sb_str(&sb, " · blocks="); sb_int(&sb, (int)Node_3_get_childCount(doc));
    sb_str(&sb, " · nodeSize="); sb_int(&sb, (int)Node_3_get_nodeSize(doc));
    sb_str(&sb, " · range=["); sb_int(&sb, (int)f); sb_str(&sb, ".."); sb_int(&sb, (int)t); sb_str(&sb, "]");
    sb_str(&sb, " · block="); sb_js(&sb, bname);
    sb_str(&sb, " · dispatches="); sb_int(&sb, g_dispatches);
    sb.buf[sb.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "meta").h }, sb.buf);
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    g_dispatches++;
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    update_meta(next);
}

static void toggle_mark(const char* name, const char* added_msg, const char* removed_msg) {
    EditorState st = EditorView_get_state(g_view);
    double f = 0, t = 0;
    target_range(st, &f, &t);
    if (t <= f) return;
    MarkType mt = Schema_marks_get_at(Schema_get_marks(g_schema), name);
    Transaction tr = EditorState_get_tr(st);
    if (Node_3_rangeHasMark_with_MarkType(EditorState_get_doc(st), f, t, mt)) {
        Transform_removeMark_with_opt_MarkType(tr, f, t, mt);
        g_last_action = removed_msg;
    } else {
        Mark m = Schema_mark_with_MarkType(g_schema, mt, (Attrs){ 0 });
        Transform_addMark(tr, f, t, m);
        g_last_action = added_msg;
    }
    call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
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
        if (js_str_eq(pname, "heading")) {
            Transform_setBlockType(tr, Selection_2_get_from(sel), js_some_double(Selection_2_get_to(sel)), Schema_nodes_get_at(nodes, "paragraph"), (Attrs){ 0 });
            g_last_action = "Converted heading -> paragraph";
        } else {
            Attrs ha = Attrs_create();
            Attrs_set_at(ha, "level", js_number(2));
            Transform_setBlockType(tr, Selection_2_get_from(sel), js_some_double(Selection_2_get_to(sel)), Schema_nodes_get_at(nodes, "heading"), ha);
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
}

__attribute__((export_name("start"))) void start(void) {
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    EditorState st = make_state();
    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, st);
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, 0));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new((dist_DOMNode){ mount.h }, props);
    js_retain(g_view);
    update_meta(EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
