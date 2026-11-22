#![no_main]

use dom::{Document, HTMLElement};

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

fn unwrap_html(r: marked::StringOrPromise) -> String {
    match r {
        marked::StringOrPromise::String(s) => s,
        marked::StringOrPromise::JsPromise(_) => String::new(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Marked Document Preview (Rust)")));
    append(&head, make_el(&doc, "span", "badge", Some("marked")));
    append(&app, head);

    let markdown_src = concat!(
        "# Marked Wireframe Spec\n\n",
        "## Features\n\n",
        "Typed bindings with *emphasis*, **strong**, `inline code`, and [reference](https://example.com \"Spec\").\n\n",
        "- Lexer AST\n",
        "- Custom renderer\n",
        "- GFM tables\n\n",
        "1. Parse\n",
        "2. Walk\n",
        "3. Render\n\n",
        "> Typed bindings from TypeScript definitions.\n\n",
        "```ts\n",
        "const ok: boolean = true;\n",
        "```\n\n",
        "---\n",
    );

    let grid = make_el(&doc, "div", "grid", None);

    let src_card = make_el(&doc, "div", "card", None);
    append(&src_card, make_el(&doc, "div", "label", Some("Markdown Source")));
    append(&src_card, make_el(&doc, "pre", "src", Some(markdown_src)));

    let out_card = make_el(&doc, "div", "card", None);
    append(&out_card, make_el(&doc, "div", "label", Some("Rendered HTML")));
    let md_out = make_el(&doc, "div", "md-out", None);
    let html = unwrap_html(marked::marked_with_opt_marked_options(markdown_src, None));
    md_out.set_inner_html(&html);
    append(&out_card, md_out);

    append(&grid, src_card);
    append(&grid, out_card);
    append(&app, grid);
}
