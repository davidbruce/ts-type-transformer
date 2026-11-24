#![no_main]

use dom::{Document, HTMLElement};
use katex::KatexOptions;

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

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(
        &head,
        make_el(&doc, "h1", "", Some("Inline vs Display Mode (Rust)")),
    );
    append(
        &head,
        make_el(&doc, "span", "badge", Some("displayMode: false vs true")),
    );
    append(&app, head);

    let inline_opts = KatexOptions::default().with_display_mode(false);
    let display_opts = KatexOptions::default().with_display_mode(true);

    let items = [
        (
            "Basel Series",
            r"\sum_{k=1}^{n} \frac{1}{k^2} = \frac{\pi^2}{6}",
        ),
        ("Dilogarithm", r"\int_0^1 \frac{\ln(1+x)}{x}\,dx"),
        ("Sinc Limit", r"\lim_{x \to 0} \frac{\sin x}{x} = 1"),
        ("Binomial", r"\binom{n}{k} = \frac{n!}{k!(n-k)!}"),
    ];

    for (name, tex) in items {
        let row = make_el(&doc, "div", "row", None);
        for (suffix, opts) in [("inline", &inline_opts), ("display", &display_opts)] {
            let cell = make_el(&doc, "div", "cell", None);
            let label = format!("{name} — {suffix}");
            append(&cell, make_el(&doc, "div", "label", Some(&label)));
            let slot = make_el(&doc, "div", "slot", None);
            slot.set_inner_html(&katex::render_to_string_with(tex, opts));
            append(&cell, slot);
            append(&row, cell);
        }
        append(&app, row);
    }
}
