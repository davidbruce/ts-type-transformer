#![no_main]

use dom::{Document, HTMLElement};
use marked::{
    BooleanOrRendererOutput, JsArray, JsValue, Marked, MarkedExtension, MarkedExtensionRenderer,
    MarkedExtensionRendererHeading, ParserOutputOrPromise, StringOrPromise,
    TokenToVoidOrPromiseOrVoidOrPromiseArrayFn, tokens,
};
use std::cell::RefCell;
use std::rc::Rc;

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

    let md =
        "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n";

    let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let heading = MarkedExtensionRendererHeading::<String, String>::new(|args: JsArray| {
        let h = args.get(0).unchecked_into::<tokens::Heading>();
        BooleanOrRendererOutput::RendererOutput(format!(
            "<h{0} class=\"wire-h\">\u{00A7} {1}</h{0}>\n",
            h.depth() as u32,
            h.text()
        ))
    });
    let renderer = MarkedExtensionRenderer::<String, String>::default().with_heading(&heading);

    let walk = TokenToVoidOrPromiseOrVoidOrPromiseArrayFn::new({
        let seen = seen.clone();
        move |t: JsValue| {
            let g = t.unchecked_into::<tokens::Generic>();
            seen.borrow_mut().push(g.type_());
            None
        }
    });
    let ext = MarkedExtension::<String, String>::default()
        .with_renderer(&renderer)
        .with_walk_tokens(&walk);

    let custom_m = Marked::<String, String>::new(vec![ext]);
    let def_html = match marked::marked_with_opt_marked_options(md, None) {
        StringOrPromise::String(s) => s,
        StringOrPromise::JsPromise(_) => String::new(),
    };
    let cust_html = match custom_m.parse().invoke_2(md, None) {
        ParserOutputOrPromise::ParserOutput(s) => s,
        ParserOutputOrPromise::JsPromise(_) => String::new(),
    };

    let visited = seen.borrow();

    let head = make_el(&doc, "div", "head", None);
    append(&head, make_el(&doc, "h1", "", Some("Marked Custom Renderer & Walker (Rust)")));
    let badge_txt = format!("{} tokens visited", visited.len());
    append(&head, make_el(&doc, "span", "badge", Some(&badge_txt)));
    append(&app, head);

    let grid = make_el(&doc, "div", "grid", None);

    let def_card = make_el(&doc, "div", "card", None);
    append(&def_card, make_el(&doc, "div", "label", Some("Default Renderer")));
    let def_out = make_el(&doc, "div", "md-out", None);
    def_out.set_inner_html(&def_html);
    append(&def_card, def_out);

    let cust_card = make_el(&doc, "div", "card", None);
    append(&cust_card, make_el(&doc, "div", "label", Some("Custom Heading Renderer")));
    let cust_out = make_el(&doc, "div", "md-out", None);
    cust_out.set_inner_html(&cust_html);
    append(&cust_card, cust_out);

    append(&grid, def_card);
    append(&grid, cust_card);
    append(&app, grid);

    let walk_card = make_el(&doc, "div", "card", None);
    append(&walk_card, make_el(&doc, "div", "label", Some("Visited Tokens (walkTokens)")));
    let strip = make_el(&doc, "div", "tok-strip", None);
    for kind in visited.iter() {
        append(&strip, make_el(&doc, "span", "tok-pill", Some(kind)));
    }
    append(&walk_card, strip);
    append(&app, walk_card);
}
