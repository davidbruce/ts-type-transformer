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
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(para_dom, NULL));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "text", text);

    MarkSpec strong = MarkSpec_create();
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    MarkSpec code = MarkSpec_create();
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));
    Attrs marks = Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "code", code);

    SchemaSpec spec = SchemaSpec_create();
    js_set_val(spec, "nodes", nodes);
    js_set_val(spec, "marks", marks);
    return Schema_new(spec);
}

static Node_3 t_node(const char* t) {
    return Schema_text(g_schema, t, (JsArray_Mark){ 0 });
}
static Node_3 p_node(const char* t) {
    return Schema_node_with_string(g_schema, "paragraph", (Attrs){ 0 }, t_node(t));
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Node_3 h = Schema_node_with_string(
        g_schema, "heading", h_attrs, t_node("Step-Inversion Undo & Transaction Filter")
    );
    Node_3 blocks[4] = {
        h,
        p_node("Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
        p_node("A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
        p_node("Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo.")
    };
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, JsArray_Node_3_from(blocks, 4));
}

static JsValue audit_init(void* ctx, EditorStateConfig cfg, EditorState inst) {
    (void)ctx; (void)cfg; (void)inst;
    return js_number(0);
}
static JsValue audit_apply(void* ctx, Transaction tr, JsValue prev, EditorState os, EditorState ns) {
    (void)ctx; (void)os; (void)ns;
    double n = js_to_number(prev);
    return js_number(Transaction_get_docChanged(tr) ? n + 1.0 : n);
}
static bool filter_tr(void* ctx, Transaction tr, EditorState st) {
    (void)ctx; (void)st;
    if (g_locked && Transaction_get_docChanged(tr)) {
        g_blocked++;
        g_last_action = "Blocked document mutation (filterTransaction=false)";
        return false;
    }
    return true;
}

static void update_meta(EditorState st) {
    js_scope_open();
    int edits = (int)js_to_number(PluginKey_getState(g_audit_key, st));
    int blocks = (int)Node_3_get_childCount(EditorState_get_doc(st));
    char meta[256];
    snprintf(
        meta, sizeof meta,
        "%s \xC2\xB7 edits=%d \xC2\xB7 undoDepth=%d \xC2\xB7 redoDepth=%d \xC2\xB7 lock=%s \xC2\xB7 blocked=%d \xC2\xB7 blocks=%d",
        g_last_action, edits, g_undo_len, g_redo_len,
        g_locked ? "LOCKED" : "OPEN", g_blocked, blocks
    );
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-4"), g_locked ? "active" : "");
    js_scope_close();
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    EditorState_applyTransaction_result res = EditorState_applyTransaction(EditorView_get_state(g_view), tr);
    if (JsArray_Transaction_length(EditorState_applyTransaction_result_get_transactions(res)) > 0) {
        if (Transaction_get_docChanged(tr)) {
            JsArray_Step steps = Transaction_get_steps(tr);
            JsArray_Node_3 docs = Transaction_get_docs(tr);
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
                if (JsString_equals(ms, "undo")) {
                    if (g_redo_len < 64) { js_retain(inv); g_redo[g_redo_len++] = inv; }
                } else {
                    if (g_undo_len < 64) { js_retain(inv); g_undo[g_undo_len++] = inv; }
                }
            }
        }
        EditorView_updateState(g_view, EditorState_applyTransaction_result_get_state(res));
    }
    update_meta(EditorView_get_state(g_view));
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    EditorState st = EditorView_get_state(g_view);
    if (idx == 0) {
        g_rev++;
        char rbuf[64];
        snprintf(rbuf, sizeof rbuf, "Revision #%d recorded via Transform step.", g_rev);
        Transaction tr = EditorState_get_tr(st);
        Transaction_insert(tr, Fragment_get_size(Node_3_get_content(EditorState_get_doc(st))), p_node(rbuf));
        if (!g_locked) g_last_action = "Appended revision paragraph";
        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    } else if (idx == 1) {
        Node_3 doc = EditorState_get_doc(st);
        if (Node_3_get_childCount(doc) >= 2.0) {
            double from_p = Node_3_get_nodeSize(Node_3_child(doc, 0)) + 1.0;
            double to_p = from_p + Fragment_get_size(Node_3_get_content(Node_3_child(doc, 1)));
            if (to_p > from_p) {
                Transaction tr = EditorState_get_tr(st);
                Transaction_addMark(tr, from_p, to_p, Schema_mark(g_schema, "strong", (Attrs){ 0 }));
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
            for (uint32_t i = 0; i < n; i++) Transaction_step(tr, JsArray_Step_get(batch, i));
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
            for (uint32_t i = 0; i < n; i++) Transaction_step(tr, JsArray_Step_get(batch, i));
            if (!g_locked) { js_release(batch); g_last_action = "Redid inverted step(s)"; }
            call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
        }
    } else if (idx == 4) {
        g_locked = !g_locked;
        g_last_action = g_locked ? "Enabled read-only lock" : "Disabled read-only lock";
        update_meta(st);
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_audit_key = PluginKey_new("audit");
    js_retain(g_audit_key);

    StateField field = StateField_create();
    StateField_set_init(field, StateField_init_from(audit_init, NULL));
    StateField_set_apply(field, StateField_apply_from(audit_apply, NULL));

    PluginSpec pspec = PluginSpec_create();
    PluginSpec_set_key(pspec, g_audit_key);
    PluginSpec_set_state(pspec, field);
    PluginSpec_set_filterTransaction(pspec, PluginSpec_filterTransaction_from(filter_tr, NULL));
    Plugin_2 plugin = Plugin_2_new(pspec);

    EditorStateConfig scfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(scfg, g_schema);
    EditorStateConfig_set_doc(scfg, make_initial_doc());
    EditorStateConfig_set_plugins(scfg, JsArray_Plugin_2_from(&plugin, 1));

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, EditorState_create(scfg));
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
