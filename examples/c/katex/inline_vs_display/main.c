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
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Inline vs Display Mode (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "displayMode: false vs true"));
    HTMLElement_appendChild(app, head);

    KatexOptions inline_opts = KatexOptions_create(.displayMode = false);
    KatexOptions display_opts = KatexOptions_create(.displayMode = true);

    const char* labels_in[] = { "Basel Series — inline", "Dilogarithm — inline", "Sinc Limit — inline", "Binomial — inline" };
    const char* labels_disp[] = { "Basel Series — display", "Dilogarithm — display", "Sinc Limit — display", "Binomial — display" };
    const char* texs[] = {
        "\\sum_{k=1}^{n} \\frac{1}{k^2} = \\frac{\\pi^2}{6}",
        "\\int_0^1 \\frac{\\ln(1+x)}{x}\\,dx",
        "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1",
        "\\binom{n}{k} = \\frac{n!}{k!(n-k)!}"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement row = make_el(doc, "div", "row", 0);
        HTMLElement c1 = make_el(doc, "div", "cell", 0);
        HTMLElement_appendChild(c1, make_el(doc, "div", "label", labels_in[i]));
        HTMLElement out1 = make_el(doc, "div", "slot", 0);
        HTMLElement_set_innerHTML(out1, katex_renderToString(texs[i], inline_opts));
        HTMLElement_appendChild(c1, out1);
        HTMLElement_appendChild(row, c1);

        HTMLElement c2 = make_el(doc, "div", "cell", 0);
        HTMLElement_appendChild(c2, make_el(doc, "div", "label", labels_disp[i]));
        HTMLElement out2 = make_el(doc, "div", "slot", 0);
        HTMLElement_set_innerHTML(out2, katex_renderToString(texs[i], display_opts));
        HTMLElement_appendChild(c2, out2);
        HTMLElement_appendChild(row, c2);

        HTMLElement_appendChild(app, row);
    }
}
