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
    if (val == 0) tmp[n++] = '0';
    while (val > 0) {
        tmp[n++] = (char)('0' + (val % 10));
        val /= 10;
    }
    for (int i = n - 1; i >= 0 && s->pos < 16380; i--) s->buf[s->pos++] = tmp[i];
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
static JsValue mark_dom(void* ctx, Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Schema g_schema;
static PluginKey g_audit_key;
static EditorView g_view;

static bool g_locked = false;
static int g_blocked = 0;
static int g_rev = 0;
static const char* g_last_action = "Ready — open write access";

static JsArray_Step g_undo[64];
static int g_undo_len = 0;
static JsArray_Step g_redo[64];
static int g_redo_len = 0;

static void clear_redo(void) {
    for (int i = 0; i < g_redo_len; i++) js_release(g_redo[i]);
    g_redo_len = 0;
}

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
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, 0));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(para_dom, 0));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    Attrs_set_at(nodes, "doc", (JsValue){ doc.h });
    Attrs_set_at(nodes, "heading", (JsValue){ heading.h });
    Attrs_set_at(nodes, "paragraph", (JsValue){ para.h });
    Attrs_set_at(nodes, "text", (JsValue){ text.h });

    MarkSpec strong = MarkSpec_create();
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    MarkSpec code = MarkSpec_create();
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));
    Attrs marks = Attrs_create();
    Attrs_set_at(marks, "strong", (JsValue){ strong.h });
    Attrs_set_at(marks, "code", (JsValue){ code.h });

    SchemaSpec spec = SchemaSpec_create();
    SchemaSpec_set_nodes(spec, (JsValue){ nodes.h });
    SchemaSpec_set_marks(spec, (JsValue){ marks.h });
    return Schema_new(spec);
}

static Node_3 p_node(const char* t) {
    return Schema_node_with_string(g_schema, "paragraph", (Attrs){ 0 }, Schema_text(g_schema, t, (JsArray_Mark){ 0 }));
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    Attrs_set_at(h_attrs, "level", js_number(2));
    Node_3 h = Schema_node_with_string(
        g_schema, "heading", h_attrs, Schema_text(g_schema, "Step-Inversion Undo & Transaction Filter", (JsArray_Mark){ 0 })
    );
    JsArray_Node_3 blocks = JsArray_Node_3_new();
    JsArray_Node_3_push(blocks, h);
    JsArray_Node_3_push(blocks, p_node("Every document change produces atomic Transform steps that can be inverted against prior snapshots."));
    JsArray_Node_3_push(blocks, p_node("A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."));
    JsArray_Node_3_push(blocks, p_node("Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo."));
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, blocks);
}

static JsValue audit_init(void* ctx, EditorStateConfig cfg, EditorState inst) {
    (void)ctx; (void)cfg; (void)inst;
    return js_number(0);
}
static JsValue audit_apply(void* ctx, Transaction tr, JsValue prev, EditorState os, EditorState ns) {
    (void)ctx; (void)os; (void)ns;
    double n = js_to_number(prev);
    return js_number(Transform_get_docChanged(tr) ? n + 1.0 : n);
}
static bool filter_tr(void* ctx, Transaction tr, EditorState st) {
    (void)ctx; (void)st;
    if (g_locked && Transform_get_docChanged(tr)) {
        g_blocked++;
        g_last_action = "Blocked document mutation (filterTransaction=false)";
        return false;
    }
    return true;
}

static void update_meta(EditorState st) {
    int edits = (int)js_to_number(PluginKey_getState(g_audit_key, st));
    StrBuf sb = { .pos = 0 };
    sb_str(&sb, g_last_action);
    sb_str(&sb, " · edits="); sb_int(&sb, edits);
    sb_str(&sb, " · undoDepth="); sb_int(&sb, g_undo_len);
    sb_str(&sb, " · redoDepth="); sb_int(&sb, g_redo_len);
    sb_str(&sb, " · lock="); sb_str(&sb, g_locked ? "LOCKED" : "OPEN");
    sb_str(&sb, " · blocked="); sb_int(&sb, g_blocked);
    sb_str(&sb, " · blocks="); sb_int(&sb, (int)Node_3_get_childCount(EditorState_get_doc(st)));
    sb.buf[sb.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "meta").h }, sb.buf);
    Element_set_className(Document_getElementById(g_doc, "btn-4"), g_locked ? "active" : "");
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    EditorState_applyTransaction_result res = EditorState_applyTransaction(EditorView_get_state(g_view), tr);
    if (JsArray_Transaction_length(EditorState_applyTransaction_result_get_transactions(res)) > 0) {
        if (Transform_get_docChanged(tr)) {
            JsArray_Step steps = Transform_get_steps(tr);
            JsArray_Node_3 docs = Transform_get_docs(tr);
            JsArray_Step inv = JsArray_Step_new();
            for (int i = (int)JsArray_Step_length(steps) - 1; i >= 0; i--) {
                JsArray_Step_push(inv, Step_invert(JsArray_Step_get(steps, (uint32_t)i), JsArray_Node_3_get(docs, (uint32_t)i)));
            }
            JsValue meta = Transaction_getMeta(tr, "history");
            if (meta.h == 0) {
                if (g_undo_len < 64) { js_retain(inv); g_undo[g_undo_len++] = inv; }
                clear_redo();
            } else {
                JsString ms = { meta.h };
                if (js_str_eq(ms, "undo")) {
                    if (g_redo_len < 64) { js_retain(inv); g_redo[g_redo_len++] = inv; }
                } else {
                    if (g_undo_len < 64) { js_retain(inv); g_undo[g_undo_len++] = inv; }
                }
            }
        }
        EditorView_updateState(g_view, EditorState_applyTransaction_result_get_state(res));
    }
    update_meta(EditorView_get_state(g_view));
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    EditorState st = EditorView_get_state(g_view);
    if (idx == 0) {
        g_rev++;
        StrBuf rb = { .pos = 0 };
        sb_str(&rb, "Revision #"); sb_int(&rb, g_rev);
        sb_str(&rb, " recorded via Transform step.");
        rb.buf[rb.pos] = 0;
        Transaction tr = EditorState_get_tr(st);
        Transform_insert_with_Node(tr, Fragment_get_size(Node_3_get_content(EditorState_get_doc(st))), p_node(rb.buf));
        if (!g_locked) g_last_action = "Appended revision paragraph";
        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    } else if (idx == 1) {
        Node_3 doc = EditorState_get_doc(st);
        if (Node_3_get_childCount(doc) >= 2.0) {
            double from_p = Node_3_get_nodeSize(Node_3_child(doc, 0)) + 1.0;
            double to_p = from_p + Fragment_get_size(Node_3_get_content(Node_3_child(doc, 1)));
            if (to_p > from_p) {
                Transaction tr = EditorState_get_tr(st);
                Transform_addMark(tr, from_p, to_p, Schema_mark(g_schema, "strong", (Attrs){ 0 }));
                if (!g_locked) g_last_action = "Bolded first body paragraph";
                call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
            }
        }
    } else if (idx == 2) {
        if (g_undo_len == 0) {
            g_last_action = "Undo stack empty";
            update_meta(st);
        } else {
            JsArray_Step batch = g_locked ? g_undo[g_undo_len - 1] : g_undo[--g_undo_len];
            Transaction tr = EditorState_get_tr(st);
            Transaction_setMeta(tr, "history", js_string("undo"));
            uint32_t n = JsArray_Step_length(batch);
            for (uint32_t i = 0; i < n; i++) Transform_step(tr, JsArray_Step_get(batch, i));
            if (!g_locked) { js_release(batch); g_last_action = "Undid step(s) via Step.invert"; }
            call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
        }
    } else if (idx == 3) {
        if (g_redo_len == 0) {
            g_last_action = "Redo stack empty";
            update_meta(st);
        } else {
            JsArray_Step batch = g_locked ? g_redo[g_redo_len - 1] : g_redo[--g_redo_len];
            Transaction tr = EditorState_get_tr(st);
            Transaction_setMeta(tr, "history", js_string("redo"));
            uint32_t n = JsArray_Step_length(batch);
            for (uint32_t i = 0; i < n; i++) Transform_step(tr, JsArray_Step_get(batch, i));
            if (!g_locked) { js_release(batch); g_last_action = "Redid inverted step(s)"; }
            call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
        }
    } else if (idx == 4) {
        g_locked = !g_locked;
        g_last_action = g_locked ? "Enabled read-only lock" : "Disabled read-only lock";
        update_meta(st);
    }
}

__attribute__((export_name("start"))) void start(void) {
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_audit_key = PluginKey_new("audit");
    js_retain(g_audit_key);

    StateField field = StateField_create();
    StateField_set_init(field, StateField_init_from(audit_init, 0));
    StateField_set_apply(field, StateField_apply_from(audit_apply, 0));

    PluginSpec pspec = PluginSpec_create();
    PluginSpec_set_key(pspec, g_audit_key);
    PluginSpec_set_state(pspec, field);
    PluginSpec_set_filterTransaction(pspec, PluginSpec_filterTransaction_from(filter_tr, 0));

    JsArray_Plugin_2 plugins = JsArray_Plugin_2_new();
    JsArray_Plugin_2_push(plugins, Plugin_2_new(pspec));

    EditorStateConfig scfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(scfg, g_schema);
    EditorStateConfig_set_doc(scfg, make_initial_doc());
    EditorStateConfig_set_plugins(scfg, plugins);

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, EditorState_create(scfg));
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
