#include "dom/dom.hpp"
#include "marked/marked.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static dom::HTMLElement make_pane(dom::Document doc, std::string_view title, const std::string& html) {
    dom::HTMLElement card = make_el(doc, "div", "card", "");
    card.appendChild(make_el(doc, "div", "label", title));
    dom::HTMLElement out = make_el(doc, "div", "md-out", "");
    out.innerHTML(html);
    card.appendChild(out);
    card.appendChild(make_el(doc, "pre", "html-raw", html));
    return card;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Marked GFM & Line Breaks (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "MarkedOptions"));
    app.appendChild(head);

    std::string gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    auto gfm_on = marked::marked_options_no_async::create().gfm(true);
    auto gfm_off = marked::marked_options_no_async::create().gfm(false);

    auto gfm_sec = make_el(doc, "div", "section", "");
    gfm_sec.appendChild(make_el(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    auto gfm_grid = make_el(doc, "div", "grid", "");
    gfm_grid.appendChild(make_pane(doc, "gfm: true", marked::marked(gfm_src, gfm_on)));
    gfm_grid.appendChild(make_pane(doc, "gfm: false", marked::marked(gfm_src, gfm_off)));
    gfm_sec.appendChild(gfm_grid);
    app.appendChild(gfm_sec);

    std::string breaks_src = "Line one\nLine two\nLine three";
    auto br_off = marked::marked_options_no_async::create().breaks(false);
    auto br_on = marked::marked_options_no_async::create().breaks(true);

    dom::HTMLElement br_sec = make_el(doc, "div", "section", "");
    br_sec.appendChild(make_el(doc, "div", "label", "Soft Line Breaks (breaks)"));
    dom::HTMLElement br_grid = make_el(doc, "div", "grid", "");
    br_grid.appendChild(make_pane(doc, "breaks: false", marked::marked(breaks_src, br_off)));
    br_grid.appendChild(make_pane(doc, "breaks: true", marked::marked(breaks_src, br_on)));
    br_sec.appendChild(br_grid);
    app.appendChild(br_sec);
}
