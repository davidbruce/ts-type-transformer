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
    js_scope();
    int lvl = (int)js_to_number(js_get_prop(Pm_Node_get_attrs(n), "level"));
    JsValue res = dom_hole(lvl == 1 ? "h1" : "h2");
    js_retain(res);
    return res;
}
static JsValue tag_dom(void* ctx, Pm_Node n) { (void)n; return dom_hole((const char*)ctx); }
static JsValue mark_dom(void* ctx, Pm_Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static Pm_TagParseRule make_tag_rule(const char* tag, int level) {
    Pm_TagParseRule r = Pm_TagParseRule_create(.tag = tag);
    if (level > 0) {
        Pm_Attrs a = Pm_Attrs_create();
        js_set_num(a, "level", (double)level);
        Pm_TagParseRule_set_attrs(r, a);
    }
    return r;
}

static Document g_doc;
static Pm_Schema g_schema;
static Pm_DOMParser g_parser;
static Pm_DOMSerializer g_serializer;
static Pm_EditorView g_view;
static const char* g_status = "JSON Node.eq() pending";

static Pm_Schema make_schema(void) {
    Pm_NodeSpec doc = Pm_NodeSpec_create(.content = "block+");

    Pm_AttributeSpec lvl = Pm_AttributeSpec_create(.default_ = js_number(2));
    Pm_NodeSpec heading = Pm_NodeSpec_create(.attrs = Pm_AttributeSpec_record({ "level", lvl }), .content = "inline*", .group = "block", .defining = true);
    Pm_TagParseRule hrules[2] = { make_tag_rule("h1", 1), make_tag_rule("h2", 2) };
    Pm_NodeSpec_set_parseDOM(heading, JsArray_Pm_TagParseRule_from(hrules, 2));
    Pm_NodeSpec_set_toDOM(heading, Pm_NodeSpecToDOMFn_from(heading_dom, NULL));

    Pm_NodeSpec para = Pm_NodeSpec_create(.content = "inline*", .group = "block");
    Pm_TagParseRule p_rule = make_tag_rule("p", 0);
    Pm_NodeSpec_set_parseDOM(para, JsArray_Pm_TagParseRule_from(&p_rule, 1));
    Pm_NodeSpec_set_toDOM(para, Pm_NodeSpecToDOMFn_from(tag_dom, (void*)"p"));

    Pm_NodeSpec quote = Pm_NodeSpec_create(.content = "block+", .group = "block", .defining = true);
    Pm_TagParseRule q_rule = make_tag_rule("blockquote", 0);
    Pm_NodeSpec_set_parseDOM(quote, JsArray_Pm_TagParseRule_from(&q_rule, 1));
    Pm_NodeSpec_set_toDOM(quote, Pm_NodeSpecToDOMFn_from(tag_dom, (void*)"blockquote"));

    Pm_NodeSpec text = Pm_NodeSpec_create(.group = "inline");

    Pm_MarkSpec strong = Pm_MarkSpec_create();
    Pm_ParseRule srule = Pm_ParseRule_of(make_tag_rule("strong", 0));
    Pm_MarkSpec_set_parseDOM(strong, JsArray_Pm_ParseRule_from(&srule, 1));
    Pm_MarkSpec_set_toDOM(strong, Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"strong"));

    Pm_MarkSpec em = Pm_MarkSpec_create();
    Pm_ParseRule erule = Pm_ParseRule_of(make_tag_rule("em", 0));
    Pm_MarkSpec_set_parseDOM(em, JsArray_Pm_ParseRule_from(&erule, 1));
    Pm_MarkSpec_set_toDOM(em, Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"em"));

    Pm_MarkSpec code = Pm_MarkSpec_create();
    Pm_ParseRule crule = Pm_ParseRule_of(make_tag_rule("code", 0));
    Pm_MarkSpec_set_parseDOM(code, JsArray_Pm_ParseRule_from(&crule, 1));
    Pm_MarkSpec_set_toDOM(code, Pm_MarkSpecToDOMFn_from(mark_dom, (void*)"code"));

    return Pm_Schema_new(Pm_SchemaSpec_create(
        .nodes = Pm_NodeSpec_record({ "doc", doc }, { "heading", heading }, { "paragraph", para }, { "blockquote", quote }, { "text", text }),
        .marks = Pm_MarkSpec_record({ "strong", strong }, { "em", em }, { "code", code })
    ));
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

static Pm_Node parse_html(const char* html) {
    HTMLElement div = Document_createElement(g_doc, "div");
    HTMLElement_set_innerHTML(div, html);
    return Pm_DOMParser_parse(g_parser, AS_Node(div));
}

static void refresh_output(Pm_Node doc) {
    js_scope();
    HTMLElement div = Document_createElement(g_doc, "div");
    Pm_DOMSerializer_serializeFragment(g_serializer, Pm_Node_get_content(doc), (Pm_DOMSerializer_serializeFragment_options){ 0 }, div);
    JsString html_js = HTMLElement_get_innerHTML(div);
    int html_len = (int)JsString_len(html_js);

    static char out_buf[16384];
    OutBuf out = { out_buf, out_buf + sizeof out_buf };
    out_fmt(&out, "SERIALIZED HTML (DOMSerializer):\n");
    out_js(&out, html_js);
    int nb = (int)Pm_Node_get_childCount(doc);
    out_fmt(&out, "\n\nDOCUMENT AST & JSON ROUND-TRIP:\ndoc(blocks=%d, nodeSize=%d)\n",
            nb, (int)Pm_Node_get_nodeSize(doc));
    for (int i = 0; i < nb; i++) {
        Pm_Node b = Pm_Node_child(doc, i);
        JsString tname = Pm_NodeType_get_name(Pm_Node_get_type(b));
        out_fmt(&out, "  [%d] ", i);
        if (JsString_equals(tname, "heading")) {
            out_fmt(&out, "heading[level=%d]", (int)js_to_number(js_get_prop(Pm_Node_get_attrs(b), "level")));
        } else {
            out_js(&out, tname);
        }
        out_fmt(&out, " (%d bytes): ", (int)Pm_Node_get_nodeSize(b));
        out_js(&out, Pm_Node_get_textContent(b));
        out_fmt(&out, "\n");
    }
    out_fmt(&out, "\nROUND-TRIP STATUS: %s", g_status);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "output"), out_buf);

    char meta[256];
    snprintf(meta, sizeof meta, "%s \xC2\xB7 blocks=%d \xC2\xB7 nodeSize=%d \xC2\xB7 htmlBytes=%d",
             g_status, nb, (int)Pm_Node_get_nodeSize(doc), html_len);
    HTMLElement_set_textContent(Document_getElementById(g_doc, "meta"), meta);
}

static void on_dispatch(void* ctx, Pm_Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    js_scope();
    Pm_EditorState next = Pm_EditorState_apply(Pm_EditorView_get_state(g_view), tr);
    Pm_EditorView_updateState(g_view, next);
    refresh_output(Pm_EditorState_get_doc(next));
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
    js_scope();
    int idx = *(const int*)ctx;
    Pm_EditorState st = Pm_EditorView_get_state(g_view);
    Pm_Node doc = Pm_EditorState_get_doc(st);
    if (idx == 0) {
        g_status = "Parsed Spec HTML via DOMParser";
        Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = parse_html(SPEC_HTML));
        Pm_EditorState next = Pm_EditorState_create(cfg);
        Pm_EditorView_updateState(g_view, next);
        refresh_output(Pm_EditorState_get_doc(next));
    } else if (idx == 1) {
        g_status = "Parsed Changelog HTML via DOMParser";
        Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = parse_html(CHANGELOG_HTML));
        Pm_EditorState next = Pm_EditorState_create(cfg);
        Pm_EditorView_updateState(g_view, next);
        refresh_output(Pm_EditorState_get_doc(next));
    } else if (idx == 2) {
        Pm_Node rebuilt = Pm_Node_fromJSON(g_schema, Pm_Node_toJSON(doc));
        bool ok = Pm_Node_eq(doc, rebuilt);
        g_status = ok ? "JSON toJSON->fromJSON eq() = true" : "JSON toJSON->fromJSON eq() = false";
        refresh_output(doc);
    } else if (idx == 3) {
        if (Pm_Node_get_childCount(doc) >= 2.0) {
            double insert_pos = Pm_Node_get_nodeSize(Pm_Node_child(doc, 0)) + Pm_Node_get_nodeSize(Pm_Node_child(doc, 1)) - 1.0;
            Pm_Mark code_mark = Pm_Schema_mark(g_schema, "code");
            Pm_Node span = Pm_Schema_text(g_schema, " roundtrip()", JsArray_Pm_Mark_from(&code_mark, 1));
            Pm_Transaction tr = Pm_EditorState_get_tr(st);
            Pm_Transaction_insert(tr, insert_pos, span);
            g_status = "Inserted code span into paragraph";
            Pm_EditorView_dispatch(g_view, tr);
        }
    } else if (idx == 4) {
        HTMLElement div = Document_createElement(g_doc, "div");
        Pm_DOMSerializer_serializeFragment(g_serializer, Pm_Node_get_content(doc), (Pm_DOMSerializer_serializeFragment_options){ 0 }, div);
        Pm_Node reparsed = Pm_DOMParser_parse(g_parser, AS_Node(div));
        bool ok = Pm_Node_eq(doc, reparsed);
        g_status = ok ? "DOM serializeFragment->parse eq() = true" : "DOM serializeFragment->parse eq() = false";
        Pm_EditorStateConfig cfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = reparsed);
        Pm_EditorState next = Pm_EditorState_create(cfg);
        Pm_EditorView_updateState(g_view, next);
        refresh_output(Pm_EditorState_get_doc(next));
    }
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    g_doc = get_document();
    js_retain(g_doc);
    g_schema = make_schema();
    js_retain(g_schema);
    g_parser = Pm_DOMParser_fromSchema(g_schema);
    js_retain(g_parser);
    g_serializer = Pm_DOMSerializer_fromSchema(g_schema);
    js_retain(g_serializer);

    Pm_EditorStateConfig scfg = Pm_EditorStateConfig_create(.schema = g_schema, .doc = parse_html(SPEC_HTML));

    Pm_DirectEditorProps props = Pm_DirectEditorProps_create(.state = Pm_EditorState_create(scfg), .dispatchTransaction = Pm_DispatchTransactionFn_from(on_dispatch, NULL));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = Pm_EditorView_new(AS_Node(mount), props);
    js_retain(g_view);
    g_status = "Parsed Spec HTML via DOMParser";
    refresh_output(Pm_EditorState_get_doc(Pm_EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
