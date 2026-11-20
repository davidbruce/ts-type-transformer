#include "dom/dom.hpp"
#include "katex/katex.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "ParseError Inspector (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "throwOnError: false vs true"));
    app.appendChild(head);

    auto lenient = katex::KatexOptions::create()
        .throwOnError(false)
        .errorColor("#555555");

    auto strict = katex::KatexOptions::create()
        .throwOnError(true);

    const char* bad_inputs[] = {
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b"
    };

    for (const char* tex : bad_inputs) {
        dom::HTMLElement row = make_el(doc, "div", "row", "");
        dom::HTMLElement left = make_el(doc, "div", "cell mode-html", "");
        left.appendChild(make_el(doc, "div", "label", "throwOnError: false (#555555)"));
        dom::HTMLElement l_slot = make_el(doc, "div", "slot", "");
        l_slot.innerHTML(katex::renderToString(tex, lenient));
        left.appendChild(l_slot);

        dom::HTMLElement right = make_el(doc, "div", "cell", "");
        right.appendChild(make_el(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        ts::Result<std::string> res = ts::attempt([&] { return katex::renderToString(tex, strict); });
        std::string msg = res ? "(did not throw)" : res.error().message();
        right.appendChild(make_el(doc, "div", "err", msg));

        row.appendChild(left);
        row.appendChild(right);
        app.appendChild(row);
    }
}
