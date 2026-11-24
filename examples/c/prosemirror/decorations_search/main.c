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
static JsValue heading_dom(void* ctx, Pm_Node n) {
    (void)ctx;
    js_scope();
    int lvl = (int)js_to_number(js_get_prop(Pm_Node_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    return res;
}
static JsValue para_dom(void* ctx, Pm_Node n) { (void)ctx; (void)n; return dom_hole("p"); }
static JsValue code_block_dom(void* ctx, Pm_Node n) {
    (void)ctx; (void)n;
    js_scope();
    JsArray inner = JsArray_new();
    js_push_str(inner, "code");
    js_push_num(inner, 0);
    JsArray outer = JsArray_new();
    js_push_str(outer, "pre");
    js_push_val(outer, inner);
    js_retain(outer);
    return js_value(outer);
}
static JsValue mark_dom(void* ctx, Pm_Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Pm_Schema g_schema;
static Pm_PluginKey g_search_key;
static Pm_EditorView g_view;
static int g_active_btn = 0;

static Pm_Schema make_schema(void) {
    Pm_NodeSpec doc = Pm_NodeSpec_create(.content = "block+");

    Pm_AttributeSpec lvl = Pm_AttributeSpec_create(.default_ = js_number(2));
    Pm_MarkSpec_attrs hattrs = Pm_MarkSpec_attrs_create();
    Pm_MarkSpec_attrs_set_at(hattrs, "level", lvl);

    Pm_NodeSpec heading = Pm_NodeSpec_create(.attrs = hattrs, .content = "inline*", .group = "block", .toDOM = Pm_Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));

    Pm_NodeSpec para = Pm_NodeSpec_create(.content = "inline*", .group = "block", .toDOM = Pm_Node_to_DOMOutputSpec_fn_from(para_dom, NULL));

    Pm_NodeSpec cblock = Pm_NodeSpec_create(.content = "text*", .marks = "", .group = "block", .code = true, .toDOM = Pm_Node_to_DOMOutputSpec_fn_from(code_block_dom, NULL));

    Pm_NodeSpec text = Pm_NodeSpec_create(.group = "inline");

    Pm_Attrs nodes = Pm_Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "code_block", cblock);
    js_set_val(nodes, "text", text);

    Pm_MarkSpec strong = Pm_MarkSpec_create(.toDOM = Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    Pm_MarkSpec code = Pm_MarkSpec_create(.toDOM = Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));
    Pm_Attrs marks = Pm_Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "code", code);

    return Pm_Schema_new(Pm_SchemaSpec_create(.nodes = js_value(nodes), .marks = js_value(marks)));
}

static Pm_Node t_node(const char* t) {
    return Pm_Schema_text(g_schema, t);
}
static Pm_Node p_node(const char* t) {
    return Pm_Schema_node(g_schema, "paragraph", (Pm_Attrs){ 0 }, t_node(t));
}

static Pm_Node make_initial_doc(void) {
    Pm_Attrs h_attrs = Pm_Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Pm_Node h = Pm_Schema_node(
        g_schema, "heading", h_attrs, t_node("Immutable State & View Decorations")
    );
    Pm_Node p1 = p_node("A ProseMirror schema validates every document tree before state updates commit to the view.");
    Pm_Node p2 = p_node("Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema.");
    Pm_Node cb = Pm_Schema_node(
        g_schema, "code_block", (Pm_Attrs){ 0 },
        t_node("const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view")
    );
    Pm_Node p3 = p_node("Inline decorations highlight matching tokens while node decorations outline matching blocks in the view.");

    Pm_Node blocks_arr[5] = { h, p1, p2, cb, p3 };
    return Pm_Schema_node(g_schema, "doc", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(blocks_arr, 5));
}

static Pm_DecorationSet compute_decos(Pm_Node doc, const char* query, int* out_inline, int* out_blocks) {
    int qlen = (int)strlen(query);
    if (qlen == 0) {
        if (out_inline) *out_inline = 0;
        if (out_blocks) *out_blocks = 0;
        return Pm_DecorationSet_get_empty();
    }
    JsArray_Pm_Decoration decos = JsArray_Pm_Decoration_new();
    Pm_DecorationAttrs inl_attr = Pm_DecorationAttrs_create(.class = "pm-match");
    Pm_DecorationAttrs blk_attr = Pm_DecorationAttrs_create(.class = "pm-hit-block");

    int inl_hits = 0;
    int blk_hits = 0;
    double block_pos = 0.0;
    int nb = (int)Pm_Node_get_childCount(doc);
    for (int i = 0; i < nb; i++) {
        Pm_Node blk = Pm_Node_child(doc, i);
        double inline_pos = block_pos + 1.0;
        bool blk_matched = false;
        int nc = (int)Pm_Node_get_childCount(blk);
        for (int j = 0; j < nc; j++) {
            Pm_Node ch = Pm_Node_child(blk, j);
            if (Pm_Node_get_isText(ch)) {
                JsString js = Pm_Node_get_textContent(ch);
                char txt[1024];
                uint32_t ulen = JsString_copy(js, txt, sizeof txt - 1);
                int tlen = (int)(ulen < sizeof txt ? ulen : sizeof txt - 1);
                txt[tlen] = 0;
                int at = find_ci(txt, tlen, query, qlen, 0);
                while (at >= 0) {
                    double fp = inline_pos + (double)at;
                    double tp = fp + (double)qlen;
                    JsArray_Pm_Decoration_push(decos, Pm_Decoration_inline(fp, tp, inl_attr));
                    inl_hits++;
                    blk_matched = true;
                    at = find_ci(txt, tlen, query, qlen, at + qlen);
                }
            }
            inline_pos += Pm_Node_get_nodeSize(ch);
        }
        if (blk_matched) {
            JsArray_Pm_Decoration_push(decos, Pm_Decoration_node(block_pos, block_pos + Pm_Node_get_nodeSize(blk), blk_attr));
            blk_hits++;
        }
        block_pos += Pm_Node_get_nodeSize(blk);
    }
    if (out_inline) *out_inline = inl_hits;
    if (out_blocks) *out_blocks = blk_hits;
    return Pm_DecorationSet_create(doc, decos);
}

static JsValue search_init(void* ctx, Pm_EditorStateConfig cfg, Pm_EditorState inst) {
    (void)ctx; (void)cfg; (void)inst;
    return js_string("state");
}
static JsValue search_apply(void* ctx, Pm_Transaction tr, JsValue prev, Pm_EditorState os, Pm_EditorState ns) {
    (void)ctx; (void)os; (void)ns;
    JsValue mv = Pm_Transaction_getMeta(tr, "search");
    if (mv.h != 0) return mv;
    js_retain(prev);
    return prev;
}
static Pm_DecorationSource search_decos(void* ctx, Pm_EditorState st) {
    (void)ctx;
    js_scope();
    JsValue qv = Pm_PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        uint32_t len = JsString_copy((JsString){ qv.h }, qbuf, sizeof qbuf - 1);
        qbuf[len < sizeof qbuf ? len : sizeof qbuf - 1] = 0;
    }
    Pm_DecorationSource res = Pm_DecorationSet_as_Pm_DecorationSource(compute_decos(Pm_EditorState_get_doc(st), qbuf, NULL, NULL));
    js_retain(res);
    return res;
}

static void sync_meta(Pm_EditorState st) {
    js_scope();
    JsValue qv = Pm_PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        uint32_t len = JsString_copy((JsString){ qv.h }, qbuf, sizeof qbuf - 1);
        qbuf[len < sizeof qbuf ? len : sizeof qbuf - 1] = 0;
    }
    int inl = 0, blk = 0;
    Pm_DecorationSet dset = compute_decos(Pm_EditorState_get_doc(st), qbuf, &inl, &blk);
    int total = (int)JsArray_Pm_Decoration_length(Pm_DecorationSet_find(dset));

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
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope();
    Pm_EditorState next = Pm_EditorState_apply(Pm_EditorView_get_state(g_view), tr);
    Pm_EditorView_updateState(g_view, next);
    sync_meta(next);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static const char* QUERIES[5] = { "state", "schema", "transaction", "view", "" };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int idx = *(const int*)ctx;
    g_active_btn = idx;
    Pm_Transaction tr = Pm_EditorState_get_tr(Pm_EditorView_get_state(g_view));
    Pm_Transaction_setMeta(tr, "search", js_string(QUERIES[idx]));
    Pm_EditorView_dispatch(g_view, tr);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_search_key = Pm_PluginKey_new("search");
    js_retain(g_search_key);
    Pm_StateField field = Pm_StateField_create();
    Pm_StateField_set_init(field, Pm_StateField_init_cb_from(search_init, NULL));
    Pm_StateField_set_apply(field, Pm_StateField_apply_cb_from(search_apply, NULL));

    Pm_EditorProps eprops = Pm_EditorProps_create(.decorations = Pm_any_EditorState_to_opt_DecorationSource_fn_from(search_decos, NULL));

    Pm_PluginSpec pspec = Pm_PluginSpec_create(.key = g_search_key, .state = field, .props = eprops);
    Pm_Plugin plugin = Pm_Plugin_new(pspec);

    Pm_EditorStateConfig scfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = make_initial_doc(), .plugins = JsArray_Pm_Plugin_from(&plugin, 1));
    Pm_EditorState st = Pm_EditorState_create(scfg);

    Pm_DirectEditorProps props = Pm_DirectEditorProps_create(.state = st, .dispatchTransaction = Pm_Transaction_fn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    sync_meta(Pm_EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
