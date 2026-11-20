#include "dom/dom.h"
#include "marked/marked.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag, (ElementCreationOptions){ 0 });
    if (cls && cls[0]) Element_set_className(el, cls);
    if (text) Node_set_textContent(el, text);
    return el;
}

static void append_and_release(HTMLElement parent, HTMLElement child) {
    js_release(Node_appendChild(parent, child));
    js_release(child);
}

static char walked_types[64][32];
static uint32_t walked_count = 0;

static JsValue walk_token_cb(void* ctx, JsValue token) {
    (void)ctx;
    if (walked_count < 64) {
        JsString t = Tokens_Generic_get_type((Tokens_Generic){ token.h });
        JsString_copy(t, walked_types[walked_count], sizeof(walked_types[0]));
        js_release(t);
        walked_count++;
    }
    return (JsValue){ 0 };
}

static JsValue render_heading(void* ctx, JsTuple_Tokens_Heading args) {
    (void)ctx;
    Tokens_Heading h = JsTuple_Tokens_Heading_get_0(args);
    uint32_t depth = (uint32_t)Tokens_Heading_get_depth(h);
    if (depth < 1 || depth > 6) depth = 1;
    JsString text = Tokens_Heading_get_text(h);
    char text_buf[128];
    uint32_t n = JsString_copy(text, text_buf, sizeof(text_buf));
    if (n >= sizeof(text_buf)) n = sizeof(text_buf) - 1;
    js_release(text);
    js_release(h);

    char out[256];
    const char* p1 = "<h  class=\"wire-h\">\xC2\xA7 ";
    uint32_t k = 0;
    for (uint32_t i = 0; p1[i]; i++) out[k++] = p1[i];
    out[2] = (char)('0' + depth);
    for (uint32_t i = 0; i < n; i++) out[k++] = text_buf[i];
    const char* p2 = "</h >\n";
    for (uint32_t i = 0; p2[i]; i++) out[k++] = p2[i];
    out[k - 3] = (char)('0' + depth);
    out[k] = '\0';
    return js_string(out);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    const char* md =
        "# Wireframe Spec\n\n"
        "## Custom Pipeline\n\n"
        "Paragraph with *italic* and **bold** and `code`.\n";

    MarkedExtension ext = MarkedExtension_create();
    MarkedExtension_renderer rend = MarkedExtension_renderer_create();
    MarkedExtension_renderer_set_heading(rend, MarkedExtension_renderer_heading_from(render_heading, 0));
    MarkedExtension_set_renderer(ext, rend);
    MarkedExtension_set_walkTokens(ext, Token_to_void_or_Promise_or_void_or_Promise_array_fn_from(walk_token_cb, 0));

    JsArray_MarkedExtension exts = JsArray_MarkedExtension_new();
    JsArray_MarkedExtension_push(exts, ext);
    Marked custom_m = Marked_new(exts);

    JsString def_html = marked(md, (marked_options_no_async){ 0 });
    JsString custom_html = js_to_string(Marked_parse_invoke(Marked_get_parse(custom_m), md, (Marked_parse_call_options_no_async){ 0 }));

    char badge_buf[48];
    uint32_t v = walked_count;
    uint32_t nl = 0;
    if (v == 0) {
        badge_buf[nl++] = '0';
    } else {
        char tmp[16];
        while (v > 0) { tmp[nl++] = (char)('0' + (v % 10)); v /= 10; }
        for (uint32_t i = 0; i < nl; i++) badge_buf[i] = tmp[nl - 1 - i];
    }
    uint32_t bp = nl;
    const char* suffix = " tokens visited";
    for (uint32_t i = 0; suffix[i]; i++) badge_buf[bp++] = suffix[i];
    badge_buf[bp] = '\0';

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Marked Custom Renderer & Walker (C)"));
    append_and_release(head, make_el(doc, "span", "badge", badge_buf));
    append_and_release(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);
    static char html_buf[4096];

    HTMLElement def_card = make_el(doc, "div", "card", 0);
    append_and_release(def_card, make_el(doc, "div", "label", "Default Renderer"));
    HTMLElement def_out = make_el(doc, "div", "md-out", 0);
    JsString_copy(def_html, html_buf, sizeof(html_buf));
    Element_set_innerHTML(def_out, html_buf);
    append_and_release(def_card, def_out);

    HTMLElement cust_card = make_el(doc, "div", "card", 0);
    append_and_release(cust_card, make_el(doc, "div", "label", "Custom Heading Renderer"));
    HTMLElement cust_out = make_el(doc, "div", "md-out", 0);
    JsString_copy(custom_html, html_buf, sizeof(html_buf));
    Element_set_innerHTML(cust_out, html_buf);
    append_and_release(cust_card, cust_out);

    append_and_release(grid, def_card);
    append_and_release(grid, cust_card);
    append_and_release(app, grid);

    HTMLElement walk_card = make_el(doc, "div", "card", 0);
    append_and_release(walk_card, make_el(doc, "div", "label", "Visited Tokens (walkTokens)"));
    HTMLElement strip = make_el(doc, "div", "tok-strip", 0);
    for (uint32_t i = 0; i < walked_count; i++) {
        append_and_release(strip, make_el(doc, "span", "tok-pill", walked_types[i]));
    }
    append_and_release(walk_card, strip);
    append_and_release(app, walk_card);

    js_release(def_html);
    js_release(custom_html);
    js_release(custom_m);
    js_release(exts);
    js_release(rend);
    js_release(ext);
    js_release(app);
    js_release(doc);
}
