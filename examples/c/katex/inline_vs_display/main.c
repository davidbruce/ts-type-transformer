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

static void render_html(HTMLElement target, const char* tex, KatexOptions opts) {
    static char buf[1 << 16];
    JsString html = renderToString(tex, opts);
    JsString_copy(html, buf, sizeof(buf));
    Element_set_innerHTML(target, buf);
    js_release(html);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Inline vs Display Mode (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "displayMode: false vs true"));
    append_and_release(app, head);

    KatexOptions inline_opts = KatexOptions_create();
    KatexOptions_set_displayMode(inline_opts, js_some_bool(false));
    KatexOptions display_opts = KatexOptions_create();
    KatexOptions_set_displayMode(display_opts, js_some_bool(true));

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
        append_and_release(c1, make_el(doc, "div", "label", labels_in[i]));
        HTMLElement out1 = make_el(doc, "div", "slot", 0);
        render_html(out1, texs[i], inline_opts);
        append_and_release(c1, out1);
        append_and_release(row, c1);

        HTMLElement c2 = make_el(doc, "div", "cell", 0);
        append_and_release(c2, make_el(doc, "div", "label", labels_disp[i]));
        HTMLElement out2 = make_el(doc, "div", "slot", 0);
        render_html(out2, texs[i], display_opts);
        append_and_release(c2, out2);
        append_and_release(row, c2);

        append_and_release(app, row);
    }

    js_release(inline_opts);
    js_release(display_opts);
    js_release(app);
    js_release(doc);
}
