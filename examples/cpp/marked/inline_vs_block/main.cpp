#include "dom/dom.hpp"
#include "marked/marked.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static dom::HTMLElement make_result_card(dom::Document doc, std::string_view label, const std::string& html) {
    dom::HTMLElement card = make_el(doc, "div", "card", "");
    card.appendChild(make_el(doc, "div", "label", label));
    dom::HTMLElement out = make_el(doc, "div", "md-out", "");
    out.innerHTML(html);
    card.appendChild(out);
    card.appendChild(make_el(doc, "pre", "html-raw", html));
    return card;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked Inline vs Block (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "parse vs parseInline"));
    app.appendChild(head);

    using Mk = marked::Marked<std::string, std::string>;
    Mk md = Mk::create(ts::Array<marked::MarkedExtension<std::string, std::string>>::create());

    const char* snippets[] = {
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***"
    };

    for (const char* s : snippets) {
        dom::HTMLElement row = make_el(doc, "div", "row-card", "");
        row.appendChild(make_el(doc, "div", "label", "Markdown Snippet"));
        row.appendChild(make_el(doc, "pre", "src", s));

        dom::HTMLElement grid = make_el(doc, "div", "grid", "");
        std::string block_html = std::get<std::string>(md.parse().invoke(s));
        std::string inl_html = std::get<std::string>(md.parseInline().invoke(s));
        grid.appendChild(make_result_card(doc, "Block (parse)", block_html));
        grid.appendChild(make_result_card(doc, "Inline (parseInline)", inl_html));
        row.appendChild(grid);
        app.appendChild(row);
    }
}
