#![no_main]

use dom::{Document, HTMLElement};
use marked::{Lexer, Token, tokens};

fn token_kind_raw(t: &Token) -> (String, String) {
    match t {
        Token::Blockquote(x) => (tokens::Blockquote::TYPE.into(), x.raw()),
        Token::Br(x) => (tokens::Br::TYPE.into(), x.raw()),
        Token::Checkbox(x) => (tokens::Checkbox::TYPE.into(), x.raw()),
        Token::Code(x) => (tokens::Code::TYPE.into(), x.raw()),
        Token::Codespan(x) => (tokens::Codespan::TYPE.into(), x.raw()),
        Token::Def(x) => (tokens::Def::TYPE.into(), x.raw()),
        Token::Del(x) => (tokens::Del::TYPE.into(), x.raw()),
        Token::Em(x) => (tokens::Em::TYPE.into(), x.raw()),
        Token::Escape(x) => (tokens::Escape::TYPE.into(), x.raw()),
        Token::Heading(x) => (tokens::Heading::TYPE.into(), x.raw()),
        Token::Hr(x) => (tokens::Hr::TYPE.into(), x.raw()),
        Token::HTML(x) => (tokens::HTML::TYPE.into(), x.raw()),
        Token::Image(x) => (tokens::Image::TYPE.into(), x.raw()),
        Token::Link(x) => (tokens::Link::TYPE.into(), x.raw()),
        Token::List(x) => (tokens::List::TYPE.into(), x.raw()),
        Token::ListItem(x) => (tokens::ListItem::TYPE.into(), x.raw()),
        Token::Paragraph(x) => (tokens::Paragraph::TYPE.into(), x.raw()),
        Token::Space(x) => (tokens::Space::TYPE.into(), x.raw()),
        Token::Strong(x) => (tokens::Strong::TYPE.into(), x.raw()),
        Token::Table(x) => (tokens::Table::TYPE.into(), x.raw()),
        Token::Tag(x) => (tokens::Tag::TYPE.into(), x.raw()),
        Token::Text(x) => (tokens::Text::TYPE.into(), x.raw()),
        Token::Generic(x) => (x.type_(), x.raw()),
    }
}

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
        let (kind, raw) = token_kind_raw(&tok);
        let snip = raw.replace('\n', "\u{21B5}");
        let idx_s = i.to_string();
        let bytes_s = raw.len().to_string();

        let tr = make_el(&doc, "tr", "", None);
        append(&tr, make_el(&doc, "td", "", Some(&idx_s)));
        append(&tr, make_el(&doc, "td", "", Some(&kind)));
        append(&tr, make_el(&doc, "td", "", Some(&bytes_s)));
        append(&tr, make_el(&doc, "td", "", Some(&snip)));
        append(&tbody, tr);
    }

    append(&table, tbody);
    append(&tbl_card, table);
    append(&app, tbl_card);
}
