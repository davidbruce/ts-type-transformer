#include "dom/dom.h"
#include "marked/marked.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
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
    js_scope_open();
    Tokens_Heading h = JsTuple_Tokens_Heading_get_0(args);
    uint32_t depth = (uint32_t)Tokens_Heading_get_depth(h);
    if (depth < 1 || depth > 6) depth = 1;
    char text_buf[128];
    JsString_copy(Tokens_Heading_get_text(h), text_buf, sizeof text_buf);
    char out[256];
    snprintf(out, sizeof out, "<h%u class=\"wire-h\">\xC2\xA7 %s</h%u>\n", depth, text_buf, depth);
    JsValue res = js_string(out);
    js_retain(res);
    js_scope_close();
    return res;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
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

    Marked custom_m = Marked_new(JsArray_MarkedExtension_from(&ext, 1));
    JsString def_html = marked(md, (marked_options_no_async){ 0 });
    JsString custom_html = js_to_string(Marked_parse_invoke(Marked_get_parse(custom_m), md, (Marked_parse_call_options_no_async){ 0 }));

    char badge_buf[48];
    snprintf(badge_buf, sizeof badge_buf, "%u tokens visited", walked_count);

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Marked Custom Renderer & Walker (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", badge_buf));
    HTMLElement_appendChild(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);

    HTMLElement def_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(def_card, make_el(doc, "div", "label", "Default Renderer"));
    HTMLElement def_out = make_el(doc, "div", "md-out", 0);
    HTMLElement_set_innerHTML(def_out, def_html);
    HTMLElement_appendChild(def_card, def_out);

    HTMLElement cust_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(cust_card, make_el(doc, "div", "label", "Custom Heading Renderer"));
    HTMLElement cust_out = make_el(doc, "div", "md-out", 0);
    HTMLElement_set_innerHTML(cust_out, custom_html);
    HTMLElement_appendChild(cust_card, cust_out);

    HTMLElement_appendChild(grid, def_card);
    HTMLElement_appendChild(grid, cust_card);
    HTMLElement_appendChild(app, grid);

    HTMLElement walk_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(walk_card, make_el(doc, "div", "label", "Visited Tokens (walkTokens)"));
    HTMLElement strip = make_el(doc, "div", "tok-strip", 0);
    for (uint32_t i = 0; i < walked_count; i++) {
        HTMLElement_appendChild(strip, make_el(doc, "span", "tok-pill", walked_types[i]));
    }
    HTMLElement_appendChild(walk_card, strip);
    HTMLElement_appendChild(app, walk_card);

    js_scope_close();
}
