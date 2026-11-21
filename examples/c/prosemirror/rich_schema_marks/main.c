#include <stdio.h>
#include "dom/dom.h"
#include "prosemirror/prosemirror.h"

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
static JsValue code_block_dom(void* ctx, Pm_Node n) {
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
static int g_dispatches = 0;
static const char* g_last_action = "Initialized schema with 7 node types & 3 marks";

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

    Pm_NodeSpec cblock = Pm_NodeSpec_create();
    Pm_NodeSpec_set_content(cblock, "text*");
    Pm_NodeSpec_set_marks(cblock, "");
    Pm_NodeSpec_set_group(cblock, "block");
    Pm_NodeSpec_set_code(cblock, true);
    Pm_NodeSpec_set_defining(cblock, true);
    Pm_NodeSpec_set_toDOM(cblock, Pm_Node_to_DOMOutputSpec_fn_from(code_block_dom, NULL));

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
    js_set_val(nodes, "code_block", cblock);
    js_set_val(nodes, "horizontal_rule", hr);
    js_set_val(nodes, "text", text);

    Pm_MarkSpec strong = Pm_MarkSpec_create();
    Pm_MarkSpec_set_toDOM(strong, Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));
    Pm_MarkSpec em = Pm_MarkSpec_create();
    Pm_MarkSpec_set_toDOM(em, Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"em"));
    Pm_MarkSpec code = Pm_MarkSpec_create();
    Pm_MarkSpec_set_toDOM(code, Pm_Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));

    Pm_Attrs marks = Pm_Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "em", em);
    js_set_val(marks, "code", code);

    Pm_SchemaSpec spec = Pm_SchemaSpec_create();
    js_set_val(spec, "nodes", nodes);
    js_set_val(spec, "marks", marks);
    return Pm_Schema_new(spec);
}

static Pm_Node t_node(const char* t) {
    return Pm_Schema_text(g_schema, t, (JsArray_Pm_Mark){ 0 });
}

static Pm_Node make_initial_doc(void) {
    Pm_Attrs h_attrs = Pm_Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Pm_Node h = Pm_Schema_node_with_string(g_schema, "heading", h_attrs, t_node("ProseMirror Document Schema"));

    Pm_Mark m_strong = Pm_Schema_mark(g_schema, "strong", (Pm_Attrs){ 0 });
    Pm_Mark m_em = Pm_Schema_mark(g_schema, "em", (Pm_Attrs){ 0 });
    Pm_Mark m_code = Pm_Schema_mark(g_schema, "code", (Pm_Attrs){ 0 });

    Pm_Node p1_items[7] = {
        t_node("Inline marks compose cleanly: "),
        Pm_Schema_text(g_schema, "bold", JsArray_Pm_Mark_from(&m_strong, 1)),
        t_node(", "),
        Pm_Schema_text(g_schema, "italic", JsArray_Pm_Mark_from(&m_em, 1)),
        t_node(", and "),
        Pm_Schema_text(g_schema, "inline_code", JsArray_Pm_Mark_from(&m_code, 1)),
        t_node(" spans.")
    };
    Pm_Node p_marks = Pm_Schema_node_with_string_and_Node_array(
        g_schema, "paragraph", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(p1_items, 7)
    );

    Pm_Node q_inner = Pm_Schema_node_with_string(
        g_schema, "paragraph", (Pm_Attrs){ 0 },
        t_node("Every document conforms to a strict schema enforced on every transaction.")
    );
    Pm_Node quote = Pm_Schema_node_with_string(g_schema, "blockquote", (Pm_Attrs){ 0 }, q_inner);

    Pm_Node code_block = Pm_Schema_node_with_string(
        g_schema, "code_block", (Pm_Attrs){ 0 },
        t_node("schema.node(\"paragraph\", null, [schema.text(\"typed\")])")
    );

    Pm_Node outro = Pm_Schema_node_with_string(
        g_schema, "paragraph", (Pm_Attrs){ 0 },
        t_node("Select any text in this editor and click a toolbar action to toggle marks or switch block types.")
    );

    Pm_Node blocks[5] = { h, p_marks, quote, code_block, outro };
    return Pm_Schema_node_with_string_and_Node_array(g_schema, "doc", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(blocks, 5));
}

static Pm_EditorState make_state(void) {
    Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create();
    Pm_EditorStateConfig_set_schema(cfg, g_schema);
    Pm_EditorStateConfig_set_doc(cfg, make_initial_doc());
    return Pm_EditorState_create(cfg);
}

static void target_range(Pm_EditorState st, double* out_from, double* out_to) {
    Pm_Selection sel = Pm_EditorState_get_selection(st);
    if (!Pm_Selection_get_empty(sel)) {
        *out_from = Pm_Selection_get_from(sel);
        *out_to = Pm_Selection_get_to(sel);
        return;
    }
    Pm_Node doc = Pm_EditorState_get_doc(st);
    if (Pm_Node_get_childCount(doc) >= 2.0) {
        Pm_Node c0 = Pm_Node_child(doc, 0);
        Pm_Node c1 = Pm_Node_child(doc, 1);
        double start_pos = Pm_Node_get_nodeSize(c0) + 1.0;
        double len = Pm_Fragment_get_size(Pm_Node_get_content(c1));
        if (len > 0.0) {
            *out_from = start_pos;
            *out_to = start_pos + (len > 12.0 ? 12.0 : len);
            return;
        }
    }
    double sz = Pm_Fragment_get_size(Pm_Node_get_content(doc));
    *out_from = 1.0;
    *out_to = sz > 2.0 ? 2.0 : sz;
}

static void update_meta(Pm_EditorState st) {
    js_scope_open();
    double f = 0, t = 0;
    target_range(st, &f, &t);
    Pm_Node doc = Pm_EditorState_get_doc(st);
    Pm_Schema_marks marks = Pm_Schema_get_marks(g_schema);
    bool has_b = Pm_Node_rangeHasMark_with_MarkType(doc, f, t, Pm_Schema_marks_get_at(marks, "strong"));
    bool has_e = Pm_Node_rangeHasMark_with_MarkType(doc, f, t, Pm_Schema_marks_get_at(marks, "em"));
    bool has_c = Pm_Node_rangeHasMark_with_MarkType(doc, f, t, Pm_Schema_marks_get_at(marks, "code"));

    HTMLElement_set_className(Document_getElementById(g_doc, "btn-0"), has_b ? "active" : "");
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-1"), has_e ? "active" : "");
    HTMLElement_set_className(Document_getElementById(g_doc, "btn-2"), has_c ? "active" : "");

    Pm_Selection sel = Pm_EditorState_get_selection(st);
    JsString bname = Pm_NodeType_get_name(Pm_Node_get_type(Pm_ResolvedPos_get_parent(Pm_Selection_get__from(sel))));
    char block_str[64];
    uint32_t blen = JsString_copy(bname, block_str, sizeof block_str - 1);
    block_str[blen < sizeof block_str ? blen : sizeof block_str - 1] = 0;

    char meta[256];
    snprintf(
        meta, sizeof meta,
        "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 range=[%d..%d] \xC2\xB7 block=%s \xC2\xB7 dispatches=%d",
        g_last_action, (int)Pm_Node_get_childCount(doc), (int)Pm_Node_get_nodeSize(doc),
        (int)f, (int)t, block_str, g_dispatches
    );
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    g_dispatches++;
    Pm_EditorState next = Pm_EditorState_apply(Pm_EditorView_get_state(g_view), tr);
    Pm_EditorView_updateState(g_view, next);
    update_meta(next);
    js_scope_close();
}

static void toggle_mark(const char* name, const char* added_msg, const char* removed_msg) {
    js_scope_open();
    Pm_EditorState st = Pm_EditorView_get_state(g_view);
    double f = 0, t = 0;
    target_range(st, &f, &t);
    if (t > f) {
        Pm_MarkType mt = Pm_Schema_marks_get_at(Pm_Schema_get_marks(g_schema), name);
        Pm_Transaction tr = Pm_EditorState_get_tr(st);
        if (Pm_Node_rangeHasMark_with_MarkType(Pm_EditorState_get_doc(st), f, t, mt)) {
            Pm_Transaction_removeMark_with_opt_MarkType(tr, f, t, mt);
            g_last_action = removed_msg;
        } else {
            Pm_Mark m = Pm_Schema_mark(g_schema, mt, (Pm_Attrs){ 0 });
            Pm_Transaction_addMark(tr, f, t, m);
            g_last_action = added_msg;
        }
        pm_call_Transaction_fn(Pm_EditorView_get_dispatch(g_view), tr);
    }
    js_scope_close();
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope_open();
    int idx = *(const int*)ctx;
    if (idx == 0) toggle_mark("strong", "Added bold mark", "Removed bold mark");
    else if (idx == 1) toggle_mark("em", "Added italic mark", "Removed italic mark");
    else if (idx == 2) toggle_mark("code", "Added code mark", "Removed code mark");
    else if (idx == 3) {
        Pm_EditorState st = Pm_EditorView_get_state(g_view);
        Pm_Selection sel = Pm_EditorState_get_selection(st);
        JsString pname = Pm_NodeType_get_name(Pm_Node_get_type(Pm_ResolvedPos_get_parent(Pm_Selection_get__from(sel))));
        Pm_Transaction tr = Pm_EditorState_get_tr(st);
        Pm_Schema_nodes nodes = Pm_Schema_get_nodes(g_schema);
        if (JsString_equals(pname, "heading")) {
            Pm_Transaction_setBlockType(tr, Pm_Selection_get_from(sel), Pm_Selection_get_to(sel), Pm_Schema_nodes_get_at(nodes, "paragraph"), (Pm_Attrs){ 0 });
            g_last_action = "Converted heading -> paragraph";
        } else {
            Pm_Attrs ha = Pm_Attrs_create();
            js_set_num(ha, "level", 2);
            Pm_Transaction_setBlockType(tr, Pm_Selection_get_from(sel), Pm_Selection_get_to(sel), Pm_Schema_nodes_get_at(nodes, "heading"), ha);
            g_last_action = "Converted block -> heading[level=2]";
        }
        pm_call_Transaction_fn(Pm_EditorView_get_dispatch(g_view), tr);
    } else if (idx == 4) {
        g_dispatches = 0;
        g_last_action = "Reset initial schema document";
        Pm_EditorState fresh = make_state();
        Pm_EditorView_updateState(g_view, fresh);
        update_meta(fresh);
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
    update_meta(Pm_EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
