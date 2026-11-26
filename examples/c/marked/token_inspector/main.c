#include "dom/dom.h"
#include "marked/marked.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    const char* sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";

    Marked m = Marked_new();
    Marked_TokensList tokens = Marked_lexer(m, sample);
    uint32_t count = (uint32_t)Marked_TokensList_get_length(tokens);

    char badge_buf[32];
    snprintf(badge_buf, sizeof badge_buf, "%u tokens", count);

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Marked Token Inspector (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", badge_buf));
    HTMLElement_appendChild(app, head);

    HTMLElement src_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(src_card, make_el(doc, "div", "label", "Markdown Input"));
    HTMLElement_appendChild(src_card, make_el(doc, "pre", "src", sample));
    HTMLElement_appendChild(app, src_card);

    HTMLElement tbl_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(tbl_card, make_el(doc, "div", "label", "Lexer Token Stream"));

    HTMLElement table = make_el(doc, "table", "tok-table", 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    const char* cols[] = { "#", "Type", "Bytes", "Raw Snippet" };
    for (int i = 0; i < 4; i++) {
        HTMLElement_appendChild(hrow, make_el(doc, "th", 0, cols[i]));
    }
    HTMLElement_appendChild(thead, hrow);
    HTMLElement_appendChild(table, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    for (uint32_t i = 0; i < count; i++) {
        Marked_Token tok = Marked_TokensList_get_at(tokens, (double)i);

        JsString raw = Marked_Token_get_raw(tok);
        uint32_t raw_len = js_len(raw);
        char raw_buf[512], snip_buf[1024];
        uint32_t rn = JsString_copy(raw, raw_buf, sizeof raw_buf);
        if (rn >= sizeof raw_buf) rn = sizeof raw_buf - 1;

        uint32_t sp = 0;
        for (uint32_t j = 0; j < rn && sp + 4 < sizeof snip_buf; j++) {
            if (raw_buf[j] == '\n') {
                snip_buf[sp++] = (char)0xE2;
                snip_buf[sp++] = (char)0x86;
                snip_buf[sp++] = (char)0xB5;
            } else {
                snip_buf[sp++] = raw_buf[j];
            }
        }
        snip_buf[sp] = '\0';

        char idx_buf[16], bytes_buf[16];
        snprintf(idx_buf, sizeof idx_buf, "%u", i);
        snprintf(bytes_buf, sizeof bytes_buf, "%u", raw_len);

        HTMLElement tr = make_el(doc, "tr", 0, 0);
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, idx_buf));
        HTMLElement td_kind = make_el(doc, "td", 0, 0);
        HTMLElement_set_textContent(td_kind, Marked_Token_get_type(tok));
        HTMLElement_appendChild(tr, td_kind);
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, bytes_buf));
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, snip_buf));
        HTMLElement_appendChild(tbody, tr);
    }

    HTMLElement_appendChild(table, tbody);
    HTMLElement_appendChild(tbl_card, table);
    HTMLElement_appendChild(app, tbl_card);
}
