#include "dom/dom.h"
#include "katex/katex.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static void add_card(Document doc, HTMLElement grid, const char* label, const char* extra_cls, const char* tex, KatexOptions opts) {
    JsString html = katex_renderToString(tex, opts);
    char len_buf[32];
    snprintf(len_buf, sizeof len_buf, "%u chars", JsString_len(html));

    char cls[64];
    if (extra_cls && extra_cls[0]) snprintf(cls, sizeof cls, "card %s", extra_cls);
    else snprintf(cls, sizeof cls, "card");

    HTMLElement card = make_el(doc, "div", cls, 0);
    HTMLElement row = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(row, make_el(doc, "div", "label", label));
    HTMLElement_appendChild(row, make_el(doc, "span", "badge", len_buf));
    HTMLElement slot = make_el(doc, "div", "slot", 0);
    HTMLElement_set_innerHTML(slot, html);
    HTMLElement_appendChild(card, row);
    HTMLElement_appendChild(card, slot);
    HTMLElement_appendChild(grid, card);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Output & Layout Options (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "html | mathml | leqno | fleqn | maxSize | strict"));
    HTMLElement_appendChild(app, head);

    HTMLElement grid = make_el(doc, "div", "grid", 0);

    KatexOptions o_html = KatexOptions_create(.output = KatexOptions_output_html);
    add_card(doc, grid, "output: html", "mode-html", "a^2 + b^2 = c^2", o_html);

    KatexOptions o_math = KatexOptions_create(.output = KatexOptions_output_mathml);
    add_card(doc, grid, "output: mathml", "", "\\frac{1}{1 + x^2}", o_math);

    KatexOptions o_both = KatexOptions_create(.output = KatexOptions_output_htmlAndMathml);
    add_card(doc, grid, "output: htmlAndMathml", "", "\\sqrt{\\alpha + \\beta}", o_both);

    KatexOptions o_leq = KatexOptions_create(.displayMode = true, .leqno = true);
    add_card(doc, grid, "displayMode + leqno", "", "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}", o_leq);

    KatexOptions o_fl = KatexOptions_create(
        .displayMode = true, .fleqn = true, .maxSize = 2.0,
        .strict = Katex_boolean_or_string_or_StrictFunction_of(false)
    );
    add_card(doc, grid, "fleqn + maxSize(2) + strict(false)", "", "\\text{café} + \\rule{10em}{0.4em} + é", o_fl);

    HTMLElement_appendChild(app, grid);
}
