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
static JsValue mark_dom(void* ctx, Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static TagParseRule make_tag_rule(const char* tag, int level) {
    TagParseRule r = TagParseRule_create();
    TagParseRule_set_tag(r, tag);
    if (level > 0) {
        Attrs a = Attrs_create();
        js_set_num(a, "level", (double)level);
        TagParseRule_set_attrs(r, a);
    }
    return r;
}

static Document g_doc;
static Schema g_schema;
static DOMParser_2 g_parser;
static DOMSerializer g_serializer;
static EditorView g_view;
static const char* g_status = "JSON Node.eq() pending";

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
    TagParseRule hrules[2] = { make_tag_rule("h1", 1), make_tag_rule("h2", 2) };
    NodeSpec_set_parseDOM(heading, JsArray_TagParseRule_from(hrules, 2));
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, NULL));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    TagParseRule p_rule = make_tag_rule("p", 0);
    NodeSpec_set_parseDOM(para, JsArray_TagParseRule_from(&p_rule, 1));
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"p"));

    NodeSpec quote = NodeSpec_create();
    NodeSpec_set_content(quote, "block+");
    NodeSpec_set_group(quote, "block");
    NodeSpec_set_defining(quote, true);
    TagParseRule q_rule = make_tag_rule("blockquote", 0);
    NodeSpec_set_parseDOM(quote, JsArray_TagParseRule_from(&q_rule, 1));
    NodeSpec_set_toDOM(quote, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"blockquote"));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    js_set_val(nodes, "doc", doc);
    js_set_val(nodes, "heading", heading);
    js_set_val(nodes, "paragraph", para);
    js_set_val(nodes, "blockquote", quote);
    js_set_val(nodes, "text", text);

    MarkSpec strong = MarkSpec_create();
    ParseRule srule = ParseRule_of(make_tag_rule("strong", 0));
    MarkSpec_set_parseDOM(strong, JsArray_ParseRule_from(&srule, 1));
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));

    MarkSpec em = MarkSpec_create();
    ParseRule erule = ParseRule_of(make_tag_rule("em", 0));
    MarkSpec_set_parseDOM(em, JsArray_ParseRule_from(&erule, 1));
    MarkSpec_set_toDOM(em, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"em"));

    MarkSpec code = MarkSpec_create();
    ParseRule crule = ParseRule_of(make_tag_rule("code", 0));
    MarkSpec_set_parseDOM(code, JsArray_ParseRule_from(&crule, 1));
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));

    Attrs marks = Attrs_create();
    js_set_val(marks, "strong", strong);
    js_set_val(marks, "em", em);
    js_set_val(marks, "code", code);

    SchemaSpec spec = SchemaSpec_create();
    js_set_val(spec, "nodes", nodes);
    js_set_val(spec, "marks", marks);
    return Schema_new(spec);
}

static const char* SPEC_HTML =
    "<h2>DOMParser &amp; JSON Serialization</h2>"
    "<p>ProseMirror converts <strong>HTML</strong> via <code>DOMParser</code> and renders back through <em>DOMSerializer</em>.</p>"
    "<blockquote><p>Schema parseDOM rules keep DOM and JSON representations aligned.</p></blockquote>"
    "<p>Trigger any action above to inspect live DOMSerializer HTML and Node.eq() round-trips.</p>";

static const char* CHANGELOG_HTML =
    "<h2>Release 1.4 Notes</h2>"
    "<p>Added <strong>typed bindings</strong> for <code>DOMParser</code> and <em>DOMSerializer</em>.</p>"
    "<blockquote><p>All round-trips verified with Node.eq().</p></blockquote>"
    "<p>Edit directly above to watch HTML output update live.</p>";

static Node_3 parse_html(const char* html) {
    HTMLElement div = Document_createElement(g_doc, "div");
    HTMLElement_set_innerHTML(div, html);
    return DOMParser_2_parse(g_parser, (Node_2){ div.h });
}

static void refresh_output(Node_3 doc) {
    js_scope_open();
    HTMLElement div = Document_createElement(g_doc, "div");
    DOMSerializer_serializeFragment(g_serializer, Node_3_get_content(doc), (DOMSerializer_serializeFragment_options){ 0 }, div);
    JsString html_js = HTMLElement_get_innerHTML(div);
    int html_len = (int)JsString_len(html_js);

    static char out_buf[16384];
    OutBuf out = { out_buf, out_buf + sizeof out_buf };
    out_fmt(&out, "SERIALIZED HTML (DOMSerializer):\n");
    out_js(&out, html_js);
    int nb = (int)Node_3_get_childCount(doc);
    out_fmt(&out, "\n\nDOCUMENT AST & JSON ROUND-TRIP:\ndoc(blocks=%d, nodeSize=%d)\n",
            nb, (int)Node_3_get_nodeSize(doc));
    for (int i = 0; i < nb; i++) {
        Node_3 b = Node_3_child(doc, i);
        JsString tname = NodeType_get_name(Node_3_get_type(b));
        out_fmt(&out, "  [%d] ", i);
        if (JsString_equals(tname, "heading")) {
            out_fmt(&out, "heading[level=%d]", (int)js_to_number(js_get_prop(Node_3_get_attrs(b), "level")));
        } else {
            out_js(&out, tname);
        }
        out_fmt(&out, " (%d bytes): ", (int)Node_3_get_nodeSize(b));
        out_js(&out, Node_3_get_textContent(b));
        out_fmt(&out, "\n");
    }
    out_fmt(&out, "\nROUND-TRIP STATUS: %s", g_status);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "output"), out_buf);

    char meta[256];
    snprintf(meta, sizeof meta, "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 htmlBytes=%d",
             g_status, nb, (int)Node_3_get_nodeSize(doc), html_len);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
    js_scope_close();
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope_open();
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    refresh_output(EditorState_get_doc(next));
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
        g_status = "Parsed Spec HTML via DOMParser";
        EditorStateConfig cfg = EditorStateConfig_create();
        EditorStateConfig_set_schema(cfg, g_schema);
        EditorStateConfig_set_doc(cfg, parse_html(SPEC_HTML));
        EditorState next = EditorState_create(cfg);
        EditorView_updateState(g_view, next);
        refresh_output(EditorState_get_doc(next));
    } else if (idx == 1) {
        g_status = "Parsed Changelog HTML via DOMParser";
        EditorStateConfig cfg = EditorStateConfig_create();
        EditorStateConfig_set_schema(cfg, g_schema);
        EditorStateConfig_set_doc(cfg, parse_html(CHANGELOG_HTML));
        EditorState next = EditorState_create(cfg);
        EditorView_updateState(g_view, next);
        refresh_output(EditorState_get_doc(next));
    } else if (idx == 2) {
        Node_3 rebuilt = Node_3_fromJSON(g_schema, Node_3_toJSON(doc));
        bool ok = Node_3_eq(doc, rebuilt);
        g_status = ok ? "JSON toJSON->fromJSON eq() = true" : "JSON toJSON->fromJSON eq() = false";
        refresh_output(doc);
    } else if (idx == 3) {
        if (Node_3_get_childCount(doc) >= 2.0) {
            double insert_pos = Node_3_get_nodeSize(Node_3_child(doc, 0)) + Node_3_get_nodeSize(Node_3_child(doc, 1)) - 1.0;
            Mark code_mark = Schema_mark(g_schema, "code", (Attrs){ 0 });
            Node_3 span = Schema_text(g_schema, " roundtrip()", JsArray_Mark_from(&code_mark, 1));
            Transaction tr = EditorState_get_tr(st);
            Transaction_insert(tr, insert_pos, span);
            g_status = "Inserted code span into paragraph";
            call_Transaction_fn(EditorView_get_dispatch(g_view), tr);
        }
    } else if (idx == 4) {
        HTMLElement div = Document_createElement(g_doc, "div");
        DOMSerializer_serializeFragment(g_serializer, Node_3_get_content(doc), (DOMSerializer_serializeFragment_options){ 0 }, div);
        Node_3 reparsed = DOMParser_2_parse(g_parser, (Node_2){ div.h });
        bool ok = Node_3_eq(doc, reparsed);
        g_status = ok ? "DOM serializeFragment->parse eq() = true" : "DOM serializeFragment->parse eq() = false";
        EditorStateConfig cfg = EditorStateConfig_create();
        EditorStateConfig_set_schema(cfg, g_schema);
        EditorStateConfig_set_doc(cfg, reparsed);
        EditorState next = EditorState_create(cfg);
        EditorView_updateState(g_view, next);
        refresh_output(EditorState_get_doc(next));
    }
    js_scope_close();
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);
    g_parser = DOMParser_2_fromSchema(g_schema);
    js_retain(g_parser);
    g_serializer = DOMSerializer_fromSchema(g_schema);
    js_retain(g_serializer);

    EditorStateConfig scfg = EditorStateConfig_create();
    EditorStateConfig_set_schema(scfg, g_schema);
    EditorStateConfig_set_doc(scfg, parse_html(SPEC_HTML));

    DirectEditorProps props = DirectEditorProps_create();
    DirectEditorProps_set_state(props, EditorState_create(scfg));
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    g_status = "Parsed Spec HTML via DOMParser";
    refresh_output(EditorState_get_doc(EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
    js_scope_close();
}
