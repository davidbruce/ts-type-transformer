#include "dom/dom.hpp"
#include "marked/marked.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

template <class T>
static T as_tok(const ts::Value& v) {
    T t;
    static_cast<ts::Handle&>(t) = v;
    return t;
}

static std::string sanitize_raw(const std::string& raw) {
    std::string out;
    out.reserve(raw.size());
    for (char c : raw) {
        if (c == '\n') out += "\xE2\x86\xB5";
        else out.push_back(c);
    }
    return out;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    std::string sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";
    marked::TokensList tokens = marked::Lexer<std::string, std::string>::lex_static(sample);

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked Token Inspector (C++)"));
    head.appendChild(make_el(doc, "span", "badge", std::to_string(int(tokens.length())) + " tokens"));
    app.appendChild(head);

    dom::HTMLElement src_card = make_el(doc, "div", "card", "");
    src_card.appendChild(make_el(doc, "div", "label", "Markdown Input"));
    src_card.appendChild(make_el(doc, "pre", "src", sample));
    app.appendChild(src_card);

    dom::HTMLElement tbl_card = make_el(doc, "div", "card", "");
    tbl_card.appendChild(make_el(doc, "div", "label", "Lexer Token Stream"));

    dom::HTMLElement table = make_el(doc, "table", "tok-table", "");
    dom::HTMLElement thead = make_el(doc, "thead", "", "");
    dom::HTMLElement hrow = make_el(doc, "tr", "", "");
    for (const char* col : { "#", "Type", "Bytes", "Raw Snippet" }) {
        hrow.appendChild(make_el(doc, "th", "", col));
    }
    thead.appendChild(hrow);
    table.appendChild(thead);

    dom::HTMLElement tbody = make_el(doc, "tbody", "", "");
    int idx = 0;
    for (ts::Value v : tokens) {
        marked::Tokens_Generic g = as_tok<marked::Tokens_Generic>(v);
        std::string raw = g.raw();
        dom::HTMLElement tr = make_el(doc, "tr", "", "");
        tr.appendChild(make_el(doc, "td", "", std::to_string(idx)));
        tr.appendChild(make_el(doc, "td", "", g.type()));
        tr.appendChild(make_el(doc, "td", "", std::to_string(raw.size())));
        tr.appendChild(make_el(doc, "td", "", sanitize_raw(raw)));
        tbody.appendChild(tr);
        idx++;
    }
    table.appendChild(tbody);
    tbl_card.appendChild(table);
    app.appendChild(tbl_card);
}
