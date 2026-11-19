#include "dom/dom.h"
#include "marked/marked.h"

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

static HTMLElement make_pane(Document doc, const char* title, JsString html) {
    static char buf[4096];
    uint32_t n = JsString_copy(html, buf, sizeof(buf) - 1);
    buf[n] = '\0';

    HTMLElement card = make_el(doc, "div", "card", 0);
    append_and_release(card, make_el(doc, "div", "label", title));
    HTMLElement out = make_el(doc, "div", "md-out", 0);
    Element_set_innerHTML(out, buf);
    append_and_release(card, out);
    append_and_release(card, make_el(doc, "pre", "html-raw", buf));
    return card;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Marked GFM & Line Breaks (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "MarkedOptions"));
    append_and_release(app, head);

    const char* gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    marked_options_no_async gfm_on = marked_options_no_async_create();
    marked_options_no_async_set_gfm(gfm_on, js_some_bool(true));
    marked_options_no_async gfm_off = marked_options_no_async_create();
    marked_options_no_async_set_gfm(gfm_off, js_some_bool(false));

    JsString gfm_on_html = marked(gfm_src, gfm_on);
    JsString gfm_off_html = marked(gfm_src, gfm_off);

    HTMLElement gfm_sec = make_el(doc, "div", "section", 0);
    append_and_release(gfm_sec, make_el(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    HTMLElement gfm_grid = make_el(doc, "div", "grid", 0);
    append_and_release(gfm_grid, make_pane(doc, "gfm: true", gfm_on_html));
    append_and_release(gfm_grid, make_pane(doc, "gfm: false", gfm_off_html));
    append_and_release(gfm_sec, gfm_grid);
    append_and_release(app, gfm_sec);

    const char* breaks_src = "Line one\nLine two\nLine three";
    marked_options_no_async br_off = marked_options_no_async_create();
    marked_options_no_async_set_breaks(br_off, js_some_bool(false));
    marked_options_no_async br_on = marked_options_no_async_create();
    marked_options_no_async_set_breaks(br_on, js_some_bool(true));

    JsString br_off_html = marked(breaks_src, br_off);
    JsString br_on_html = marked(breaks_src, br_on);

    HTMLElement br_sec = make_el(doc, "div", "section", 0);
    append_and_release(br_sec, make_el(doc, "div", "label", "Soft Line Breaks (breaks)"));
    HTMLElement br_grid = make_el(doc, "div", "grid", 0);
    append_and_release(br_grid, make_pane(doc, "breaks: false", br_off_html));
    append_and_release(br_grid, make_pane(doc, "breaks: true", br_on_html));
    append_and_release(br_sec, br_grid);
    append_and_release(app, br_sec);

    js_release(gfm_on_html);
    js_release(gfm_off_html);
    js_release(gfm_on);
    js_release(gfm_off);
    js_release(br_off_html);
    js_release(br_on_html);
    js_release(br_off);
    js_release(br_on);
    js_release(app);
    js_release(doc);
}
