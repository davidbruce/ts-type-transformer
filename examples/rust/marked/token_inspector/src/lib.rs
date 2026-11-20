#![no_main]

use dom::{Document, HTMLElement};
use marked::{Lexer, js, tokens};

fn make_el(doc: &Document, tag: &str, cls: &str, text: Option<&str>) -> HTMLElement {
    let el = doc.create_element(tag, None);
    if !cls.is_empty() {
        el.set_class_name(cls);
    }
    if let Some(t) = text {
        el.set_text_content(Some(t));
    }
    el
}

fn append(parent: &HTMLElement, child: HTMLElement) {
    parent.append_child(child);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";
    let list = Lexer::<String, String>::lex::<String, String>(sample, None);
    let count = list.length() as usize;

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Marked Token Inspector (Rust)")));
    let badge_text = format!("{count} tokens");
    append(&head, make_el(&doc, "span", "badge", Some(&badge_text)));
    append(&app, head);

    let src_card = make_el(&doc, "div", "card", None);
    append(&src_card, make_el(&doc, "div", "label", Some("Markdown Input")));
    append(&src_card, make_el(&doc, "pre", "src", Some(sample)));
    append(&app, src_card);

    let tbl_card = make_el(&doc, "div", "card", None);
    append(&tbl_card, make_el(&doc, "div", "label", Some("Lexer Token Stream")));

    let table = make_el(&doc, "table", "tok-table", None);
    let thead = make_el(&doc, "thead", "", None);
    let hrow = make_el(&doc, "tr", "", None);
    for col in ["#", "Type", "Bytes", "Raw Snippet"] {
        append(&hrow, make_el(&doc, "th", "", Some(col)));
    }
    append(&thead, hrow);
    append(&table, thead);

    let tbody = make_el(&doc, "tbody", "", None);
    for i in 0..count {
        let tok = list.at(i as f64).expect("missing token");
        let g = js::unchecked_cast::<tokens::Generic>(&tok);
        let raw = g.raw();
        let snip = raw.replace('\n', "\u{21B5}");
        let idx_s = i.to_string();
        let bytes_s = raw.len().to_string();

        let tr = make_el(&doc, "tr", "", None);
        append(&tr, make_el(&doc, "td", "", Some(&idx_s)));
        append(&tr, make_el(&doc, "td", "", Some(&g.type_())));
        append(&tr, make_el(&doc, "td", "", Some(&bytes_s)));
        append(&tr, make_el(&doc, "td", "", Some(&snip)));
        append(&tbody, tr);
    }

    append(&table, tbody);
    append(&tbl_card, table);
    append(&app, tbl_card);
}
