#include "dom/dom.h"
#include "marked/marked.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static HTMLElement make_result_card(Document doc, const char* title, JsString html) {
    HTMLElement card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(card, make_el(doc, "div", "label", title));
    HTMLElement out = make_el(doc, "div", "md-out", 0);
    HTMLElement_set_innerHTML(out, html);
    HTMLElement_appendChild(card, out);
    HTMLElement raw = make_el(doc, "pre", "html-raw", 0);
    HTMLElement_set_textContent(raw, html);
    HTMLElement_appendChild(card, raw);
    return card;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Marked Inline vs Block (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "parse vs parseInline"));
    HTMLElement_appendChild(app, head);

    Marked m = Marked_new();

    const char* snippets[] = {
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement row = make_el(doc, "div", "row-card", 0);
        HTMLElement_appendChild(row, make_el(doc, "div", "label", "Markdown Snippet"));
        HTMLElement_appendChild(row, make_el(doc, "pre", "src", snippets[i]));

        HTMLElement grid = make_el(doc, "div", "grid", 0);
        JsString block_html = Marked_parse(m, snippets[i]);
        JsString inl_html = Marked_parseInline(m, snippets[i]);

        HTMLElement_appendChild(grid, make_result_card(doc, "Block (parse)", block_html));
        HTMLElement_appendChild(grid, make_result_card(doc, "Inline (parseInline)", inl_html));
        HTMLElement_appendChild(row, grid);
        HTMLElement_appendChild(app, row);
    }
}
