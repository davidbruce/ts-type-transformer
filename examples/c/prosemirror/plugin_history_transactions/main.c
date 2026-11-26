#include <stdio.h>
#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

static JsValue dom_hole(const char* tag) {
    return js_value(js_values(tag, 0));
}
static JsValue heading_dom(void* ctx, Pm_Node n) {
    (void)ctx;
    js_scope();
    int lvl = (int)js_to_number(js_get_prop(Pm_Node_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    return res;
}
static JsValue para_dom(void* ctx, Pm_Node n) { (void)ctx; (void)n; return dom_hole("p"); }
static JsValue mark_dom(void* ctx, Pm_Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Pm_Schema g_schema;
static Pm_PluginKey g_audit_key;
static Pm_EditorView g_view;

static bool g_locked = false;
static int g_blocked = 0;
static int g_rev = 0;
static const char* g_last_action = "Ready — open write access";

static JsArray_Pm_Step g_undo[64];
static int g_undo_len = 0;
static JsArray_Pm_Step g_redo[64];
static int g_redo_len = 0;

static void clear_redo(void) {
    for (int i = 0; i < g_redo_len; i++) js_release(g_redo[i]);
    g_redo_len = 0;
}

static Pm_Schema make_schema(void) {
    Pm_NodeSpec doc = Pm_NodeSpec_create(.content = "block+");

    Pm_AttributeSpec lvl = Pm_AttributeSpec_create(.default_ = js_number(2));
    Pm_NodeSpec heading = Pm_NodeSpec_create(.attrs = Pm_AttributeSpec_record({ "level", lvl }), .content = "inline*", .group = "block", .toDOM_fn = heading_dom);

    Pm_NodeSpec para = Pm_NodeSpec_create(.content = "inline*", .group = "block", .toDOM_fn = para_dom);

    Pm_NodeSpec text = Pm_NodeSpec_create(.group = "inline");

    Pm_MarkSpec strong = Pm_MarkSpec_create(.toDOM = Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"strong"));
    Pm_MarkSpec code = Pm_MarkSpec_create(.toDOM = Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"code"));

    return Pm_Schema_new(Pm_SchemaSpec_create(
        .nodes = Pm_NodeSpec_record({ "doc", doc }, { "heading", heading }, { "paragraph", para }, { "text", text }),
        .marks = Pm_MarkSpec_record({ "strong", strong }, { "code", code })
    ));
}

static Pm_Node t_node(const char* t) {
    return Pm_Schema_text(g_schema, t);
}
static Pm_Node p_node(const char* t) {
    return Pm_Schema_node(g_schema, "paragraph", NULL, t_node(t));
}

static Pm_Node make_initial_doc(void) {
    Pm_Node h = Pm_Schema_node(
        g_schema, "heading", Pm_Attrs_record({ "level", js_number(2) }), t_node("Step-Inversion Undo & Transaction Filter")
    );
    return Pm_Schema_node(g_schema, "doc", NULL, Pm_Node_list(
        h,
        p_node("Every document change produces atomic Transform steps that can be inverted against prior snapshots."),
        p_node("A ProseMirror Plugin inspects transactions in filterTransaction and can enforce a read-only lock."),
        p_node("Click Append Revision or Bold First Line, then step backward with Undo and forward with Redo.")
    ));
}

static JsValue audit_init(void* ctx, Pm_EditorStateConfig cfg, Pm_EditorState inst) {
    (void)ctx; (void)cfg; (void)inst;
    return js_number(0);
}
static JsValue audit_apply(void* ctx, Pm_Transaction tr, JsValue prev, Pm_EditorState os, Pm_EditorState ns) {
    (void)ctx; (void)os; (void)ns;
    double n = js_to_number(prev);
    return js_number(Pm_Transaction_get_docChanged(tr) ? n + 1.0 : n);
}
static bool filter_tr(void* ctx, Pm_Transaction tr, Pm_EditorState st) {
    (void)ctx; (void)st;
    if (g_locked && Pm_Transaction_get_docChanged(tr)) {
        g_blocked++;
        g_last_action = "Blocked document mutation (filterTransaction=false)";
        return false;
    }
    return true;
}

static void update_meta(Pm_EditorState st) {
    js_scope();
    int edits = (int)js_to_number(Pm_PluginKey_getState(g_audit_key, st));
    int blocks = (int)Pm_Node_get_childCount(Pm_EditorState_get_doc(st));
    char meta[256];
    snprintf(
        meta, sizeof meta,
        "%s \xC2\xB7 edits=%d \xC2\xB7 undoDepth=%d \xC2\xB7 redoDepth=%d \xC2\xB7 lock=%s \xC2\xB7 blocked=%d \xC2\xB7 blocks=%d",
        g_last_action, edits, g_undo_len, g_redo_len,
        g_locked ? "LOCKED" : "OPEN", g_blocked, blocks
    );
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-4"), g_locked ? "active" : "");
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope();
    Pm_EditorState_applyTransaction_result res = Pm_EditorState_applyTransaction(Pm_EditorView_get_state(g_view), tr);
    if (JsArray_Pm_Transaction_length(Pm_EditorState_applyTransaction_result_get_transactions(res)) > 0) {
        if (Pm_Transaction_get_docChanged(tr)) {
            JsArray_Pm_Step steps = Pm_Transaction_get_steps(tr);
            JsArray_Pm_Node docs = Pm_Transaction_get_docs(tr);
            JsArray_Pm_Step inv = JsArray_Pm_Step_new();
            for (int i = (int)JsArray_Pm_Step_length(steps) - 1; i >= 0; i--) {
                JsArray_Pm_Step_push(inv, Pm_Step_invert(JsArray_Pm_Step_get(steps, (uint32_t)i), JsArray_Pm_Node_get(docs, (uint32_t)i)));
            }
            JsValue meta = Pm_Transaction_getMeta(tr, "history");
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
        Pm_EditorView_updateState(g_view, Pm_EditorState_applyTransaction_result_get_state(res));
    }
    update_meta(Pm_EditorView_get_state(g_view));
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int idx = *(const int*)ctx;
    Pm_EditorState st = Pm_EditorView_get_state(g_view);
    if (idx == 0) {
        g_rev++;
        char rbuf[64];
        snprintf(rbuf, sizeof rbuf, "Revision #%d recorded via Transform step.", g_rev);
        Pm_Transaction tr = Pm_EditorState_get_tr(st);
        Pm_Transaction_insert(tr, Pm_Fragment_get_size(Pm_Node_get_content(Pm_EditorState_get_doc(st))), p_node(rbuf));
        if (!g_locked) g_last_action = "Appended revision paragraph";
        Pm_EditorView_dispatch(g_view, tr);
    } else if (idx == 1) {
        Pm_Node doc = Pm_EditorState_get_doc(st);
        if (Pm_Node_get_childCount(doc) >= 2.0) {
            double from_p = Pm_Node_get_nodeSize(Pm_Node_child(doc, 0)) + 1.0;
            double to_p = from_p + Pm_Fragment_get_size(Pm_Node_get_content(Pm_Node_child(doc, 1)));
            if (to_p > from_p) {
                Pm_Transaction tr = Pm_EditorState_get_tr(st);
                Pm_Transaction_addMark(tr, from_p, to_p, Pm_Schema_mark(g_schema, "strong"));
                if (!g_locked) g_last_action = "Bolded first body paragraph";
                Pm_EditorView_dispatch(g_view, tr);
            }
        }
    } else if (idx == 2) {
        if (g_undo_len == 0) {
            g_last_action = "Undo stack empty";
            update_meta(st);
        } else {
            JsArray_Pm_Step batch = g_locked ? g_undo[g_undo_len - 1] : g_undo[--g_undo_len];
            Pm_Transaction tr = Pm_EditorState_get_tr(st);
            Pm_Transaction_setMeta(tr, "history", js_string("undo"));
            uint32_t n = JsArray_Pm_Step_length(batch);
            for (uint32_t i = 0; i < n; i++) Pm_Transaction_step(tr, JsArray_Pm_Step_get(batch, i));
            if (!g_locked) { js_release(batch); g_last_action = "Undid step(s) via Step.invert"; }
            Pm_EditorView_dispatch(g_view, tr);
        }
    } else if (idx == 3) {
        if (g_redo_len == 0) {
            g_last_action = "Redo stack empty";
            update_meta(st);
        } else {
            JsArray_Pm_Step batch = g_locked ? g_redo[g_redo_len - 1] : g_redo[--g_redo_len];
            Pm_Transaction tr = Pm_EditorState_get_tr(st);
            Pm_Transaction_setMeta(tr, "history", js_string("redo"));
            uint32_t n = JsArray_Pm_Step_length(batch);
            for (uint32_t i = 0; i < n; i++) Pm_Transaction_step(tr, JsArray_Pm_Step_get(batch, i));
            if (!g_locked) { js_release(batch); g_last_action = "Redid inverted step(s)"; }
            Pm_EditorView_dispatch(g_view, tr);
        }
    } else if (idx == 4) {
        g_locked = !g_locked;
        g_last_action = g_locked ? "Enabled read-only lock" : "Disabled read-only lock";
        update_meta(st);
    }
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_audit_key = Pm_PluginKey_new("audit");
    js_retain(g_audit_key);

    Pm_StateField field = Pm_StateField_create();
    Pm_StateField_set_init(field, audit_init);
    Pm_StateField_set_apply(field, audit_apply);

    Pm_PluginSpec pspec = Pm_PluginSpec_create(.key = g_audit_key, .state = field, .filterTransaction_fn = filter_tr);
    Pm_Plugin plugin = Pm_Plugin_new(pspec);

    Pm_EditorStateConfig scfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = make_initial_doc(), .plugins = Pm_Plugin_list(plugin));

    Pm_DirectEditorProps props = Pm_DirectEditorProps_create(.state = Pm_EditorState_create(scfg), .dispatchTransaction_fn = on_dispatch);

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(mount, props);
    js_retain(g_view);
    update_meta(Pm_EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
