#![no_main]

use dom::{Document, HTMLElement};
use marked::{
    Marked, MarkedExtension, MarkedExtensionRenderer, MarkedExtensionRendererHeading,
    MarkedExtensionWalkTokens, StringOrBoolean, StringOrPromise, Token, tokens,
};
use std::cell::RefCell;
use std::rc::Rc;

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

fn token_type(t: &Token) -> String {
    match t {
        Token::Blockquote(_) => tokens::Blockquote::TYPE.into(),
        Token::Br(_) => tokens::Br::TYPE.into(),
        Token::Checkbox(_) => tokens::Checkbox::TYPE.into(),
        Token::Code(_) => tokens::Code::TYPE.into(),
        Token::Codespan(_) => tokens::Codespan::TYPE.into(),
        Token::Def(_) => tokens::Def::TYPE.into(),
        Token::Del(_) => tokens::Del::TYPE.into(),
        Token::Em(_) => tokens::Em::TYPE.into(),
        Token::Escape(_) => tokens::Escape::TYPE.into(),
        Token::Heading(_) => tokens::Heading::TYPE.into(),
        Token::Hr(_) => tokens::Hr::TYPE.into(),
        Token::HTML(_) => tokens::HTML::TYPE.into(),
        Token::Image(_) => tokens::Image::TYPE.into(),
        Token::Link(_) => tokens::Link::TYPE.into(),
        Token::List(_) => tokens::List::TYPE.into(),
        Token::ListItem(_) => tokens::ListItem::TYPE.into(),
        Token::Paragraph(_) => tokens::Paragraph::TYPE.into(),
        Token::Space(_) => tokens::Space::TYPE.into(),
        Token::Strong(_) => tokens::Strong::TYPE.into(),
        Token::Table(_) => tokens::Table::TYPE.into(),
        Token::Tag(_) => tokens::Tag::TYPE.into(),
        Token::Text(_) => tokens::Text::TYPE.into(),
        Token::Generic(g) => g.type_(),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn start() {
    let doc = dom::document();
    let app = doc.get_element_by_id("app").expect("missing #app");

    let md = "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n";

    let seen: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(Vec::new()));

    let heading = MarkedExtensionRendererHeading::new(|h: tokens::Heading| {
        StringOrBoolean::String(format!(
            "<h{0} class=\"wire-h\">\u{00A7} {1}</h{0}>\n",
            h.depth() as u32,
            h.text()
        ))
    });
    let renderer = MarkedExtensionRenderer::default().with_heading(&heading);

    let walk = MarkedExtensionWalkTokens::new({
        let seen = seen.clone();
        move |t: Token| {
            seen.borrow_mut().push(token_type(&t));
            None
        }
    });
    let ext = MarkedExtension::default()
        .with_renderer(&renderer)
        .with_walk_tokens(&walk);

    let custom_m = Marked::new(vec![ext]);
    let def_html = match marked::marked_with_opt_marked_options(md) {
        StringOrPromise::String(s) => s,
        StringOrPromise::JsPromise(_) => String::new(),
    };
    let cust_html = match custom_m.parse().invoke_2(md) {
        StringOrPromise::String(s) => s,
        StringOrPromise::JsPromise(_) => String::new(),
    };

    let visited = seen.borrow();

    let head = make_el(&doc, "div", "head", None);
    append(
        &head,
        make_el(
            &doc,
            "h1",
            "",
            Some("Marked Custom Renderer & Walker (Rust)"),
        ),
    );
    let badge_txt = format!("{} tokens visited", visited.len());
    append(&head, make_el(&doc, "span", "badge", Some(&badge_txt)));
    append(&app, head);

    let grid = make_el(&doc, "div", "grid", None);

    let def_card = make_el(&doc, "div", "card", None);
    append(
        &def_card,
        make_el(&doc, "div", "label", Some("Default Renderer")),
    );
    let def_out = make_el(&doc, "div", "md-out", None);
    def_out.set_inner_html(&def_html);
    append(&def_card, def_out);

    let cust_card = make_el(&doc, "div", "card", None);
    append(
        &cust_card,
        make_el(&doc, "div", "label", Some("Custom Heading Renderer")),
    );
    let cust_out = make_el(&doc, "div", "md-out", None);
    cust_out.set_inner_html(&cust_html);
    append(&cust_card, cust_out);

    append(&grid, def_card);
    append(&grid, cust_card);
    append(&app, grid);

    let walk_card = make_el(&doc, "div", "card", None);
    append(
        &walk_card,
        make_el(&doc, "div", "label", Some("Visited Tokens (walkTokens)")),
    );
    let strip = make_el(&doc, "div", "tok-strip", None);
    for kind in visited.iter() {
        append(&strip, make_el(&doc, "span", "tok-pill", Some(kind)));
    }
    append(&walk_card, strip);
    append(&app, walk_card);
}
