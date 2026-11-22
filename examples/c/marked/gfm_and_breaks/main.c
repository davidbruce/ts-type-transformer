#include "dom/dom.h"
#include "marked/marked.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static HTMLElement make_pane(Document doc, const char* title, JsString html) {
    HTMLElement card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(card, make_el(doc, "div", "label", title));
    HTMLElement out = make_el(doc, "div", "md-out", 0);
    HTMLElement_set_innerHTML(out, html);
    HTMLElement_appendChild(card, out);
    HTMLElement raw = make_el(doc, "pre", "html-raw", 0);
    HTMLElement_set_textContent(raw, html);
    HTMLElement_appendChild(card, raw);
    return card;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Marked GFM & Line Breaks (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "MarkedOptions"));
    HTMLElement_appendChild(app, head);

    const char* gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    marked_options_no_async gfm_on = marked_options_no_async_create(.gfm = true);
    marked_options_no_async gfm_off = marked_options_no_async_create(.gfm = false);

    HTMLElement gfm_sec = make_el(doc, "div", "section", 0);
    HTMLElement_appendChild(gfm_sec, make_el(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    HTMLElement gfm_grid = make_el(doc, "div", "grid", 0);
    HTMLElement_appendChild(gfm_grid, make_pane(doc, "gfm: true", marked(gfm_src, gfm_on)));
    HTMLElement_appendChild(gfm_grid, make_pane(doc, "gfm: false", marked(gfm_src, gfm_off)));
    HTMLElement_appendChild(gfm_sec, gfm_grid);
    HTMLElement_appendChild(app, gfm_sec);

    const char* breaks_src = "Line one\nLine two\nLine three";
    marked_options_no_async br_off = marked_options_no_async_create(.breaks = false);
    marked_options_no_async br_on = marked_options_no_async_create(.breaks = true);

    HTMLElement br_sec = make_el(doc, "div", "section", 0);
    HTMLElement_appendChild(br_sec, make_el(doc, "div", "label", "Soft Line Breaks (breaks)"));
    HTMLElement br_grid = make_el(doc, "div", "grid", 0);
    HTMLElement_appendChild(br_grid, make_pane(doc, "breaks: false", marked(breaks_src, br_off)));
    HTMLElement_appendChild(br_grid, make_pane(doc, "breaks: true", marked(breaks_src, br_on)));
    HTMLElement_appendChild(br_sec, br_grid);
    HTMLElement_appendChild(app, br_sec);
}
