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

static char lower_ch(char c) {
    return (c >= 'A' && c <= 'Z') ? (char)(c + ('a' - 'A')) : c;
}
static int str_len_c(const char* s) {
    int n = 0;
    while (s[n]) n++;
    return n;
}
static int find_ci(const char* hay, int hlen, const char* needle, int nlen, int from) {
    if (nlen <= 0) return -1;
    for (int i = from; i <= hlen - nlen; i++) {
        bool ok = true;
        for (int j = 0; j < nlen; j++) {
            if (lower_ch(hay[i + j]) != lower_ch(needle[j])) { ok = false; break; }
        }
        if (ok) return i;
    }
    return -1;
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
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, 0));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(para_dom, 0));

    NodeSpec cblock = NodeSpec_create();
    NodeSpec_set_content(cblock, "text*");
    NodeSpec_set_marks(cblock, "");
    NodeSpec_set_group(cblock, "block");
    NodeSpec_set_code(cblock, js_some_bool(true));
    NodeSpec_set_toDOM(cblock, Node_to_DOMOutputSpec_fn_from(code_block_dom, 0));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    Attrs_set_at(nodes, "doc", (JsValue){ doc.h });
    Attrs_set_at(nodes, "heading", (JsValue){ heading.h });
    Attrs_set_at(nodes, "paragraph", (JsValue){ para.h });
    Attrs_set_at(nodes, "code_block", (JsValue){ cblock.h });
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
        g_schema, "heading", h_attrs, Schema_text(g_schema, "Immutable State & View Decorations", (JsArray_Mark){ 0 })
    );
    Node_3 p1 = p_node("A ProseMirror schema validates every document tree before state updates commit to the view.");
    Node_3 p2 = p_node("Each transaction maps positions forward so state plugins can recompute decorations without mutating the schema.");
    Node_3 cb = Schema_node_with_string(
        g_schema, "code_block", (Attrs){ 0 },
        Schema_text(g_schema, "const nextState = state.apply(transaction);\nview.updateState(nextState); // sync view", (JsArray_Mark){ 0 })
    );
    Node_3 p3 = p_node("Inline decorations highlight matching tokens while node decorations outline matching blocks in the view.");

    JsArray_Node_3 blocks = JsArray_Node_3_new();
    JsArray_Node_3_push(blocks, h);
    JsArray_Node_3_push(blocks, p1);
    JsArray_Node_3_push(blocks, p2);
    JsArray_Node_3_push(blocks, cb);
    JsArray_Node_3_push(blocks, p3);
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, blocks);
}

static DecorationSet compute_decos(Node_3 doc, const char* query, int* out_inline, int* out_blocks) {
    int qlen = str_len_c(query);
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
                uint32_t ulen = JsString_copy(js, txt, sizeof txt);
                int tlen = (int)(ulen < sizeof txt ? ulen : sizeof txt - 1);
                js_release(js);
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
    JsValue qv = PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        JsString qs = { qv.h };
        JsString_copy(qs, qbuf, sizeof qbuf);
    }
    DecorationSet dset = compute_decos(EditorState_get_doc(st), qbuf, 0, 0);
    return DecorationSet_as_DecorationSource(dset);
}

static void sync_meta(EditorState st) {
    JsValue qv = PluginKey_getState(g_search_key, st);
    char qbuf[64] = { 0 };
    if (qv.h != 0) {
        JsString qs = { qv.h };
        JsString_copy(qs, qbuf, sizeof qbuf);
    }
    int inl = 0, blk = 0;
    DecorationSet dset = compute_decos(EditorState_get_doc(st), qbuf, &inl, &blk);
    int total = (int)JsArray_Decoration_length(DecorationSet_find(dset));

    StrBuf sb = { .pos = 0 };
    sb_str(&sb, "Query: ");
    if (qbuf[0] == 0) sb_str(&sb, "(none)");
    else { sb_str(&sb, "\""); sb_str(&sb, qbuf); sb_str(&sb, "\""); }
    sb_str(&sb, " · inline matches="); sb_int(&sb, inl);
    sb_str(&sb, " · highlighted blocks="); sb_int(&sb, blk);
    sb_str(&sb, " · decorations="); sb_int(&sb, total);
    sb.buf[sb.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "meta").h }, sb.buf);

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        Element_set_className(Document_getElementById(g_doc, ids[i]), i == g_active_btn ? "active" : "");
    }
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    sync_meta(next);
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static const char* QUERIES[5] = { "state", "schema", "transaction", "view", "" };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    g_active_btn = idx;
    Transaction tr = EditorState_get_tr(EditorView_get_state(g_view));
    Transaction_setMeta(tr, "search", js_string(QUERIES[idx]));
    call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
}

__attribute__((export_name("start"))) void start(void) {
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    g_search_key = PluginKey_new("search");
    js_retain(g_search_key);
    StateField field = StateField_create();
    StateField_set_init(field, StateField_init_from(search_init, 0));
    StateField_set_apply(field, StateField_apply_from(search_apply, 0));

    EditorProps eprops = EditorProps_create();
    EditorProps_set_decorations(eprops, any_EditorState_to_opt_DecorationSource_fn_from(search_decos, 0));

    PluginSpec pspec = PluginSpec_create();
    PluginSpec_set_key(pspec, g_search_key);
    PluginSpec_set_state(pspec, field);
    PluginSpec_set_props(pspec, eprops);
    Plugin_2 plugin = Plugin_2_new(pspec);

    JsArray_Plugin_2 plugins = JsArray_Plugin_2_new();
    JsArray_Plugin_2_push(plugins, plugin);

    EditorStateConfig scfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(scfg, g_schema);
    EditorStateConfig_set_doc(scfg, make_initial_doc());
    EditorStateConfig_set_plugins(scfg, plugins);
    EditorState st = EditorState_create(scfg);

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, st);
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, 0));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    sync_meta(EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
