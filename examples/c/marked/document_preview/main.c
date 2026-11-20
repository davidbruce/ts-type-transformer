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

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Marked Document Preview (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "marked"));
    append_and_release(app, head);

    const char* markdown_src =
        "# Marked Wireframe Spec\n\n"
        "## Features\n\n"
        "Typed bindings with *emphasis*, **strong**, `inline code`, and [reference](https://example.com \"Spec\").\n\n"
        "- Lexer AST\n"
        "- Custom renderer\n"
        "- GFM tables\n\n"
        "1. Parse\n"
        "2. Walk\n"
        "3. Render\n\n"
        "> Typed bindings from TypeScript definitions.\n\n"
        "```ts\n"
        "const ok: boolean = true;\n"
        "```\n\n"
        "---\n";

    HTMLElement grid = make_el(doc, "div", "grid", 0);

    HTMLElement src_card = make_el(doc, "div", "card", 0);
    append_and_release(src_card, make_el(doc, "div", "label", "Markdown Source"));
    append_and_release(src_card, make_el(doc, "pre", "src", markdown_src));

    HTMLElement out_card = make_el(doc, "div", "card", 0);
    append_and_release(out_card, make_el(doc, "div", "label", "Rendered HTML"));
    HTMLElement md_out = make_el(doc, "div", "md-out", 0);

    JsString html = marked(markdown_src, (marked_options_no_async){ 0 });
    static char html_buf[8192];
    JsString_copy(html, html_buf, sizeof(html_buf));
    js_release(html);
    Element_set_innerHTML(md_out, html_buf);

    append_and_release(out_card, md_out);
    append_and_release(grid, src_card);
    append_and_release(grid, out_card);
    append_and_release(app, grid);

    js_release(app);
    js_release(doc);
}
