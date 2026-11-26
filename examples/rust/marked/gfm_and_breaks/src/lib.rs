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

fn render_with(src: &str, ext: &MarkedExtension) -> String {
    Marked::new([ext.clone()]).parse(src)
}

fn make_pane(doc: &Document, title: &str, html: &str) -> HTMLElement {
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
        make_el(&doc, "h1", "", Some("Marked GFM & Line Breaks (Rust)")),
    );
    append(&head, make_el(&doc, "span", "badge", Some("MarkedOptions")));
    append(&app, head);

    let gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    let gfm_on = MarkedExtension::default().with_gfm(true);
    let gfm_off = MarkedExtension::default().with_gfm(false);

    let gfm_sec = make_el(&doc, "div", "section", None);
    append(
        &gfm_sec,
        make_el(&doc, "div", "label", Some("GitHub Flavored Markdown (gfm)")),
    );
    let gfm_grid = make_el(&doc, "div", "grid", None);
    append(
        &gfm_grid,
        make_pane(&doc, "gfm: true", &render_with(gfm_src, &gfm_on)),
    );
    append(
        &gfm_grid,
        make_pane(&doc, "gfm: false", &render_with(gfm_src, &gfm_off)),
    );
    append(&gfm_sec, gfm_grid);
    append(&app, gfm_sec);

    let breaks_src = "Line one\nLine two\nLine three";
    let br_off = MarkedExtension::default().with_breaks(false);
    let br_on = MarkedExtension::default().with_breaks(true);

    let br_sec = make_el(&doc, "div", "section", None);
    append(
        &br_sec,
        make_el(&doc, "div", "label", Some("Soft Line Breaks (breaks)")),
    );
    let br_grid = make_el(&doc, "div", "grid", None);
    append(
        &br_grid,
        make_pane(&doc, "breaks: false", &render_with(breaks_src, &br_off)),
    );
    append(
        &br_grid,
        make_pane(&doc, "breaks: true", &render_with(breaks_src, &br_on)),
    );
    append(&br_sec, br_grid);
    append(&app, br_sec);
}
