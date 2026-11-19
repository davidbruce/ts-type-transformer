#include "dom/dom.hpp"
#include "katex/katex.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

extern "C" __attribute__((export_name("start"))) void start() {
    dom::Document doc = dom::get_document();
    dom::HTMLElement app = *doc.getElementById("app");

    dom::HTMLElement head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "ParseError Inspector (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "throwOnError: false vs true"));
    app.appendChild(head);

    katex::KatexOptions lenient = katex::KatexOptions::create();
    lenient.throwOnError(false);
    lenient.errorColor("#f87171");

    katex::KatexOptions strict = katex::KatexOptions::create();
    strict.throwOnError(true);

    const char* bad_inputs[] = {
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b"
    };

    for (const char* tex : bad_inputs) {
        dom::HTMLElement row = make_el(doc, "div", "row", "");
        dom::HTMLElement left = make_el(doc, "div", "cell mode-html", "");
        left.appendChild(make_el(doc, "div", "label", "throwOnError: false (#f87171)"));
        dom::HTMLElement l_slot = make_el(doc, "div", "slot", "");
        l_slot.innerHTML(katex::renderToString(tex, lenient));
        left.appendChild(l_slot);

        dom::HTMLElement right = make_el(doc, "div", "cell", "");
        right.appendChild(make_el(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        ts::Result<std::string> res = ts::attempt([&] { return katex::renderToString(tex, strict); });
        std::string msg = "(did not throw)";
        if (!res) {
            katex::ParseError pe;
            static_cast<ts::Handle&>(pe) = res.error();
            msg = pe.message();
        }
        right.appendChild(make_el(doc, "div", "err", msg));

        row.appendChild(left);
        row.appendChild(right);
        app.appendChild(row);
    }
}
