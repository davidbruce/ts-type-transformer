#include "dom/dom.hpp"
#include "katex/katex.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Custom Macros (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "\\RR, \\NN, \\dd, \\norm"));
    app.appendChild(head);

    auto macros = katex::KatexOptions_macros::create()
        .set_at("\\RR", ts::Value::string("\\mathbb{R}"))
        .set_at("\\NN", ts::Value::string("\\mathbb{N}"))
        .set_at("\\dd", ts::Value::string("\\mathrm{d}"))
        .set_at("\\norm", ts::Value::string("\\left\\| #1 \\right\\|"));

    auto opts = katex::KatexOptions::create()
        .displayMode(true)
        .macros(macros);

    dom::HTMLElement legend = make_el(doc, "div", "card", "");
    legend.appendChild(make_el(doc, "div", "label", "Defined Macros"));
    legend.appendChild(make_el(doc, "pre", "tex", "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"));
    app.appendChild(legend);

    dom::HTMLElement grid = make_el(doc, "div", "grid", "");
    struct Expr { const char* name; const char* tex; };
    const Expr exprs[] = {
        { "Euclidean Space", "v \\in \\RR^n,\\quad k \\in \\NN" },
        { "Vector Norm", "\\norm{x + y} \\le \\norm{x} + \\norm{y}" },
        { "Differential Form", "\\int_a^b f(x)\\,\\dd x = F(b) - F(a)" },
        { "Inner Product Norm", "\\norm{\\phi}_2 = \\sqrt{\\int_{\\RR} |\\phi(t)|^2\\,\\dd t}" }
    };

    for (const auto& e : exprs) {
        dom::HTMLElement card = make_el(doc, "div", "card", "");
        card.appendChild(make_el(doc, "div", "label", e.name));
        card.appendChild(make_el(doc, "pre", "tex", e.tex));
        dom::HTMLElement slot = make_el(doc, "div", "slot", "");
        slot.innerHTML(katex::renderToString(e.tex, opts));
        card.appendChild(slot);
        grid.appendChild(card);
    }

    app.appendChild(grid);
}
