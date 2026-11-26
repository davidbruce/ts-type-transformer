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
    js_scope();
    int lvl = (int)js_to_number(js_get_prop(Pm_Node_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    return res;
}
static JsValue tag_dom(void* ctx, Pm_Node n) { (void)n; return dom_hole((const char*)ctx); }
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
    Pm_NodeSpec doc = Pm_NodeSpec_create(.content = "block+");

    Pm_AttributeSpec lvl = Pm_AttributeSpec_create(.default_ = js_number(2));
    Pm_MarkSpec_attrs hattrs = Pm_MarkSpec_attrs_create();
    Pm_MarkSpec_attrs_set_at(hattrs, "level", lvl);

    Pm_NodeSpec heading = Pm_NodeSpec_create(.attrs = hattrs, .content = "inline*", .group = "block", .defining = true, .toDOM = Pm_NodeSpecToDOMFn_from(heading_dom, NULL));

    Pm_NodeSpec para = Pm_NodeSpec_create(.content = "inline*", .group = "block", .toDOM = Pm_NodeSpecToDOMFn_from(tag_dom, (void*)"p"));

    Pm_NodeSpec quote = Pm_NodeSpec_create(.content = "block+", .group = "block", .defining = true, .toDOM = Pm_NodeSpecToDOMFn_from(tag_dom, (void*)"blockquote"));

    Pm_NodeSpec cblock = Pm_NodeSpec_create(.content = "text*", .marks = "", .group = "block", .code = true, .defining = true, .toDOM = Pm_NodeSpecToDOMFn_from(code_block_dom, NULL));

    Pm_NodeSpec hr = Pm_NodeSpec_create(.group = "block", .atom = true, .toDOM = Pm_NodeSpecToDOMFn_from(hr_dom, NULL));

    Pm_NodeSpec text = Pm_NodeSpec_create(.group = "inline");

    Pm_SchemaSpec_nodes nodes = Pm_SchemaSpec_nodes_create();
    Pm_SchemaSpec_nodes_set_at(nodes, "doc", doc);
    Pm_SchemaSpec_nodes_set_at(nodes, "heading", heading);
    Pm_SchemaSpec_nodes_set_at(nodes, "paragraph", para);
    Pm_SchemaSpec_nodes_set_at(nodes, "blockquote", quote);
    Pm_SchemaSpec_nodes_set_at(nodes, "code_block", cblock);
    Pm_SchemaSpec_nodes_set_at(nodes, "horizontal_rule", hr);
    Pm_SchemaSpec_nodes_set_at(nodes, "text", text);

    Pm_MarkSpec strong = Pm_MarkSpec_create(.toDOM = Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"strong"));
    Pm_MarkSpec em = Pm_MarkSpec_create(.toDOM = Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"em"));
    Pm_MarkSpec code = Pm_MarkSpec_create(.toDOM = Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"code"));

    Pm_SchemaSpec_marks marks = Pm_SchemaSpec_marks_create();
    Pm_SchemaSpec_marks_set_at(marks, "strong", strong);
    Pm_SchemaSpec_marks_set_at(marks, "em", em);
    Pm_SchemaSpec_marks_set_at(marks, "code", code);

    return Pm_Schema_new(Pm_SchemaSpec_create(.nodes = nodes, .marks = marks));
}

static Pm_Node t_node(const char* t) {
    return Pm_Schema_text(g_schema, t);
}

static Pm_Node make_initial_doc(void) {
    Pm_Attrs h_attrs = Pm_Attrs_create();
    js_set_num(h_attrs, "level", 2);
    Pm_Node h = Pm_Schema_node(g_schema, "heading", h_attrs, t_node("ProseMirror Document Schema"));

    Pm_Mark m_strong = Pm_Schema_mark(g_schema, "strong");
    Pm_Mark m_em = Pm_Schema_mark(g_schema, "em");
    Pm_Mark m_code = Pm_Schema_mark(g_schema, "code");

    Pm_Node p1_items[7] = {
        t_node("Inline marks compose cleanly: "),
        Pm_Schema_text(g_schema, "bold", JsArray_Pm_Mark_from(&m_strong, 1)),
        t_node(", "),
        Pm_Schema_text(g_schema, "italic", JsArray_Pm_Mark_from(&m_em, 1)),
        t_node(", and "),
        Pm_Schema_text(g_schema, "inline_code", JsArray_Pm_Mark_from(&m_code, 1)),
        t_node(" spans.")
    };
    Pm_Node p_marks = Pm_Schema_node(
        g_schema, "paragraph", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(p1_items, 7)
    );

    Pm_Node q_inner = Pm_Schema_node(
        g_schema, "paragraph", (Pm_Attrs){ 0 },
        t_node("Every document conforms to a strict schema enforced on every transaction.")
    );
    Pm_Node quote = Pm_Schema_node(g_schema, "blockquote", (Pm_Attrs){ 0 }, q_inner);

    Pm_Node code_block = Pm_Schema_node(
        g_schema, "code_block", (Pm_Attrs){ 0 },
        t_node("schema.node(\"paragraph\", null, [schema.text(\"typed\")])")
    );

    Pm_Node outro = Pm_Schema_node(
        g_schema, "paragraph", (Pm_Attrs){ 0 },
        t_node("Select any text in this editor and click a toolbar action to toggle marks or switch block types.")
    );

    Pm_Node blocks[5] = { h, p_marks, quote, code_block, outro };
    return Pm_Schema_node(g_schema, "doc", (Pm_Attrs){ 0 }, JsArray_Pm_Node_from(blocks, 5));
}

static Pm_EditorState make_state(void) {
    Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = make_initial_doc());
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
    js_scope();
    double f = 0, t = 0;
    target_range(st, &f, &t);
    Pm_Node doc = Pm_EditorState_get_doc(st);
    Pm_Schema_marks marks = Pm_Schema_get_marks(g_schema);
    bool has_b = Pm_Node_rangeHasMark(doc, f, t, Pm_Schema_marks_get_at(marks, "strong"));
    bool has_e = Pm_Node_rangeHasMark(doc, f, t, Pm_Schema_marks_get_at(marks, "em"));
    bool has_c = Pm_Node_rangeHasMark(doc, f, t, Pm_Schema_marks_get_at(marks, "code"));

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
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope();
    g_dispatches++;
    Pm_EditorState next = Pm_EditorState_apply(Pm_EditorView_get_state(g_view), tr);
    Pm_EditorView_updateState(g_view, next);
    update_meta(next);
}

static void toggle_mark(const char* name, const char* added_msg, const char* removed_msg) {
    js_scope();
    Pm_EditorState st = Pm_EditorView_get_state(g_view);
    double f = 0, t = 0;
    target_range(st, &f, &t);
    if (t > f) {
        Pm_MarkType mt = Pm_Schema_marks_get_at(Pm_Schema_get_marks(g_schema), name);
        Pm_Transaction tr = Pm_EditorState_get_tr(st);
        if (Pm_Node_rangeHasMark(Pm_EditorState_get_doc(st), f, t, mt)) {
            Pm_Transaction_removeMark(tr, f, t, mt);
            g_last_action = removed_msg;
        } else {
            Pm_Mark m = Pm_Schema_mark(g_schema, mt);
            Pm_Transaction_addMark(tr, f, t, m);
            g_last_action = added_msg;
        }
        Pm_EditorView_dispatch(g_view, tr);
    }
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
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
            Pm_Transaction_setBlockType(tr, Pm_Selection_get_from(sel), Pm_Selection_get_to(sel), Pm_Schema_nodes_get_at(nodes, "paragraph"));
            g_last_action = "Converted heading -> paragraph";
        } else {
            Pm_Attrs ha = Pm_Attrs_create();
            js_set_num(ha, "level", 2);
            Pm_Transaction_setBlockType(tr, Pm_Selection_get_from(sel), Pm_Selection_get_to(sel), Pm_Schema_nodes_get_at(nodes, "heading"), ha);
            g_last_action = "Converted block -> heading[level=2]";
        }
        Pm_EditorView_dispatch(g_view, tr);
    } else if (idx == 4) {
        g_dispatches = 0;
        g_last_action = "Reset initial schema document";
        Pm_EditorState fresh = make_state();
        Pm_EditorView_updateState(g_view, fresh);
        update_meta(fresh);
    }
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);

    Pm_DirectEditorProps props = Pm_DirectEditorProps_create(.state = make_state(), .dispatchTransaction = Pm_DispatchTransactionFn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    update_meta(Pm_EditorView_get_state(g_view));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
