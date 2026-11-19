#![no_main]

use dom::{Document, DocumentMethods as _, ElementMethods as _, HTMLElement, NodeMethods};
use katex::{KatexOptions, KatexOptionsMethods as _};

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
    let _ = NodeMethods::append_child(parent, child);
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").unwrap();

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("KaTeX Formula Gallery (Rust)")));
    let badge_text = format!("katex v{}", katex::version());
    append(&head, make_el(&doc, "span", "badge", Some(&badge_text)));
    append(&app, head);

    let grid = make_el(&doc, "div", "grid", None);
    let opts = KatexOptions::create();
    opts.set_display_mode(Some(true));

    let formulas = [
        ("Euler's Identity", r"e^{i\pi} + 1 = 0"),
        ("Quadratic Formula", r"x = \frac{-b \pm \sqrt{b^2 - 4ac}}{2a}"),
        ("Gaussian Integral", r"\int_{-\infty}^{\infty} e^{-x^2}\,dx = \sqrt{\pi}"),
        ("Maxwell-Faraday", r"\nabla \times \mathbf{E} = -\frac{\partial \mathbf{B}}{\partial t}"),
        ("Fourier Transform", r"\hat{f}(\xi) = \int_{-\infty}^{\infty} f(x)e^{-2\pi i x \xi}\,dx"),
    ];

    for (name, tex) in formulas {
        let card = make_el(&doc, "div", "card", None);
        append(&card, make_el(&doc, "div", "label", Some(name)));
        append(&card, make_el(&doc, "pre", "tex", Some(tex)));
        let slot = make_el(&doc, "div", "slot", None);
        katex::render(tex, &slot, Some(&opts));
        append(&card, slot);
        append(&grid, card);
    }

    append(&app, grid);
}
