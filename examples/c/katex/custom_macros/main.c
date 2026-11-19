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

static void add_macro(KatexOptions_macros macros, const char* name, const char* expansion) {
    JsValue val = js_string(expansion);
    KatexOptions_macros_set_at(macros, name, val);
    js_release(val);
}

__attribute__((export_name("start"))) void start(void) {
    static char buf[1 << 16];
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Custom Macros (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "\\RR, \\NN, \\dd, \\norm"));
    append_and_release(app, head);

    KatexOptions_macros macros = KatexOptions_macros_create();
    add_macro(macros, "\\RR", "\\mathbb{R}");
    add_macro(macros, "\\NN", "\\mathbb{N}");
    add_macro(macros, "\\dd", "\\mathrm{d}");
    add_macro(macros, "\\norm", "\\left\\| #1 \\right\\|");

    KatexOptions opts = KatexOptions_create();
    KatexOptions_set_displayMode(opts, js_some_bool(true));
    KatexOptions_set_macros(opts, macros);

    HTMLElement legend = make_el(doc, "div", "card", 0);
    append_and_release(legend, make_el(doc, "div", "label", "Defined Macros"));
    append_and_release(legend, make_el(doc, "pre", "tex", "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"));
    append_and_release(app, legend);

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
        append_and_release(card, make_el(doc, "div", "label", names[i]));
        append_and_release(card, make_el(doc, "pre", "tex", texs[i]));
        HTMLElement slot = make_el(doc, "div", "slot", 0);
        JsString html = renderToString(texs[i], opts);
        JsString_copy(html, buf, sizeof(buf));
        Element_set_innerHTML(slot, buf);
        js_release(html);
        append_and_release(card, slot);
        append_and_release(grid, card);
    }

    append_and_release(app, grid);
    js_release(opts);
    js_release(macros);
    js_release(app);
    js_release(doc);
}
