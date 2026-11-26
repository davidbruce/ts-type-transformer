// marked from Rust, in Node: markdown to HTML, the lexer's tokens, a Marked
// instance with options, and extensions. Every check! must hold.
#![no_main]
use bindings::*;
use std::cell::Cell;
use std::rc::Rc;

#[link(wasm_import_module = "env")]
unsafe extern "C" {
    #[link_name = "check"]
    fn check_at(ok: i32, line: u32);
}
macro_rules! check {
    ($c:expr) => {
        unsafe { check_at($c as i32, line!()) }
    };
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Markdown to HTML.
    let out = marked("# Title\n\nSome *emphasis* and **strong** text.");
    check!(out.contains("<h1>Title</h1>"));
    check!(out.contains("<em>emphasis</em>"));
    check!(out.contains("<strong>strong</strong>"));
    check!(out.contains("<p>Some "));

    // Lists, code, links, quotes.
    let doc = "- one\n- two\n\n1. first\n2. second\n\n```js\nlet x = 1;\n```\n\n> quoted\n\n[link](https://example.com \"t\")";
    let out = marked(doc);
    check!(out.contains("<ul>\n<li>one</li>\n<li>two</li>\n</ul>"));
    check!(out.contains("<ol>"));
    check!(out.contains("<pre><code class=\"language-js\">let x = 1;\n</code></pre>"));
    check!(out.contains("<blockquote>\n<p>quoted</p>\n</blockquote>"));
    check!(out.contains("<a href=\"https://example.com\" title=\"t\">link</a>"));

    // GFM tables and task lists; and gfm switched off.
    let table = "| a | b |\n|---|:-:|\n| 1 | 2 |";
    check!(marked(table).contains("<table>"));
    let plain = MarkedExtension::create();
    plain.set_gfm(Some(false));
    check!(plain.gfm() == Some(false));
    let flat = Marked::new(Vec::<MarkedExtension>::new());
    flat.use_(vec![plain]);
    check!(!flat.parse(table).contains("<table>"));

    // Options: breaks turns a newline into <br>.
    let breaks = MarkedExtension::create();
    breaks.set_breaks(Some(true));
    let soft = Marked::new(Vec::<MarkedExtension>::new());
    soft.use_(vec![breaks]);
    check!(soft.parse("a\nb").contains("a<br>b"));
    check!(!marked("a\nb").contains("<br>"));

    // The library's defaults.
    let defaults = get_defaults();
    check!(defaults.gfm() == Some(true));
    check!(defaults.breaks() == Some(false));

    // An inline parse has no paragraph.
    // marked's `markedInstance` is not exported, so an instance is made here.
    let plain_instance = Marked::new(Vec::<MarkedExtension>::new());
    let inline = plain_instance.parse_inline("a *b* `c`");
    check!(inline == "a <em>b</em> <code>c</code>");

    // The lexer: tokens of a document, the type of each, and their children.
    let list = Lexer::lex_2("# Head\n\npara with **bold**\n\n- a\n- b\n");
    let types: Vec<String> = (0..list.length() as usize)
        .map(|i| list.at(i as f64).unwrap().type_())
        .collect();
    check!(types == ["heading", "space", "paragraph", "space", "list"]);
    let Token::Heading(heading) = list.at(0.0).unwrap() else {
        panic!("expected heading")
    };
    check!(heading.depth() == 1.0 && heading.text() == "Head");
    let Token::Paragraph(para) = list.at(2.0).unwrap() else {
        panic!("expected paragraph")
    };
    let children = para.tokens();
    check!(children.len() == 2);
    let child_types: Vec<String> = children.iter().map(|c| c.type_()).collect();
    check!(child_types == ["text", "strong"]);
    check!(para.raw().starts_with("para with"));
    let Token::List(list_token) = list.at(4.0).unwrap() else {
        panic!("expected list")
    };
    check!(list_token.ordered() == false && list_token.items().len() == 2);
    check!(list_token.items().get(1).text() == "b");

    // A Marked instance keeps its own options, apart from the global ones.
    let ext = MarkedExtension::create();
    ext.set_breaks(Some(true));
    let m = Marked::new(vec![ext]);
    check!(m.parse("x\ny").contains("x<br>y"));
    check!(m.defaults().breaks() == Some(true));
    check!(!marked("x\ny").contains("<br>"));
    let toks = m.lexer("hi *there*");
    check!(toks.length() == 1.0 && matches!(toks.at(0.0), Some(Token::Paragraph(_))));

    // A walkTokens extension sees every token, and may change them.
    let seen = Rc::new(Cell::new(0));
    let ext = MarkedExtension::create();
    let walk = MarkedExtensionWalkTokensFn::new({
        let seen = seen.clone();
        move |t: Token| {
            seen.set(seen.get() + 1);
            if let Token::Heading(h) = t {
                h.set_depth(2.0);
            }
            None
        }
    });
    ext.set_walk_tokens(Some(&walk));
    let m2 = Marked::new(vec![ext]);
    let out = m2.parse("# Top\n\ntext");
    check!(out.contains("<h2>Top</h2>"));
    check!(seen.get() >= 4);

    // A renderer extension replaces how one token is written.
    let ext = MarkedExtension::create();
    let renderer = RendererObject::create();
    let heading = RendererObjectHeadingFn::new(|h: tokens::Heading| {
        Some(format!(
            "<h{0} class=\"x\">{1}</h{0}>",
            h.depth() as u32,
            h.text()
        ))
    });
    renderer.set_heading(Some(&heading));
    ext.set_renderer(Some(&renderer));
    let m3 = Marked::new(vec![ext]);
    check!(m3.parse("## Sub") == "<h2 class=\"x\">Sub</h2>");
}
