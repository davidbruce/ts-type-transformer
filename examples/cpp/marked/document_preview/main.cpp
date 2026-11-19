#include "dom/dom.hpp"
#include "marked/marked.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked Document Preview (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "marked"));
    app.appendChild(head);

    std::string markdown_src =
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

    dom::HTMLElement grid = make_el(doc, "div", "grid", "");

    dom::HTMLElement src_card = make_el(doc, "div", "card", "");
    src_card.appendChild(make_el(doc, "div", "label", "Markdown Source"));
    src_card.appendChild(make_el(doc, "pre", "src", markdown_src));

    dom::HTMLElement out_card = make_el(doc, "div", "card", "");
    out_card.appendChild(make_el(doc, "div", "label", "Rendered HTML"));
    dom::HTMLElement md_out = make_el(doc, "div", "md-out", "");
    std::string html = marked::marked(markdown_src, marked::marked_options_no_async::create());
    md_out.innerHTML(html);
    out_card.appendChild(md_out);

    grid.appendChild(src_card);
    grid.appendChild(out_card);
    app.appendChild(grid);
}
