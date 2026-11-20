#include <stdarg.h>
#include <stdio.h>
#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

typedef struct { char* p; char* end; } OutBuf;
__attribute__((format(printf, 2, 3)))
static void out_fmt(OutBuf* o, const char* fmt, ...) {
    if (o->p >= o->end) return;
    va_list ap;
    va_start(ap, fmt);
    int n = vsnprintf(o->p, (size_t)(o->end - o->p), fmt, ap);
    va_end(ap);
    if (n > 0) {
        ptrdiff_t rem = o->end - o->p - 1;
        o->p += (n <= rem) ? n : rem;
    }
}
static void out_js(OutBuf* o, JsString js) {
    if (o->p >= o->end - 1) return;
    uint32_t cap = (uint32_t)(o->end - o->p - 1);
    uint32_t len = JsString_copy(js, o->p, cap);
    o->p += (len < cap) ? len : cap;
    *o->p = 0;
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
static JsValue tag_dom(void* ctx, Node_3 n) { (void)n; return dom_hole((const char*)ctx); }
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

    NodeSpec ulist = NodeSpec_create();
    NodeSpec_set_content(ulist, "list_item+");
    NodeSpec_set_group(ulist, "block");
    NodeSpec_set_toDOM(ulist, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"ul"));

    NodeSpec li = NodeSpec_create();
    NodeSpec_set_content(li, "paragraph block*");
    NodeSpec_set_defining(li, true);
    NodeSpec_set_toDOM(li, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"li"));

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
    js_set_val(nodes, "bullet_list", ulist);
    js_set_val(nodes, "list_item", li);
    js_set_val(nodes, "horizontal_rule", hr);
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
static Node_3 li_node(const char* t) {
    return Schema_node_with_string(g_schema, "list_item", (Attrs){ 0 }, p_node(t));
}

static Node_3 make_initial_doc(void) {
    Attrs h_attrs = Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Node_3 h = Schema_node_with_string(g_schema, "heading", h_attrs, t_node("Structural Transform Pipeline"));
    Node_3 p1 = p_node("ProseMirror transforms express structural edits as invertible steps.");
    Node_3 p2 = p_node("Wrap blocks in quotes, lift nested content, or split paragraphs cleanly.");
    Node_3 quote = Schema_node_with_string(
        g_schema, "blockquote", (Attrs){ 0 }, p_node("Blockquote content can be lifted back to top-level document depth.")
    );
    Node_3 lis[3] = {
        li_node("Split at cursor position"),
        li_node("Wrap range with blockquote"),
        li_node("Lift nested block to root")
    };
    Node_3 ulist = Schema_node_with_string_and_Node_array(g_schema, "bullet_list", (Attrs){ 0 }, JsArray_Node_3_from(lis, 3));

    Node_3 blocks[5] = { h, p1, p2, quote, ulist };
    return Schema_node_with_string_and_Node_array(g_schema, "doc", (Attrs){ 0 }, JsArray_Node_3_from(blocks, 5));
}

static EditorState make_state(void) {
    EditorStateConfig cfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(cfg, g_schema);
    EditorStateConfig_set_doc(cfg, make_initial_doc());
    return EditorState_create(cfg);
}

static void update_meta(Node_3 doc) {
    js_scope_open();
    char meta[512];
    OutBuf out = { meta, meta + sizeof meta };
    int n = (int)Node_3_get_childCount(doc);
    out_fmt(&out, "%s \xC2\xB7 blocks=%d \xC2\xB7 outline=[", g_last_op, n);
    for (int i = 0; i < n; i++) {
        if (i > 0) out_fmt(&out, " \xC2\xB7 ");
        out_js(&out, NodeType_get_name(Node_3_get_type(Node_3_child(doc, i))));
    }
    out_fmt(&out, "] \xC2\xB7 totalSteps=%d \xC2\xB7 nodeSize=%d", g_total_steps, (int)Node_3_get_nodeSize(doc));
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    g_total_steps += (int)JsArray_Step_length(Transaction_get_steps(tr));
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    update_meta(EditorState_get_doc(next));
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
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
                if (split_pos < 0.0 && JsString_equals(NodeType_get_name(Node_3_get_type(ch)), "paragraph")) {
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
            Transaction_split(tr, split_pos);
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
        if (ResolvedPos_get_depth(sf) == 1.0 && JsString_equals(NodeType_get_name(Node_3_get_type(ResolvedPos_get_parent(sf))), "paragraph")) {
            NodeRange r = ResolvedPos_blockRange(sf, stp);
            if (r.h != 0 && findWrapping(r, quote_type, (Attrs){ 0 }).h != 0) range = r;
        }
        if (range.h == 0) {
            double pos = 0.0;
            int n = (int)Node_3_get_childCount(doc);
            for (int i = 0; i < n; i++) {
                Node_3 ch = Node_3_child(doc, i);
                if (range.h == 0 && JsString_equals(NodeType_get_name(Node_3_get_type(ch)), "paragraph")) {
                    ResolvedPos rp = Node_3_resolve(doc, pos + 1.0);
                    NodeRange r = ResolvedPos_blockRange(rp);
                    if (r.h != 0 && findWrapping(r, quote_type, (Attrs){ 0 }).h != 0) range = r;
                }
                pos += Node_3_get_nodeSize(ch);
            }
        }
        bool wrapped = false;
        if (range.h != 0) {
            JsArray_canSplit_typesAfter wrappers = findWrapping(range, quote_type, (Attrs){ 0 });
            if (wrappers.h != 0) {
                Transaction tr = EditorState_get_tr(st);
                Transaction_wrap(tr, range, wrappers);
                g_last_op = "Wrapped paragraph block in blockquote";
                call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
                wrapped = true;
            }
        }
        if (!wrapped) {
            g_last_op = "No top-level paragraph left to wrap";
            update_meta(doc);
        }
    } else if (idx == 2) {
        double pos = 0.0;
        int n = (int)Node_3_get_childCount(doc);
        bool lifted = false;
        for (int i = 0; i < n && !lifted; i++) {
            Node_3 ch = Node_3_child(doc, i);
            if (JsString_equals(NodeType_get_name(Node_3_get_type(ch)), "blockquote") && Node_3_get_childCount(ch) > 0.0) {
                ResolvedPos rp = Node_3_resolve(doc, pos + 2.0);
                NodeRange range = ResolvedPos_blockRange(rp);
                if (range.h != 0) {
                    JsOptionalDouble target = liftTarget(range);
                    if (target.has) {
                        Transaction tr = EditorState_get_tr(st);
                        Transaction_lift(tr, range, target.value);
                        g_last_op = "Lifted blockquote child to root depth";
                        call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
                        lifted = true;
                    }
                }
            }
            pos += Node_3_get_nodeSize(ch);
        }
        if (!lifted) {
            g_last_op = "No blockquote left to lift";
            update_meta(doc);
        }
    } else if (idx == 3) {
        g_note_counter++;
        Node_3 hr = Schema_node(g_schema, "horizontal_rule", (Attrs){ 0 });
        char nbuf[96];
        double doc_end = Fragment_get_size(Node_3_get_content(doc));
        snprintf(nbuf, sizeof nbuf, "Appended structural section #%d at doc offset %d.", g_note_counter, (int)doc_end);
        Node_3 items[2] = { hr, p_node(nbuf) };
        Transaction tr = EditorState_get_tr(st);
        Transaction_insert(tr, doc_end, JsArray_Node_3_from(items, 2));
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
    update_meta(EditorState_get_doc(EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
