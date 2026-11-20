#![no_main]

use dom::{Document, HTMLElement};
use katex::{KatexOptions, KatexOptionsOutput};

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

fn add_card(doc: &Document, grid: &HTMLElement, label: &str, extra_cls: &str, tex: &str, opts: &KatexOptions) {
    let html = katex::render_to_string(tex, Some(opts));
    let cls = if extra_cls.is_empty() { String::from("card") } else { format!("card {extra_cls}") };
    let card = make_el(doc, "div", &cls, None);
    let row = make_el(doc, "div", "head", None);
    append(&row, make_el(doc, "div", "label", Some(label)));
    let len_str = format!("{} chars", html.len());
    append(&row, make_el(doc, "span", "badge", Some(&len_str)));
    append(&card, row);
    let slot = make_el(doc, "div", "slot", None);
    slot.set_inner_html(&html);
    append(&card, slot);
    append(grid, card);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Output & Layout Options (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("html | mathml | leqno | fleqn | maxSize | strict")));
    append(&app, head);

    let grid = make_el(&doc, "div", "grid", None);

    let o_html = KatexOptions::default().with_output(KatexOptionsOutput::Html);
    add_card(&doc, &grid, "output: html", "mode-html", "a^2 + b^2 = c^2", &o_html);

    let o_math = KatexOptions::default().with_output(KatexOptionsOutput::Mathml);
    add_card(&doc, &grid, "output: mathml", "", r"\frac{1}{1 + x^2}", &o_math);

    let o_both = KatexOptions::default().with_output(KatexOptionsOutput::HtmlAndMathml);
    add_card(&doc, &grid, "output: htmlAndMathml", "", r"\sqrt{\alpha + \beta}", &o_both);

    let o_leq = KatexOptions::default()
        .with_display_mode(true)
        .with_leqno(true);
    add_card(&doc, &grid, "displayMode + leqno", "", r"\int_0^1 x^2\,dx = \frac{1}{3} \tag{1}", &o_leq);

    let o_fl = KatexOptions::default()
        .with_display_mode(true)
        .with_fleqn(true)
        .with_max_size(2.0)
        .with_strict(false);
    add_card(&doc, &grid, "fleqn + maxSize(2) + strict(false)", "", r"\text{café} + \rule{10em}{0.4em} + é", &o_fl);

    append(&app, grid);
}
