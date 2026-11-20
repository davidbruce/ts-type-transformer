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
    head.appendChild(make_el(doc, "h1", "", "KaTeX Formula Gallery (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "katex v" + katex::get_version()));
    app.appendChild(head);

    auto grid = make_el(doc, "div", "grid", "");
    auto opts = katex::KatexOptions::create().displayMode(true);

    struct Entry { const char* name; const char* tex; };
    const Entry formulas[] = {
        { "Euler's Identity", "e^{i\\pi} + 1 = 0" },
        { "Quadratic Formula", "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}" },
        { "Gaussian Integral", "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}" },
        { "Maxwell-Faraday", "\\nabla \\times \\mathbf{E} = -\\frac{\\partial \\mathbf{B}}{\\partial t}" },
        { "Fourier Transform", "\\hat{f}(\\xi) = \\int_{-\\infty}^{\\infty} f(x)e^{-2\\pi i x \\xi}\\,dx" }
    };

    for (const auto& f : formulas) {
        dom::HTMLElement card = make_el(doc, "div", "card", "");
        card.appendChild(make_el(doc, "div", "label", f.name));
        card.appendChild(make_el(doc, "pre", "tex", f.tex));
        dom::HTMLElement slot = make_el(doc, "div", "slot", "");
        katex::render(f.tex, slot, opts);
        card.appendChild(slot);
        grid.appendChild(card);
    }

    app.appendChild(grid);
}
