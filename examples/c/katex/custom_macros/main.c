#include "dom/dom.h"
#include "katex/katex.h"

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
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Custom Macros (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "\\RR, \\NN, \\dd, \\norm"));
    HTMLElement_appendChild(app, head);

    KatexOptions opts = KatexOptions_create(
        .displayMode = true,
        .macros = js_record(
            { "\\RR", js_string("\\mathbb{R}") },
            { "\\NN", js_string("\\mathbb{N}") },
            { "\\dd", js_string("\\mathrm{d}") },
            { "\\norm", js_string("\\left\\| #1 \\right\\|") }
        )
    );

    HTMLElement legend = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(legend, make_el(doc, "div", "label", "Defined Macros"));
    HTMLElement_appendChild(legend, make_el(doc, "pre", "tex", "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"));
    HTMLElement_appendChild(app, legend);

    HTMLElement grid = make_el(doc, "div", "grid", 0);
    const char* names[] = { "Euclidean Space", "Vector Norm", "Differential Form", "Inner Product Norm" };
    const char* texs[] = {
        "v \\in \\RR^n,\\quad k \\in \\NN",
        "\\norm{x + y} \\le \\norm{x} + \\norm{y}",
        "\\int_a^b f(x)\\,\\dd x = F(b) - F(a)",
        "\\norm{\\phi}_2 = \\sqrt{\\int_{\\RR} |\\phi(t)|^2\\,\\dd t}"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement card = make_el(doc, "div", "card", 0);
        HTMLElement_appendChild(card, make_el(doc, "div", "label", names[i]));
        HTMLElement_appendChild(card, make_el(doc, "pre", "tex", texs[i]));
        HTMLElement slot = make_el(doc, "div", "slot", 0);
        HTMLElement_set_innerHTML(slot, katex_renderToString(texs[i], opts));
        HTMLElement_appendChild(card, slot);
        HTMLElement_appendChild(grid, card);
    }

    HTMLElement_appendChild(app, grid);
}
