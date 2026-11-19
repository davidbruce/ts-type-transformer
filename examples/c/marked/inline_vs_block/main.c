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

static HTMLElement make_result_card(Document doc, const char* title, JsString html) {
    static char buf[2048];
    uint32_t n = JsString_copy(html, buf, sizeof(buf) - 1);
    buf[n] = '\0';

    HTMLElement card = make_el(doc, "div", "card", 0);
    append_and_release(card, make_el(doc, "div", "label", title));
    HTMLElement out = make_el(doc, "div", "md-out", 0);
    Element_set_innerHTML(out, buf);
    append_and_release(card, out);
    append_and_release(card, make_el(doc, "pre", "html-raw", buf));
    return card;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Marked Inline vs Block (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "parse vs parseInline"));
    append_and_release(app, head);

    Marked m = Marked_new(JsArray_MarkedExtension_new());
    Marked_parse parse_fn = Marked_get_parse(m);
    Marked_parse parse_inline_fn = Marked_get_parseInline(m);
    Marked_parse_call_options_no_async call_opts = { 0 };

    const char* snippets[] = {
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement row = make_el(doc, "div", "row-card", 0);
        append_and_release(row, make_el(doc, "div", "label", "Markdown Snippet"));
        append_and_release(row, make_el(doc, "pre", "src", snippets[i]));

        HTMLElement grid = make_el(doc, "div", "grid", 0);
        JsString block_html = js_to_string(Marked_parse_invoke(parse_fn, snippets[i], call_opts));
        JsString inl_html = js_to_string(Marked_parse_invoke(parse_inline_fn, snippets[i], call_opts));

        append_and_release(grid, make_result_card(doc, "Block (parse)", block_html));
        append_and_release(grid, make_result_card(doc, "Inline (parseInline)", inl_html));
        js_release(block_html);
        js_release(inl_html);

        append_and_release(row, grid);
        append_and_release(app, row);
    }

    js_release(parse_fn);
    js_release(parse_inline_fn);
    js_release(m);
    js_release(app);
    js_release(doc);
}
