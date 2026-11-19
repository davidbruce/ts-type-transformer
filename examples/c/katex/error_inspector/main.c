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

__attribute__((export_name("start"))) void start(void) {
    static char html_buf[1 << 16];
    static char err_buf[512];
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "ParseError Inspector (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "throwOnError: false vs true"));
    append_and_release(app, head);

    KatexOptions lenient = KatexOptions_create();
    KatexOptions_set_throwOnError(lenient, js_some_bool(false));
    KatexOptions_set_errorColor(lenient, "#555555");

    KatexOptions strict = KatexOptions_create();
    KatexOptions_set_throwOnError(strict, js_some_bool(true));

    const char* bad_inputs[] = {
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b"
    };

    for (int i = 0; i < 4; i++) {
        HTMLElement row = make_el(doc, "div", "row", 0);
        HTMLElement left = make_el(doc, "div", "cell mode-html", 0);
        append_and_release(left, make_el(doc, "div", "label", "throwOnError: false (#555555)"));
        HTMLElement l_slot = make_el(doc, "div", "slot", 0);
        JsString html = renderToString(bad_inputs[i], lenient);
        JsString_copy(html, html_buf, sizeof(html_buf));
        Element_set_innerHTML(l_slot, html_buf);
        js_release(html);
        append_and_release(left, l_slot);

        HTMLElement right = make_el(doc, "div", "cell", 0);
        append_and_release(right, make_el(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        HTMLElement r_out = make_el(doc, "div", "err", 0);
        JsString res = renderToString(bad_inputs[i], strict);
        if (js_failed()) {
            JsValue err = js_take_error();
            JsString msg = js_error_message(err);
            JsString_copy(msg, err_buf, sizeof(err_buf));
            Node_set_textContent(r_out, err_buf);
            js_release(msg);
            js_release(err);
        } else {
            Node_set_textContent(r_out, "(did not throw)");
        }
        if (res.h) js_release(res);
        append_and_release(right, r_out);

        append_and_release(row, left);
        append_and_release(row, right);
        append_and_release(app, row);
    }

    js_release(lenient);
    js_release(strict);
    js_release(app);
    js_release(doc);
}
