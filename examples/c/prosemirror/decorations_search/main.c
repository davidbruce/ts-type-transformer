#include <ctype.h>
#include <stdio.h>
#include <string.h>
#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

static int find_ci(const char* hay, int hlen, const char* needle, int nlen, int from) {
    if (nlen <= 0) return -1;
    for (int i = from; i <= hlen - nlen; i++) {
        bool ok = true;
        for (int j = 0; j < nlen; j++) {
            if (tolower((unsigned char)hay[i + j]) != tolower((unsigned char)needle[j])) {
                ok = false;
                break;
            }
        }
        if (ok) return i;
    }
    return -1;
}

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
static JsValue mark_dom(void* ctx, Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Schema g_schema;
static PluginKey g_search_key;
static EditorView g_view;
static int g_active_btn = 0;

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

    NodeSpec cblock = NodeSpec_create();
    NodeSpec_set_content(cblock, "text*");
    NodeSpec_set_marks(cblock, "");
    NodeSpec_set_group(cblock, "block");
    NodeSpec_set_code(cblock, true);
    NodeSpec_set_toDOM(cblock, Node_to_DOMOutputSpec_fn_from(code_block_dom, NULL));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "code_block", cblock);
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
        g_schema, "heading", h_attrs, t_node("Immutable State & View Decorations")
    );
    Node_3 p1 = p_node("A ProseMirror schema validates every document tree before state updates commit to the view.");
    Node_3 p2 = p_node("Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema.");
    Node_3 cb = Schema_node_with_string(
        g_schema, "code_block", (Attrs){ 0 },
        t_node("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view")
    );
    Node_3 p3 = p_node("Inline decorations highlight matching tokens while node decorations outline matching blocks in the view.");

    Node_3 blocks_arr[5] = { h, p1, p2, cb, p3 };
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, JsArray_Node_3_from(blocks_arr, 5));
}

static DecorationSet compute_decos(Node_3 doc, const char* query, int* out_inline, int* out_blocks) {
    int qlen = (int)strlen(query);
    if (qlen == 0) {
        if (out_inline) *out_inline = 0;
        if (out_blocks) *out_blocks = 0;
        return DecorationSet_get_empty();
    }
    JsArray_Decoration decos = JsArray_Decoration_new();
    DecorationAttrs inl_attr = DecorationAttrs_create();
    DecorationAttrs_set_class(inl_attr, "pm-match");
    DecorationAttrs blk_attr = DecorationAttrs_create();
    DecorationAttrs_set_class(blk_attr, "pm-hit-block");

    int inl_hits = 0;
    int blk_hits = 0;
    double block_pos = 0.0;
    int nb = (int)Node_3_get_childCount(doc);
    for (int i = 0; i < nb; i++) {
        Node_3 blk = Node_3_child(doc, i);
        double inline_pos = block_pos + 1.0;
        bool blk_matched = false;
        int nc = (int)Node_3_get_childCount(blk);
        for (int j = 0; j < nc; j++) {
            Node_3 ch = Node_3_child(blk, j);
            if (Node_3_get_isText(ch)) {
                JsString js = Node_3_get_textContent(ch);
                char txt[1024];
                uint32_t ulen = JsString_copy(js, txt, sizeof txt - 1);
                int tlen = (int)(ulen < sizeof txt ? ulen : sizeof txt - 1);
                txt[tlen] = 0;
                int at = find_ci(txt, tlen, query, qlen, 0);
                while (at >= 0) {
                    double fp = inline_pos + (double)at;
                    double tp = fp + (double)qlen;
                    JsArray_Decoration_push(decos, Decoration_inline(fp, tp, inl_attr));
                    inl_hits++;
                    blk_matched = true;
                    at = find_ci(txt, tlen, query, qlen, at + qlen);
                }
            }
            inline_pos += Node_3_get_nodeSize(ch);
        }
        if (blk_matched) {
            JsArray_Decoration_push(decos, Decoration_node(block_pos, block_pos + Node_3_get_nodeSize(blk), blk_attr));
            blk_hits++;
        }
        block_pos += Node_3_get_nodeSize(blk);
    }
    if (out_inline) *out_inline = inl_hits;
    if (out_blocks) *out_blocks = blk_hits;
    return DecorationSet_create(doc, decos);
}

static JsValue search_init(void* ctx, EditorStateConfig cfg, EditorState inst) {
    (void)ctx; (void)cfg; (void)inst;
    return js_string("state");
}
static JsValue search_apply(void* ctx, Transaction tr, JsValue prev, EditorState os, EditorState ns) {
    (void)ctx; (void)os; (void)ns;
    JsValue mv = Transaction_getMeta(tr, "search");
    if (mv.h != 0) return mv;
    js_retain(prev);
    return prev;
}
static DecorationSource search_decos(void* ctx, EditorState st) {
    (void)ctx;
    js_scope_open();
    JsValue qv = PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        uint32_t len = JsString_copy((JsString){ qv.h }, qbuf, sizeof qbuf - 1);
        qbuf[len < sizeof qbuf ? len : sizeof qbuf - 1] = 0;
    }
    DecorationSource res = DecorationSet_as_DecorationSource(compute_decos(EditorState_get_doc(st), qbuf, NULL, NULL));
    js_retain(res);
    js_scope_close();
    return res;
}

static void sync_meta(EditorState st) {
    js_scope_open();
    JsValue qv = PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        uint32_t len = JsString_copy((JsString){ qv.h }, qbuf, sizeof qbuf - 1);
        qbuf[len < sizeof qbuf ? len : sizeof qbuf - 1] = 0;
    }
    int inl = 0, blk = 0;
    DecorationSet dset = compute_decos(EditorState_get_doc(st), qbuf, &inl, &blk);
    int total = (int)JsArray_Decoration_length(DecorationSet_find(dset));

    char qdisp[80];
    if (qbuf[0] == 0) snprintf(qdisp, sizeof qdisp, "(none)");
    else snprintf(qdisp, sizeof qdisp, "\"%s\"", qbuf);

    char meta[256];
    snprintf(meta, sizeof meta, "Query: %s \xC2\xB7 inline matches=%d \xC2\xB7 highlighted blocks=%d \xC2\xB7 decorations=%d",
             qdisp, inl, blk, total);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement_set_className(Document_getElementById(g_doc, ids[i]), i == g_active_btn ? "active" : "");
    }
    js_scope_close();
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    sync_meta(next);
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static const char* QUERIES[5] = { "state", "schema", "transaction", "view", "" };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    g_active_btn = idx;
    Transaction tr = EditorState_get_tr(EditorView_get_state(g_view));
    Transaction_setMeta(tr, "search", js_string(QUERIES[idx]));
    call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_search_key = PluginKey_new("search");
    js_retain(g_search_key);
    StateField field = StateField_create();
    StateField_set_init(field, StateField_init_from(search_init, NULL));
    StateField_set_apply(field, StateField_apply_from(search_apply, NULL));

    EditorProps eprops = EditorProps_create();
    EditorProps_set_decorations(eprops, any_EditorState_to_opt_DecorationSource_fn_from(search_decos, NULL));

    PluginSpec pspec = PluginSpec_create();
    PluginSpec_set_key(pspec, g_search_key);
    PluginSpec_set_state(pspec, field);
    PluginSpec_set_props(pspec, eprops);
    Plugin_2 plugin = Plugin_2_new(pspec);

    EditorStateConfig scfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(scfg, g_schema);
    EditorStateConfig_set_doc(scfg, make_initial_doc());
    EditorStateConfig_set_plugins(scfg, JsArray_Plugin_2_from(&plugin, 1));
    EditorState st = EditorState_create(scfg);

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, st);
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    sync_meta(EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
