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

static void fmt_chars(uint32_t n, char* out) {
    char rev[16];
    int k = 0;
    if (n == 0) rev[k++] = '0';
    while (n > 0) {
        rev[k++] = (char)('0' + (n % 10));
        n /= 10;
    }
    int pos = 0;
    for (int i = k - 1; i >= 0; i--) out[pos++] = rev[i];
    const char suffix[] = " chars";
    for (uint32_t i = 0; i < sizeof(suffix); i++) out[pos++] = suffix[i];
}

static void add_card(Document doc, HTMLElement grid, const char* label, const char* extra_cls, const char* tex, KatexOptions opts) {
    static char buf[1 << 16];
    char len_buf[32];
    JsString html = renderToString(tex, opts);
    uint32_t len = JsString_len(html);
    JsString_copy(html, buf, sizeof(buf));
    js_release(html);
    fmt_chars(len, len_buf);

    char cls[64] = "card";
    if (extra_cls && extra_cls[0]) {
        cls[4] = ' ';
        int p = 5;
        for (int i = 0; extra_cls[i] && p < 62; i++) cls[p++] = extra_cls[i];
        cls[p] = 0;
    }
    HTMLElement card = make_el(doc, "div", cls, 0);
    HTMLElement row = make_el(doc, "div", "head", 0);
    append_and_release(row, make_el(doc, "div", "label", label));
    append_and_release(row, make_el(doc, "span", "badge", len_buf));
    HTMLElement slot = make_el(doc, "div", "slot", 0);
    Element_set_innerHTML(slot, buf);
    append_and_release(card, row);
    append_and_release(card, slot);
    append_and_release(grid, card);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Output & Layout Options (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "html | mathml | leqno | fleqn | maxSize | strict"));
    append_and_release(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);

    KatexOptions o_html = KatexOptions_create();
    KatexOptions_set_output(o_html, KatexOptions_output_html);
    add_card(doc, grid, "output: html", "mode-html", "a^2 + b^2 = c^2", o_html);

    KatexOptions o_math = KatexOptions_create();
    KatexOptions_set_output(o_math, KatexOptions_output_mathml);
    add_card(doc, grid, "output: mathml", "", "\\frac{1}{1 + x^2}", o_math);

    KatexOptions o_both = KatexOptions_create();
    KatexOptions_set_output(o_both, KatexOptions_output_htmlAndMathml);
    add_card(doc, grid, "output: htmlAndMathml", "", "\\sqrt{\\alpha + \\beta}", o_both);

    KatexOptions o_leq = KatexOptions_create();
    KatexOptions_set_displayMode(o_leq, js_some_bool(true));
    KatexOptions_set_leqno(o_leq, js_some_bool(true));
    add_card(doc, grid, "displayMode + leqno", "", "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}", o_leq);

    KatexOptions o_fl = KatexOptions_create();
    KatexOptions_set_displayMode(o_fl, js_some_bool(true));
    KatexOptions_set_fleqn(o_fl, js_some_bool(true));
    KatexOptions_set_maxSize(o_fl, js_some_double(2.0));
    KatexOptions_set_strict(o_fl, boolean_or_string_or_StrictFunction_of_boolean(false));
    add_card(doc, grid, "fleqn + maxSize(2) + strict(false)", "", "\\text{café} + \\rule{10em}{0.4em} + é", o_fl);

    append_and_release(app, grid);
    js_release(o_html);
    js_release(o_math);
    js_release(o_both);
    js_release(o_leq);
    js_release(o_fl);
    js_release(app);
    js_release(doc);
}
