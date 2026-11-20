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

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked Inline vs Block (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "parse vs parseInline"));
    app.appendChild(head);

    auto md = marked::Marked<std::string, std::string>::create();

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
        std::string block_html = std::get<std::string>(md.parse()(s));
        std::string inl_html = std::get<std::string>(md.parseInline()(s));
        grid.appendChild(make_result_card(doc, "Block (parse)", block_html));
        grid.appendChild(make_result_card(doc, "Inline (parseInline)", inl_html));
        row.appendChild(grid);
        app.appendChild(row);
    }
}
