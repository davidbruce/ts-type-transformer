#![no_main]

use dom::{Document, HTMLElement};
use katex::{KatexOptions, KatexOptionsMacros};

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
    append(&head, make_el(&doc, "h1", "", Some("Custom Macros (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some(r"\RR, \NN, \dd, \norm")));
    append(&app, head);

    let macros = KatexOptionsMacros::default();
    macros.set_at(r"\RR", r"\mathbb{R}");
    macros.set_at(r"\NN", r"\mathbb{N}");
    macros.set_at(r"\dd", r"\mathrm{d}");
    macros.set_at(r"\norm", r"\left\| #1 \right\|");

    let opts = KatexOptions::default()
        .with_display_mode(true)
        .with_macros(&macros);

    let legend = make_el(&doc, "div", "card", None);
    append(&legend, make_el(&doc, "div", "label", Some("Defined Macros")));
    append(
        &legend,
        make_el(&doc, "pre", "tex", Some("\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|")),
    );
    append(&app, legend);

    let grid = make_el(&doc, "div", "grid", None);
    let exprs = [
        ("Euclidean Space", r"v \in \RR^n,\quad k \in \NN"),
        ("Vector Norm", r"\norm{x + y} \le \norm{x} + \norm{y}"),
        ("Differential Form", r"\int_a^b f(x)\,\dd x = F(b) - F(a)"),
        ("Inner Product Norm", r"\norm{\phi}_2 = \sqrt{\int_{\RR} |\phi(t)|^2\,\dd t}"),
    ];

    for (name, tex) in exprs {
        let card = make_el(&doc, "div", "card", None);
        append(&card, make_el(&doc, "div", "label", Some(name)));
        append(&card, make_el(&doc, "pre", "tex", Some(tex)));
        let slot = make_el(&doc, "div", "slot", None);
        slot.set_inner_html(&katex::render_to_string(tex, &opts));
        append(&card, slot);
        append(&grid, card);
    }

    append(&app, grid);
}
