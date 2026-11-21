#include "dom/dom.h"
#include "katex/katex.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "ParseError Inspector (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "throwOnError: false vs true"));
    HTMLElement_appendChild(app, head);

    KatexOptions lenient = KatexOptions_create();
    KatexOptions_set_throwOnError(lenient, false);
    KatexOptions_set_errorColor(lenient, "#555555");

    KatexOptions strict = KatexOptions_create();
    KatexOptions_set_throwOnError(strict, true);

    const char* bad_inputs[] = {
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement row = make_el(doc, "div", "row", 0);
        HTMLElement left = make_el(doc, "div", "cell mode-html", 0);
        HTMLElement_appendChild(left, make_el(doc, "div", "label", "throwOnError: false (#555555)"));
        HTMLElement l_slot = make_el(doc, "div", "slot", 0);
        HTMLElement_set_innerHTML(l_slot, katex_renderToString(bad_inputs[i], lenient));
        HTMLElement_appendChild(left, l_slot);

        HTMLElement right = make_el(doc, "div", "cell", 0);
        HTMLElement_appendChild(right, make_el(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        HTMLElement r_out = make_el(doc, "div", "err", 0);
        (void)katex_renderToString(bad_inputs[i], strict);
        if (js_failed()) {
            HTMLElement_set_textContent(r_out, js_error_message(js_take_error()));
        } else {
            HTMLElement_set_textContent(r_out, "(did not throw)");
        }
        HTMLElement_appendChild(right, r_out);

        HTMLElement_appendChild(row, left);
        HTMLElement_appendChild(row, right);
        HTMLElement_appendChild(app, row);
    }

    js_scope_close();
}
