#include "dom/dom.h"
#include "marked/marked.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static uint32_t token_h(Marked_Token tok) {
    switch (tok.tag) {
    case Marked_Token_Marked_Tokens_Blockquote: return tok.as.Marked_Tokens_Blockquote.h;
    case Marked_Token_Marked_Tokens_Br: return tok.as.Marked_Tokens_Br.h;
    case Marked_Token_Marked_Tokens_Checkbox: return tok.as.Marked_Tokens_Checkbox.h;
    case Marked_Token_Marked_Tokens_Code: return tok.as.Marked_Tokens_Code.h;
    case Marked_Token_Marked_Tokens_Codespan: return tok.as.Marked_Tokens_Codespan.h;
    case Marked_Token_Marked_Tokens_Def: return tok.as.Marked_Tokens_Def.h;
    case Marked_Token_Marked_Tokens_Del: return tok.as.Marked_Tokens_Del.h;
    case Marked_Token_Marked_Tokens_Em: return tok.as.Marked_Tokens_Em.h;
    case Marked_Token_Marked_Tokens_Escape: return tok.as.Marked_Tokens_Escape.h;
    case Marked_Token_Marked_Tokens_Heading: return tok.as.Marked_Tokens_Heading.h;
    case Marked_Token_Marked_Tokens_Hr: return tok.as.Marked_Tokens_Hr.h;
    case Marked_Token_Marked_Tokens_HTML: return tok.as.Marked_Tokens_HTML.h;
    case Marked_Token_Marked_Tokens_Image: return tok.as.Marked_Tokens_Image.h;
    case Marked_Token_Marked_Tokens_Link: return tok.as.Marked_Tokens_Link.h;
    case Marked_Token_Marked_Tokens_List: return tok.as.Marked_Tokens_List.h;
    case Marked_Token_Marked_Tokens_ListItem: return tok.as.Marked_Tokens_ListItem.h;
    case Marked_Token_Marked_Tokens_Paragraph: return tok.as.Marked_Tokens_Paragraph.h;
    case Marked_Token_Marked_Tokens_Space: return tok.as.Marked_Tokens_Space.h;
    case Marked_Token_Marked_Tokens_Strong: return tok.as.Marked_Tokens_Strong.h;
    case Marked_Token_Marked_Tokens_Table: return tok.as.Marked_Tokens_Table.h;
    case Marked_Token_Marked_Tokens_Tag: return tok.as.Marked_Tokens_Tag.h;
    case Marked_Token_Marked_Tokens_Text: return tok.as.Marked_Tokens_Text.h;
    case Marked_Token_Marked_Tokens_Generic: return tok.as.Marked_Tokens_Generic.h;
    default: return 0;
    }
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
        Marked_Tokens_Generic g = { token_h(Marked_TokensList_get_at(tokens, (double)i)) };

        JsString raw = Marked_Tokens_Generic_get_raw(g);
        uint32_t raw_len = JsString_len(raw);
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
        HTMLElement_set_textContent(td_kind, Marked_Tokens_Generic_get_type(g));
        HTMLElement_appendChild(tr, td_kind);
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, bytes_buf));
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, snip_buf));
        HTMLElement_appendChild(tbody, tr);
    }

    HTMLElement_appendChild(table, tbody);
    HTMLElement_appendChild(tbl_card, table);
    HTMLElement_appendChild(app, tbl_card);
}
