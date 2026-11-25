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

// An instance with these options, set with use.
static Marked with_options(MarkedExtension opts) {
    Marked m = Marked_new(JsArray_MarkedExtension_new());
    Marked_use(m, JsArray_MarkedExtension_from(&opts, 1));
    return m;
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
    Marked gfm_on = with_options(MarkedExtension_create(.gfm = true));
    Marked gfm_off = with_options(MarkedExtension_create(.gfm = false));

    HTMLElement gfm_sec = make_el(doc, "div", "section", 0);
    HTMLElement_appendChild(gfm_sec, make_el(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    HTMLElement gfm_grid = make_el(doc, "div", "grid", 0);
    HTMLElement_appendChild(gfm_grid, make_pane(doc, "gfm: true", Marked_parse(gfm_on, gfm_src)));
    HTMLElement_appendChild(gfm_grid, make_pane(doc, "gfm: false", Marked_parse(gfm_off, gfm_src)));
    HTMLElement_appendChild(gfm_sec, gfm_grid);
    HTMLElement_appendChild(app, gfm_sec);

    const char* breaks_src = "Line one\nLine two\nLine three";
    Marked br_off = with_options(MarkedExtension_create(.breaks = false));
    Marked br_on = with_options(MarkedExtension_create(.breaks = true));

    HTMLElement br_sec = make_el(doc, "div", "section", 0);
    HTMLElement_appendChild(br_sec, make_el(doc, "div", "label", "Soft Line Breaks (breaks)"));
    HTMLElement br_grid = make_el(doc, "div", "grid", 0);
    HTMLElement_appendChild(br_grid, make_pane(doc, "breaks: false", Marked_parse(br_off, breaks_src)));
    HTMLElement_appendChild(br_grid, make_pane(doc, "breaks: true", Marked_parse(br_on, breaks_src)));
    HTMLElement_appendChild(br_sec, br_grid);
    HTMLElement_appendChild(app, br_sec);
}
