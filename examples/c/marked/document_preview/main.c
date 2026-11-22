#include "dom/dom.h"
#include "marked/marked.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Marked Document Preview (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "marked"));
    HTMLElement_appendChild(app, head);

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
    HTMLElement_appendChild(src_card, make_el(doc, "div", "label", "Markdown Source"));
    HTMLElement_appendChild(src_card, make_el(doc, "pre", "src", markdown_src));

    HTMLElement out_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(out_card, make_el(doc, "div", "label", "Rendered HTML"));
    HTMLElement md_out = make_el(doc, "div", "md-out", 0);
    HTMLElement_set_innerHTML(md_out, marked(markdown_src, (marked_options_no_async){ 0 }));

    HTMLElement_appendChild(out_card, md_out);
    HTMLElement_appendChild(grid, src_card);
    HTMLElement_appendChild(grid, out_card);
    HTMLElement_appendChild(app, grid);
}
