#![no_main]

use dom::{Document, HTMLElement};
use marked::{Marked, MarkedExtension};

fn make_el(doc: &Document, tag: &str, cls: &str, text: Option<&str>) -> HTMLElement {
    let el = doc.create_element(tag, None);
    if !cls.is_empty() {
        el.set_class_name(cls);
    }
    if let Some(t) = text {
        el.set_text_content(t);
    }
    el
}

fn append(parent: &HTMLElement, child: HTMLElement) {
    parent.append_child(child);
}

fn make_result_card(doc: &Document, title: &str, html: &str) -> HTMLElement {
    let card = make_el(doc, "div", "card", None);
    append(&card, make_el(doc, "div", "label", Some(title)));
    let out = make_el(doc, "div", "md-out", None);
    out.set_inner_html(html);
    append(&card, out);
    append(&card, make_el(doc, "pre", "html-raw", Some(html)));
    card
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(
        &head,
        make_el(&doc, "h1", "", Some("Marked Inline vs Block (Rust)")),
    );
    append(
        &head,
        make_el(&doc, "span", "badge", Some("parse vs parseInline")),
    );
    append(&app, head);

    let m = Marked::new(Vec::<MarkedExtension>::new());
    let snippets = [
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***",
    ];

    for s in snippets {
        let row = make_el(&doc, "div", "row-card", None);
        append(
            &row,
            make_el(&doc, "div", "label", Some("Markdown Snippet")),
        );
        append(&row, make_el(&doc, "pre", "src", Some(s)));

        let grid = make_el(&doc, "div", "grid", None);
        let block_html = m.parse(s);
        let inl_html = m.parse_inline(s);
        append(&grid, make_result_card(&doc, "Block (parse)", &block_html));
        append(
            &grid,
            make_result_card(&doc, "Inline (parseInline)", &inl_html),
        );
        append(&row, grid);
        append(&app, row);
    }
}
