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
static void sb_js(StrBuf* s, JsString js) {
    char tmp[2048];
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
static JsValue mark_dom(void* ctx, Mark m, bool inl) {
    (void)m; (void)inl;
    return dom_hole((const char*)ctx);
}

static TagParseRule make_tag_rule(const char* tag, int level) {
    TagParseRule r = TagParseRule_create();
    TagParseRule_set_tag(r, tag);
    if (level > 0) {
        Attrs a = Attrs_create();
        Attrs_set_at(a, "level", js_number((double)level));
        GenericParseRule_set_attrs(TagParseRule_as_GenericParseRule(r), a);
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
    NodeSpec_set_defining(heading, js_some_bool(true));
    JsArray_TagParseRule hrules = JsArray_TagParseRule_new();
    JsArray_TagParseRule_push(hrules, make_tag_rule("h1", 1));
    JsArray_TagParseRule_push(hrules, make_tag_rule("h2", 2));
    NodeSpec_set_parseDOM(heading, hrules);
    NodeSpec_set_toDOM(heading, Node_to_DOMOutputSpec_fn_from(heading_dom, 0));

    NodeSpec para = NodeSpec_create();
    NodeSpec_set_content(para, "inline*");
    NodeSpec_set_group(para, "block");
    JsArray_TagParseRule prules = JsArray_TagParseRule_new();
    JsArray_TagParseRule_push(prules, make_tag_rule("p", 0));
    NodeSpec_set_parseDOM(para, prules);
    NodeSpec_set_toDOM(para, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"p"));

    NodeSpec quote = NodeSpec_create();
    NodeSpec_set_content(quote, "block+");
    NodeSpec_set_group(quote, "block");
    NodeSpec_set_defining(quote, js_some_bool(true));
    JsArray_TagParseRule qrules = JsArray_TagParseRule_new();
    JsArray_TagParseRule_push(qrules, make_tag_rule("blockquote", 0));
    NodeSpec_set_parseDOM(quote, qrules);
    NodeSpec_set_toDOM(quote, Node_to_DOMOutputSpec_fn_from(tag_dom, (void*)"blockquote"));

    NodeSpec text = NodeSpec_create();
    NodeSpec_set_group(text, "inline");

    Attrs nodes = Attrs_create();
    Attrs_set_at(nodes, "doc", (JsValue){ doc.h });
    Attrs_set_at(nodes, "heading", (JsValue){ heading.h });
    Attrs_set_at(nodes, "paragraph", (JsValue){ para.h });
    Attrs_set_at(nodes, "blockquote", (JsValue){ quote.h });
    Attrs_set_at(nodes, "text", (JsValue){ text.h });

    MarkSpec strong = MarkSpec_create();
    JsArray_ParseRule srules = JsArray_ParseRule_new();
    JsArray_ParseRule_push(srules, (ParseRule){ .tag = ParseRule_TagParseRule, .as = { .TagParseRule = make_tag_rule("strong", 0) } });
    MarkSpec_set_parseDOM(strong, srules);
    MarkSpec_set_toDOM(strong, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"strong"));

    MarkSpec em = MarkSpec_create();
    JsArray_ParseRule erules = JsArray_ParseRule_new();
    JsArray_ParseRule_push(erules, (ParseRule){ .tag = ParseRule_TagParseRule, .as = { .TagParseRule = make_tag_rule("em", 0) } });
    MarkSpec_set_parseDOM(em, erules);
    MarkSpec_set_toDOM(em, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"em"));

    MarkSpec code = MarkSpec_create();
    JsArray_ParseRule crules = JsArray_ParseRule_new();
    JsArray_ParseRule_push(crules, (ParseRule){ .tag = ParseRule_TagParseRule, .as = { .TagParseRule = make_tag_rule("code", 0) } });
    MarkSpec_set_parseDOM(code, crules);
    MarkSpec_set_toDOM(code, Mark_boolean_to_DOMOutputSpec_fn_from(mark_dom, (void*)"code"));

    Attrs marks = Attrs_create();
    Attrs_set_at(marks, "strong", (JsValue){ strong.h });
    Attrs_set_at(marks, "em", (JsValue){ em.h });
    Attrs_set_at(marks, "code", (JsValue){ code.h });

    SchemaSpec spec = SchemaSpec_create();
    SchemaSpec_set_nodes(spec, (JsValue){ nodes.h });
    SchemaSpec_set_marks(spec, (JsValue){ marks.h });
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
    Element_set_innerHTML(div, html);
    return DOMParser_2_parse(g_parser, (Node_2){ div.h });
}

static void refresh_output(Node_3 doc) {
    HTMLElement div = Document_createElement(g_doc, "div");
    DOMSerializer_serializeFragment(g_serializer, Node_3_get_content(doc), (DOMSerializer_serializeFragment_options){ 0 }, div);
    JsString html_js = Element_get_innerHTML(div);
    int html_len = (int)ts_raw_js_string_len(html_js.h);

    StrBuf out = { .pos = 0 };
    sb_str(&out, "SERIALIZED HTML (DOMSerializer):\n");
    sb_js(&out, html_js);
    sb_str(&out, "\n\nDOCUMENT AST & JSON ROUND-TRIP:\ndoc(blocks=");
    int nb = (int)Node_3_get_childCount(doc);
    sb_int(&out, nb);
    sb_str(&out, ", nodeSize="); sb_int(&out, (int)Node_3_get_nodeSize(doc));
    sb_str(&out, ")\n");
    for (int i = 0; i < nb; i++) {
        Node_3 b = Node_3_child(doc, i);
        JsString tname = NodeType_get_name(Node_3_get_type(b));
        sb_str(&out, "  ["); sb_int(&out, i); sb_str(&out, "] ");
        if (js_str_eq(tname, "heading")) {
            sb_str(&out, "heading[level=");
            sb_int(&out, (int)js_to_number(Attrs_get_at(Node_3_get_attrs(b), "level")));
            sb_str(&out, "]");
        } else {
            sb_js(&out, NodeType_get_name(Node_3_get_type(b)));
        }
        sb_str(&out, " ("); sb_int(&out, (int)Node_3_get_nodeSize(b)); sb_str(&out, " bytes): ");
        sb_js(&out, Node_3_get_textContent(b));
        sb_str(&out, "\n");
    }
    sb_str(&out, "\nROUND-TRIP STATUS: "); sb_str(&out, g_status);
    out.buf[out.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "output").h }, out.buf);

    StrBuf meta = { .pos = 0 };
    sb_str(&meta, g_status);
    sb_str(&meta, " · blocks="); sb_int(&meta, nb);
    sb_str(&meta, " · nodeSize="); sb_int(&meta, (int)Node_3_get_nodeSize(doc));
    sb_str(&meta, " · htmlBytes="); sb_int(&meta, html_len);
    meta.buf[meta.pos] = 0;
    Node_set_textContent((Node){ Document_getElementById(g_doc, "meta").h }, meta.buf);
}

static void on_dispatch(void* ctx, Transaction tr) {
    (void)ctx;
    if (g_view.h == 0) return;
    EditorState next = EditorState_apply(EditorView_get_state(g_view), tr);
    EditorView_updateState(g_view, next);
    refresh_output(EditorState_get_doc(next));
}

static const int BTN_IDS[5] = { 0, 1, 2, 3, 4 };
static void on_click(void* ctx, PointerEvent ev) {
    (void)ev;
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
            JsArray_Mark mc = JsArray_Mark_new();
            JsArray_Mark_push(mc, Schema_mark(g_schema, "code", (Attrs){ 0 }));
            Node_3 span = Schema_text(g_schema, " roundtrip()", mc);
            Transaction tr = EditorState_get_tr(st);
            Transform_insert_with_Node(tr, insert_pos, span);
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
}

__attribute__((export_name("start"))) void start(void) {
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
    DirectEditorProps_set_dispatchTransaction(props, Transaction_fn_from(on_dispatch, 0));

    HTMLElement mount = Document_getElementById(g_doc, "editor");
    g_view = EditorView_new((dist_DOMNode){ mount.h }, props);
    js_retain(g_view);
    g_status = "Parsed Spec HTML via DOMParser";
    refresh_output(EditorState_get_doc(EditorView_get_state(g_view)));

    static const char* ids[5] = { "btn-0", "btn-1", "btn-2", "btn-3", "btn-4" };
    for (int i = 0; i < 5; i++) {
        HTMLElement btn = Document_getElementById(g_doc, ids[i]);
        HTMLElement_addEventListener_click(btn, HTMLElement_PointerEvent_fn_from(on_click, (void*)&BTN_IDS[i]));
    }
}
