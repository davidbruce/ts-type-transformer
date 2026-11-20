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

static void u32_to_str(uint32_t v, char* buf) {
    char tmp[16];
    uint32_t len = 0;
    if (v == 0) {
        buf[0] = '0';
        buf[1] = '\0';
        return;
    }
    while (v > 0) {
        tmp[len++] = (char)('0' + (v % 10));
        v /= 10;
    }
    for (uint32_t i = 0; i < len; i++) {
        buf[i] = tmp[len - 1 - i];
    }
    buf[len] = '\0';
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    const char* sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";

    Marked m = Marked_new(JsArray_MarkedExtension_new());
    TokensList tokens = Marked_lexer(m, sample);
    uint32_t count = (uint32_t)TokensList_get_length(tokens);

    char badge_buf[32];
    char num_buf[16];
    u32_to_str(count, num_buf);
    uint32_t bp = 0;
    for (uint32_t i = 0; num_buf[i]; i++) badge_buf[bp++] = num_buf[i];
    const char* suffix = " tokens";
    for (uint32_t i = 0; suffix[i]; i++) badge_buf[bp++] = suffix[i];
    badge_buf[bp] = '\0';

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Marked Token Inspector (C)"));
    append_and_release(head, make_el(doc, "span", "badge", badge_buf));
    append_and_release(app, head);

    HTMLElement src_card = make_el(doc, "div", "card", 0);
    append_and_release(src_card, make_el(doc, "div", "label", "Markdown Input"));
    append_and_release(src_card, make_el(doc, "pre", "src", sample));
    append_and_release(app, src_card);

    HTMLElement tbl_card = make_el(doc, "div", "card", 0);
    append_and_release(tbl_card, make_el(doc, "div", "label", "Lexer Token Stream"));

    HTMLElement table = make_el(doc, "table", "tok-table", 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    const char* cols[] = { "#", "Type", "Bytes", "Raw Snippet" };
    for (int i = 0; i < 4; i++) {
        append_and_release(hrow, make_el(doc, "th", 0, cols[i]));
    }
    append_and_release(thead, hrow);
    append_and_release(table, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    for (uint32_t i = 0; i < count; i++) {
        JsValue tok = TokensList_get_at(tokens, (double)i);
        Tokens_Generic g = { tok.h };

        JsString kind = Tokens_Generic_get_type(g);
        char kind_buf[64];
        JsString_copy(kind, kind_buf, sizeof(kind_buf));
        js_release(kind);

        JsString raw = Tokens_Generic_get_raw(g);
        uint32_t raw_len = JsString_len(raw);
        static char raw_buf[512];
        static char snip_buf[1024];
        uint32_t rn = JsString_copy(raw, raw_buf, sizeof(raw_buf));
        if (rn >= sizeof(raw_buf)) rn = sizeof(raw_buf) - 1;
        js_release(raw);

        uint32_t sp = 0;
        for (uint32_t j = 0; j < rn && sp + 4 < sizeof(snip_buf); j++) {
            if (raw_buf[j] == '\n') {
                snip_buf[sp++] = (char)0xE2;
                snip_buf[sp++] = (char)0x86;
                snip_buf[sp++] = (char)0xB5;
            } else {
                snip_buf[sp++] = raw_buf[j];
            }
        }
        snip_buf[sp] = '\0';

        char idx_buf[16];
        char bytes_buf[16];
        u32_to_str(i, idx_buf);
        u32_to_str(raw_len, bytes_buf);

        HTMLElement tr = make_el(doc, "tr", 0, 0);
        append_and_release(tr, make_el(doc, "td", 0, idx_buf));
        append_and_release(tr, make_el(doc, "td", 0, kind_buf));
        append_and_release(tr, make_el(doc, "td", 0, bytes_buf));
        append_and_release(tr, make_el(doc, "td", 0, snip_buf));
        append_and_release(tbody, tr);

        js_release(tok);
    }

    append_and_release(table, tbody);
    append_and_release(tbl_card, table);
    append_and_release(app, tbl_card);

    js_release(tokens);
    js_release(m);
    js_release(app);
    js_release(doc);
}
