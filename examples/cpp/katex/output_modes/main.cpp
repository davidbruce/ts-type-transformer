#include "dom/dom.hpp"
#include "katex/katex.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static void add_card(dom::Document doc, dom::HTMLElement grid, std::string_view name, std::string_view cls, const std::string& html) {
    dom::HTMLElement card = make_el(doc, "div", cls.empty() ? "card" : std::string("card ") + std::string(cls), "");
    dom::HTMLElement row = make_el(doc, "div", "head", "");
    row.appendChild(make_el(doc, "div", "label", name));
    row.appendChild(make_el(doc, "span", "badge", std::to_string(html.size()) + " chars"));
    dom::HTMLElement slot = make_el(doc, "div", "slot", "");
    slot.innerHTML(html);
    card.appendChild(row);
    card.appendChild(slot);
    grid.appendChild(card);
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Output & Layout Options (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "html | mathml | leqno | fleqn | maxSize | strict"));
    app.appendChild(head);

    dom::HTMLElement grid = make_el(doc, "div", "grid", "");

    katex::KatexOptions o_html = katex::KatexOptions::create();
    o_html.output(katex::KatexOptions_output::html);
    add_card(doc, grid, "output: html", "mode-html", katex::renderToString("a^2 + b^2 = c^2", o_html));

    katex::KatexOptions o_math = katex::KatexOptions::create();
    o_math.output(katex::KatexOptions_output::mathml);
    add_card(doc, grid, "output: mathml", "", katex::renderToString("\\frac{1}{1 + x^2}", o_math));

    katex::KatexOptions o_both = katex::KatexOptions::create();
    o_both.output(katex::KatexOptions_output::htmlAndMathml);
    add_card(doc, grid, "output: htmlAndMathml", "", katex::renderToString("\\sqrt{\\alpha + \\beta}", o_both));

    katex::KatexOptions o_leq = katex::KatexOptions::create();
    o_leq.displayMode(true);
    o_leq.leqno(true);
    add_card(doc, grid, "displayMode + leqno", "", katex::renderToString("\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}", o_leq));

    katex::KatexOptions o_fl = katex::KatexOptions::create();
    o_fl.displayMode(true);
    o_fl.fleqn(true);
    o_fl.maxSize(2.0);
    o_fl.strict(false);
    add_card(doc, grid, "fleqn + maxSize(2) + strict(false)", "", katex::renderToString("\\text{café} + \\rule{10em}{0.4em} + é", o_fl));

    app.appendChild(grid);
}
