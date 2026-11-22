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
static JsValue heading_dom(void* ctx, Pm_Node n) {
    (void)ctx;
    js_scope_open();
    int lvl = (int)js_to_number(js_get_prop(Pm_Node_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    js_scope_close();
    return res;
}
static JsValue tag_dom(void* ctx, Pm_Node n) { (void)n; return dom_hole((const char*)ctx); }
static JsValue hr_dom(void* ctx, Pm_Node n) {
    (void)ctx; (void)n;
    JsArray a = JsArray_new();
    js_push_str(a, "hr");
    return js_value(a);
}
static JsValue mark_dom(void* ctx, Pm_Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Document g_doc;
static Pm_Schema g_schema;
static Pm_EditorView g_view;
static int g_total_steps = 0;
static int g_note_counter = 0;
static const char* g_last_op = "Initialized document structure";

static Pm_Schema make_schema(void) {
    Pm_NodeSpec doc = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(doc, "block+");

    Pm_AttributeSpec lvl = Pm_AttributeSpec_create();
    Pm_AttributeSpec_set_default(lvl, js_number(2));
    Pm_MarkSpec_attrs hattrs = Pm_MarkSpec_attrs_create();
    Pm_MarkSpec_attrs_set_at(hattrs, "level", lvl);

    Pm_NodeSpec heading = Pm_NodeSpec_create();
    Pm_NodeSpec_set_attrs(heading, hattrs);
    Pm_NodeSpec_set_content(heading, "inline*");
    Pm_NodeSpec_set_group(heading, "block");
    Pm_NodeSpec_set_defining(heading, true);
    Pm_NodeSpec_set_toDOM(heading, Pm_Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));

    Pm_NodeSpec para = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(para, "inline*");
    Pm_NodeSpec_set_group(para, "block");
    Pm_NodeSpec_set_toDOM(para, Pm_Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"p"));

    Pm_NodeSpec quote = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(quote, "block+");
    Pm_NodeSpec_set_group(quote, "block");
    Pm_NodeSpec_set_defining(quote, true);
    Pm_NodeSpec_set_toDOM(quote, Pm_Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"blockquote"));

    Pm_NodeSpec ulist = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(ulist, "list_item+");
    Pm_NodeSpec_set_group(ulist, "block");
    Pm_NodeSpec_set_toDOM(ulist, Pm_Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"ul"));

    Pm_NodeSpec li = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(li, "paragraph block*");
    Pm_NodeSpec_set_defining(li, true);
    Pm_NodeSpec_set_toDOM(li, Pm_Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"li"));

    Pm_NodeSpec hr = Pm_NodeSpec_create();
    Pm_NodeSpec_set_group(hr, "block");
    Pm_NodeSpec_set_atom(hr, true);
    Pm_NodeSpec_set_toDOM(hr, Pm_Node_to_DOMOutputSpec_fn_from(hr_dom, NULL));

    Pm_NodeSpec text = Pm_NodeSpec_create();
    Pm_NodeSpec_set_group(text, "inline");

    Pm_Attrs nodes = Pm_Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "blockquote", quote);
    js_set_val(nodes, "bullet_list", ulist);
    js_set_val(nodes, "list_item", li);
    js_set_val(nodes, "horizontal_rule", hr);
    js_set_val(nodes, "text", text);

    Pm_MarkSpec strong = Pm_MarkSpec_create();
    Pm_MarkSpec_set_toDOM(strong, Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    Pm_MarkSpec code = Pm_MarkSpec_create();
    Pm_MarkSpec_set_toDOM(code, Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));
    Pm_Attrs marks = Pm_Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "code", code);

    Pm_SchemaSpec spec = Pm_SchemaSpec_create();
    js_set_val(spec, "nodes", nodes);
    js_set_val(spec, "marks", marks);
    return Pm_Schema_new(spec);
}

static Pm_Node t_node(const char* t) {
    return Pm_Schema_text(g_schema, t, (JsArray_Pm_Mark){ 0 });
}
static Pm_Node p_node(const char* t) {
    return Pm_Schema_node_with_string(g_schema, "paragraph", (Pm_Attrs){ 0 }, t_node(t));
}
static Pm_Node li_node(const char* t) {
    return Pm_Schema_node_with_string(g_schema, "list_item", (Pm_Attrs){ 0 }, p_node(t));
}

static Pm_Node make_initial_doc(void) {
    Pm_Attrs h_attrs = Pm_Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Pm_Node h = Pm_Schema_node_with_string(g_schema, "heading", h_attrs, t_node("Structural Transform Pipeline"));
    Pm_Node p1 = p_node("ProseMirror transforms express structural edits as invertible steps.");
    Pm_Node p2 = p_node("Wrap blocks in quotes, lift nested content, or split paragraphs cleanly.");
    Pm_Node quote = Pm_Schema_node_with_string(
        g_schema, "blockquote", (Pm_Attrs){ 0 }, p_node("Blockquote content can be lifted back to top-level document depth.")
    );
    Pm_Node lis[3] = {
        li_node("Split at cursor position"),
        li_node("Wrap range with blockquote"),
        li_node("Lift nested block to root")
    };
    Pm_Node ulist = Pm_Schema_node_with_string_and_Node_array(g_schema, "bullet_list", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(lis, 3));

    Pm_Node blocks[5] = { h, p1, p2, quote, ulist };
    return Pm_Schema_node_with_string_and_Node_array(g_schema, "doc", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(blocks, 5));
}

static Pm_EditorState make_state(void) {
    Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create();
    Pm_EditorStateConfig_set_schema(cfg, g_schema);
    Pm_EditorStateConfig_set_doc(cfg, make_initial_doc());
    return Pm_EditorState_create(cfg);
}

static void update_meta(Pm_Node doc) {
    js_scope_open();
    char meta[512];
    OutBuf out = { meta, meta + sizeof meta };
    int n = (int)Pm_Node_get_childCount(doc);
    out_fmt(&out, "%s \xC2\xB7 blocks=%d \xC2\xB7 outline=[", g_last_op, n);
    for (int i = 0; i < n; i++) {
        if (i > 0) out_fmt(&out, " \xC2\xB7 ");
        out_js(&out, Pm_NodeType_get_name(Pm_Node_get_type(Pm_Node_child(doc, i))));
    }
    out_fmt(&out, "] \xC2\xB7 totalSteps=%d \xC2\xB7 nodeSize=%d", g_total_steps, (int)Pm_Node_get_nodeSize(doc));
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    g_total_steps += (int)JsArray_Pm_Step_length(Pm_Transaction_get_steps(tr));
    Pm_EditorState next = Pm_EditorState_apply(Pm_EditorView_get_state(g_view), tr);
    Pm_EditorView_updateState(g_view, next);
    update_meta(Pm_EditorState_get_doc(next));
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    Pm_EditorState st = Pm_EditorView_get_state(g_view);
    Pm_Node doc = Pm_EditorState_get_doc(st);

    if (idx == 0) {
        Pm_Selection sel = Pm_EditorState_get_selection(st);
        double sel_from = Pm_Selection_get_from(sel);
        double split_pos = -1.0;
        if (pm_canSplit(doc, sel_from)) {
            split_pos = sel_from;
        } else {
            double pos = 0.0;
            int n = (int)Pm_Node_get_childCount(doc);
            for (int i = 0; i < n; i++) {
                Pm_Node ch = Pm_Node_child(doc, i);
                if (split_pos < 0.0 && JsString_equals(Pm_NodeType_get_name(Pm_Node_get_type(ch)), "paragraph")) {
                    double len = Pm_Fragment_get_size(Pm_Node_get_content(ch));
                    if (len >= 8.0) {
                        double mid = pos + 1.0 + (double)((int)len / 2);
                        if (pm_canSplit(doc, mid)) split_pos = mid;
                    }
                }
                pos += Pm_Node_get_nodeSize(ch);
            }
        }
        if (split_pos >= 0.0) {
            Pm_Transaction tr = Pm_EditorState_get_tr(st);
            Pm_Transaction_split(tr, split_pos);
            g_last_op = "Split paragraph node";
            Pm_EditorView_dispatch(g_view, tr);
        } else {
            g_last_op = "No splittable paragraph found";
            update_meta(doc);
        }
    } else if (idx == 1) {
        Pm_NodeType quote_type = Pm_Schema_nodes_get_at(Pm_Schema_get_nodes(g_schema), "blockquote");
        Pm_Selection sel = Pm_EditorState_get_selection(st);
        Pm_ResolvedPos sf = Pm_Selection_get__from(sel);
        Pm_ResolvedPos stp = Pm_Selection_get__to(sel);
        Pm_NodeRange range = { 0 };
        if (Pm_ResolvedPos_get_depth(sf) == 1.0 && JsString_equals(Pm_NodeType_get_name(Pm_Node_get_type(Pm_ResolvedPos_get_parent(sf))), "paragraph")) {
            Pm_NodeRange r = Pm_ResolvedPos_blockRange(sf, stp);
            if (r.h != 0 && pm_findWrapping(r, quote_type, (Pm_Attrs){ 0 }).h != 0) range = r;
        }
        if (range.h == 0) {
            double pos = 0.0;
            int n = (int)Pm_Node_get_childCount(doc);
            for (int i = 0; i < n; i++) {
                Pm_Node ch = Pm_Node_child(doc, i);
                if (range.h == 0 && JsString_equals(Pm_NodeType_get_name(Pm_Node_get_type(ch)), "paragraph")) {
                    Pm_ResolvedPos rp = Pm_Node_resolve(doc, pos + 1.0);
                    Pm_NodeRange r = Pm_ResolvedPos_blockRange(rp);
                    if (r.h != 0 && pm_findWrapping(r, quote_type, (Pm_Attrs){ 0 }).h != 0) range = r;
                }
                pos += Pm_Node_get_nodeSize(ch);
            }
        }
        bool wrapped = false;
        if (range.h != 0) {
            JsArray_Pm_canSplit_typesAfter wrappers = pm_findWrapping(range, quote_type, (Pm_Attrs){ 0 });
            if (wrappers.h != 0) {
                Pm_Transaction tr = Pm_EditorState_get_tr(st);
                Pm_Transaction_wrap(tr, range, wrappers);
                g_last_op = "Wrapped paragraph block in blockquote";
                Pm_EditorView_dispatch(g_view, tr);
                wrapped = true;
            }
        }
        if (!wrapped) {
            g_last_op = "No top-level paragraph left to wrap";
            update_meta(doc);
        }
    } else if (idx == 2) {
        double pos = 0.0;
        int n = (int)Pm_Node_get_childCount(doc);
        bool lifted = false;
        for (int i = 0; i < n && !lifted; i++) {
            Pm_Node ch = Pm_Node_child(doc, i);
            if (JsString_equals(Pm_NodeType_get_name(Pm_Node_get_type(ch)), "blockquote") && Pm_Node_get_childCount(ch) > 0.0) {
                Pm_ResolvedPos rp = Pm_Node_resolve(doc, pos + 2.0);
                Pm_NodeRange range = Pm_ResolvedPos_blockRange(rp);
                if (range.h != 0) {
                    JsOptionalDouble target = pm_liftTarget(range);
                    if (target.has) {
                        Pm_Transaction tr = Pm_EditorState_get_tr(st);
                        Pm_Transaction_lift(tr, range, target.value);
                        g_last_op = "Lifted blockquote child to root depth";
                        Pm_EditorView_dispatch(g_view, tr);
                        lifted = true;
                    }
                }
            }
            pos += Pm_Node_get_nodeSize(ch);
        }
        if (!lifted) {
            g_last_op = "No blockquote left to lift";
            update_meta(doc);
        }
    } else if (idx == 3) {
        g_note_counter++;
        Pm_Node hr = Pm_Schema_node(g_schema, "horizontal_rule", (Pm_Attrs){ 0 });
        char nbuf[96];
        double doc_end = Pm_Fragment_get_size(Pm_Node_get_content(doc));
        snprintf(nbuf, sizeof nbuf, "Appended structural section #%d at doc offset %d.", g_note_counter, (int)doc_end);
        Pm_Node items[2] = { hr, p_node(nbuf) };
        Pm_Transaction tr = Pm_EditorState_get_tr(st);
        Pm_Transaction_insert(tr, doc_end, JsArray_Pm_Node_from(items, 2));
        g_last_op = "Inserted horizontal_rule + note";
        Pm_EditorView_dispatch(g_view, tr);
    } else if (idx == 4) {
        g_total_steps = 0;
        g_note_counter = 0;
        g_last_op = "Reset structural sample";
        Pm_EditorState fresh = make_state();
        Pm_EditorView_updateState(g_view, fresh);
        update_meta(Pm_EditorState_get_doc(fresh));
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    Pm_DirectEditorProps props = Pm_DirectEditorProps_create();
    Pm_DirectEditorProps_set_state(props, make_state());
    Pm_DirectEditorProps_set_dispatchTransaction(props, Pm_Transaction_fn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    update_meta(Pm_EditorState_get_doc(Pm_EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
