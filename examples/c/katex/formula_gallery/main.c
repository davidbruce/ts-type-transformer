#include "dom/dom.h"
#include "katex/katex.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag, (ElementCreationOptions){ 0 });
    if (cls && cls[0]) Element_set_className(el, cls);
    if (text) Node_set_textContent(el, text);
    return el;
}

static void append_and_release(HTMLElement parent, HTMLElement child) {
    js_release(Node_appendChild(parent, child));
    js_release(child);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement title = make_el(doc, "h1", 0, "KaTeX Formula Gallery (C)");
    JsString ver = get_version();
    char badge_buf[64] = "katex v";
    JsString_copy(ver, badge_buf + 7, sizeof(badge_buf) - 7);
    js_release(ver);
    HTMLElement badge = make_el(doc, "span", "badge", badge_buf);
    append_and_release(head, title);
    append_and_release(head, badge);
    append_and_release(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);
    KatexOptions opts = KatexOptions_create();
    KatexOptions_set_displayMode(opts, js_some_bool(true));

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
        HTMLElement lbl = make_el(doc, "div", "label", names[i]);
        HTMLElement src = make_el(doc, "pre", "tex", texs[i]);
        HTMLElement slot = make_el(doc, "div", "slot", 0);
        render(texs[i], slot, opts);
        append_and_release(card, lbl);
        append_and_release(card, src);
        append_and_release(card, slot);
        append_and_release(grid, card);
    }

    append_and_release(app, grid);
    js_release(opts);
    js_release(app);
    js_release(doc);
}
