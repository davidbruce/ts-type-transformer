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
static JsValue tag_dom(void* ctx, Node_3 n) { (void)n; return dom_hole((const char*)ctx); }
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
static int g_total_steps = 0;
static int g_note_counter = 0;
static const char* g_last_op = "Initialized document structure";

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
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"p"));

    NodeSpec quote = NodeSpec_create();
    NodeSpec_set_content(quote, "block+");
    NodeSpec_set_group(quote, "block");
    NodeSpec_set_defining(quote, js_some_bool(true));
    NodeSpec_set_toDOM(quote, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"blockquote"));

    NodeSpec ulist = NodeSpec_create();
    NodeSpec_set_content(ulist, "list_item+");
    NodeSpec_set_group(ulist, "block");
    NodeSpec_set_toDOM(ulist, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"ul"));

    NodeSpec li = NodeSpec_create();
    NodeSpec_set_content(li, "paragraph block*");
    NodeSpec_set_defining(li, js_some_bool(true));
    NodeSpec_set_toDOM(li, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"li"));

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
    Attrs_set_at(nodes, "bullet_list", (JsValue){ ulist.h });
    Attrs_set_at(nodes, "list_item", (JsValue){ li.h });
    Attrs_set_at(nodes, "horizontal_rule", (JsValue){ hr.h });
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
static Node_3 li_node(const char* t) {
    return Schema_node_with_string(g_schema, "list_item", (Attrs){ 0 }, p_node(t));
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    Attrs_set_at(h_attrs, "level", js_number(2));
    Node_3 h = Schema_node_with_string(
        g_schema, "heading", h_attrs, Schema_text(g_schema, "Structural Transform Pipeline", (JsArray_Mark){ 0 })
    );
    Node_3 p1 = p_node("ProseMirror transforms express structural edits as invertible steps.");
    Node_3 p2 = p_node("Wrap blocks in quotes, lift nested content, or split paragraphs cleanly.");
    Node_3 quote = Schema_node_with_string(
        g_schema, "blockquote", (Attrs){ 0 }, p_node("Blockquote content can be lifted back to top-level document depth.")
    );
    JsArray_Node_3 lis = JsArray_Node_3_new();
    JsArray_Node_3_push(lis, li_node("Split at cursor position"));
    JsArray_Node_3_push(lis, li_node("Wrap range with blockquote"));
    JsArray_Node_3_push(lis, li_node("Lift nested block to root"));
    Node_3 ulist = Schema_node_with_string_and_Node_array(g_schema, "bullet_list", (Attrs){ 0 }, lis);

    JsArray_Node_3 blocks = JsArray_Node_3_new();
    JsArray_Node_3_push(blocks, h);
    JsArray_Node_3_push(blocks, p1);
    JsArray_Node_3_push(blocks, p2);
    JsArray_Node_3_push(blocks, quote);
    JsArray_Node_3_push(blocks, ulist);
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, blocks);
}

static EditorState make_state(void) {
    EditorStateConfig cfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(cfg, g_schema);
    EditorStateConfig_set_doc(cfg, make_initial_doc());
    return EditorState_create(cfg);
}

static void update_meta(Node_3 doc) {
    StrBuf sb = { .pos = 0 };
    sb_str(&sb, g_last_op);
    int n = (int)Node_3_get_childCount(doc);
    sb_str(&sb, " · blocks="); sb_int(&sb, n);
    sb_str(&sb, " · outline=[");
    for (int i = 0; i < n; i++) {
        if (i > 0) sb_str(&sb, " · ");
        sb_js(&sb, NodeType_get_name(Node_3_get_type(Node_3_child(doc, i))));
    }
    sb_str(&sb, "] · totalSteps="); sb_int(&sb, g_total_steps);
    sb_str(&sb, " · nodeSize="); sb_int(&sb, (int)Node_3_get_nodeSize(doc));
    sb.buf[sb.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "meta").h }, sb.buf);
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    g_total_steps += (int)JsArray_Step_length(Transform_get_steps(tr));
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    update_meta(EditorState_get_doc(next));
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    int idx = *(const int*)ctx;
    EditorState st = EditorView_get_state(g_view);
    Node_3 doc = EditorState_get_doc(st);

    if (idx == 0) {
        Selection_2 sel = EditorState_get_selection(st);
        double sel_from = Selection_2_get_from(sel);
        double split_pos = -1.0;
        if (canSplit(doc, sel_from)) {
            split_pos = sel_from;
        } else {
            double pos = 0.0;
            int n = (int)Node_3_get_childCount(doc);
            for (int i = 0; i < n; i++) {
                Node_3 ch = Node_3_child(doc, i);
                if (split_pos < 0.0 && js_str_eq(NodeType_get_name(Node_3_get_type(ch)), "paragraph")) {
                    double len = Fragment_get_size(Node_3_get_content(ch));
                    if (len >= 8.0) {
                        double mid = pos + 1.0 + (double)((int)len / 2);
                        if (canSplit(doc, mid)) split_pos = mid;
                    }
                }
                pos += Node_3_get_nodeSize(ch);
            }
        }
        if (split_pos >= 0.0) {
            Transaction tr = EditorState_get_tr(st);
            Transform_split(tr, split_pos);
            g_last_op = "Split paragraph node";
            call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
        } else {
            g_last_op = "No splittable paragraph found";
            update_meta(doc);
        }
    } else if (idx == 1) {
        NodeType quote_type = Schema_nodes_get_at(Schema_get_nodes(g_schema), "blockquote");
        Selection_2 sel = EditorState_get_selection(st);
        ResolvedPos sf = Selection_2_get__from(sel);
        ResolvedPos stp = Selection_2_get__to(sel);
        NodeRange range = { 0 };
        if (ResolvedPos_get_depth(sf) == 1.0 && js_str_eq(NodeType_get_name(Node_3_get_type(ResolvedPos_get_parent(sf))), "paragraph")) {
            NodeRange r = ResolvedPos_blockRange(sf, stp);
            if (r.h != 0 && findWrapping(r, quote_type, (Attrs){ 0 }).h != 0) range = r;
        }
        if (range.h == 0) {
            double pos = 0.0;
            int n = (int)Node_3_get_childCount(doc);
            for (int i = 0; i < n; i++) {
                Node_3 ch = Node_3_child(doc, i);
                if (range.h == 0 && js_str_eq(NodeType_get_name(Node_3_get_type(ch)), "paragraph")) {
                    ResolvedPos rp = Node_3_resolve(doc, pos + 1.0);
                    NodeRange r = ResolvedPos_blockRange(rp);
                    if (r.h != 0 && findWrapping(r, quote_type, (Attrs){ 0 }).h != 0) range = r;
                }
                pos += Node_3_get_nodeSize(ch);
            }
        }
        if (range.h != 0) {
            JsArray_findWrapping_result wrappers = findWrapping(range, quote_type, (Attrs){ 0 });
            if (wrappers.h != 0) {
                Transaction tr = EditorState_get_tr(st);
                Transform_wrap(tr, range, (JsArray_canSplit_typesAfter){ wrappers.h });
                g_last_op = "Wrapped paragraph block in blockquote";
                call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
                return;
            }
        }
        g_last_op = "No top-level paragraph left to wrap";
        update_meta(doc);
    } else if (idx == 2) {
        double pos = 0.0;
        int n = (int)Node_3_get_childCount(doc);
        for (int i = 0; i < n; i++) {
            Node_3 ch = Node_3_child(doc, i);
            if (js_str_eq(NodeType_get_name(Node_3_get_type(ch)), "blockquote") && Node_3_get_childCount(ch) > 0.0) {
                ResolvedPos rp = Node_3_resolve(doc, pos + 2.0);
                NodeRange range = ResolvedPos_blockRange(rp);
                if (range.h != 0) {
                    JsOptionalDouble target = liftTarget(range);
                    if (target.has) {
                        Transaction tr = EditorState_get_tr(st);
                        Transform_lift(tr, range, target.value);
                        g_last_op = "Lifted blockquote child to root depth";
                        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
                        return;
                    }
                }
            }
            pos += Node_3_get_nodeSize(ch);
        }
        g_last_op = "No blockquote left to lift";
        update_meta(doc);
    } else if (idx == 3) {
        g_note_counter++;
        Node_3 hr = Schema_node(g_schema, "horizontal_rule", (Attrs){ 0 });
        StrBuf nb = { .pos = 0 };
        sb_str(&nb, "Appended structural section #"); sb_int(&nb, g_note_counter);
        sb_str(&nb, " at doc offset "); sb_int(&nb, (int)Fragment_get_size(Node_3_get_content(doc)));
        sb_str(&nb, ".");
        nb.buf[nb.pos] = 0;
        JsArray_Node_3 items = JsArray_Node_3_new();
        JsArray_Node_3_push(items, hr);
        JsArray_Node_3_push(items, p_node(nb.buf));
        Transaction tr = EditorState_get_tr(st);
        Transform_insert_with_Node_array(tr, Fragment_get_size(Node_3_get_content(doc)), items);
        g_last_op = "Inserted horizontal_rule + note";
        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
    } else if (idx == 4) {
        g_total_steps = 0;
        g_note_counter = 0;
        g_last_op = "Reset structural sample";
        EditorState fresh = make_state();
        EditorView_updateState(g_view, fresh);
        update_meta(EditorState_get_doc(fresh));
    }
}

__attribute__((export_name("start"))) void start(void) {
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, make_state());
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, 0));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new((dist_DOMNode){ mount.h }, props);
    js_retain(g_view);
    update_meta(EditorState_get_doc(EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
