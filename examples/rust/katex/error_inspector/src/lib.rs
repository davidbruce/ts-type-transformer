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
    append(&head, make_el(&doc, "h1", "", Some("ParseError Inspector (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("throwOnError: false vs true")));
    append(&app, head);

    let lenient = KatexOptions::create();
    lenient.set_throw_on_error(Some(false));
    lenient.set_error_color(Some("#555555"));

    let strict = KatexOptions::create();
    strict.set_throw_on_error(Some(true));

    let bad_inputs = [
        r"\frac{1}{",
        r"\unknowncmd{x}",
        r"x^{2 + ",
        r"\sqrt{a + b",
    ];

    for tex in bad_inputs {
        let row = make_el(&doc, "div", "row", None);

        let left = make_el(&doc, "div", "cell mode-html", None);
        append(&left, make_el(&doc, "div", "label", Some("throwOnError: false (#555555)")));
        let l_slot = make_el(&doc, "div", "slot", None);
        l_slot.set_inner_html(&katex::render_to_string(tex, Some(&lenient)));
        append(&left, l_slot);
        append(&row, left);

        let right = make_el(&doc, "div", "cell", None);
        append(&right, make_el(&doc, "div", "label", Some("throwOnError: true (caught ParseError)")));
        let err_msg = match katex::js::attempt(|| katex::render_to_string(tex, Some(&strict))) {
            Ok(_) => String::from("(did not throw)"),
            Err(e) => e.message(),
        };
        append(&right, make_el(&doc, "div", "err", Some(&err_msg)));
        append(&row, right);

        append(&app, row);
    }
}
