#include "dom/dom.hpp"
#include "katex/katex.hpp"

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
    head.appendChild(make_el(doc, "h1", "", "Inline vs Display Mode (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "displayMode: false vs true"));
    app.appendChild(head);

    katex::KatexOptions inline_opts = katex::KatexOptions::create();
    inline_opts.displayMode(false);
    katex::KatexOptions display_opts = katex::KatexOptions::create();
    display_opts.displayMode(true);

    struct Item { const char* name; const char* tex; };
    const Item items[] = {
        { "Basel Series", "\\sum_{k=1}^{n} \\frac{1}{k^2} = \\frac{\\pi^2}{6}" },
        { "Dilogarithm", "\\int_0^1 \\frac{\\ln(1+x)}{x}\\,dx" },
        { "Sinc Limit", "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1" },
        { "Binomial", "\\binom{n}{k} = \\frac{n!}{k!(n-k)!}" }
    };

    for (const auto& item : items) {
        dom::HTMLElement row = make_el(doc, "div", "row", "");
        for (int m = 0; m < 2; m++) {
            dom::HTMLElement cell = make_el(doc, "div", "cell", "");
            cell.appendChild(make_el(doc, "div", "label", std::string(item.name) + (m == 0 ? " — inline" : " — display")));
            dom::HTMLElement out = make_el(doc, "div", "slot", "");
            out.innerHTML(katex::renderToString(item.tex, m == 0 ? inline_opts : display_opts));
            cell.appendChild(out);
            row.appendChild(cell);
        }
        app.appendChild(row);
    }
}
