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
    HTMLElement title = make_el(doc, "h1", 0, "KaTeX Formula Gallery (C)");
    char badge_buf[64] = "katex v";
    JsString_copy(katex_get_version_2(), badge_buf + 7, sizeof(badge_buf) - 7);
    HTMLElement badge = make_el(doc, "span", "badge", badge_buf);
    HTMLElement_appendChild(head, title);
    HTMLElement_appendChild(head, badge);
    HTMLElement_appendChild(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);
    KatexOptions opts = KatexOptions_create(.displayMode = true);

    const char* names[] = {
        "Euler's Identity",
        "Quadratic Formula",
        "Gaussian Integral",
        "Maxwell-Faraday",
        "Fourier Transform"
    };
    const char* texs[] = {
        "e^{i\\pi} + 1 = 0",
        "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}",
        "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}",
        "\\nabla \\times \\mathbf{E} = -\\frac{\\partial \\mathbf{B}}{\\partial t}",
        "\\hat{f}(\\xi) = \\int_{-\\infty}^{\\infty} f(x)e^{-2\\pi i x \\xi}\\,dx"
    };

    for (int i = 0; i < 5; i++) {
        HTMLElement card = make_el(doc, "div", "card", 0);
        HTMLElement_appendChild(card, make_el(doc, "div", "label", names[i]));
        HTMLElement_appendChild(card, make_el(doc, "pre", "tex", texs[i]));
        HTMLElement slot = make_el(doc, "div", "slot", 0);
        katex_render(texs[i], slot, opts);
        HTMLElement_appendChild(card, slot);
        HTMLElement_appendChild(grid, card);
    }

    HTMLElement_appendChild(app, grid);
}
