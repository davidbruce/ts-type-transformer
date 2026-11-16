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

fn html(r: StringOrPromise) -> String {
    match r {
        StringOrPromise::String(s) => s,
        StringOrPromise::JsPromise(_) => String::from("<promise>"),
    }
}

// A token out of a token list, as the generic token every token is.
fn token(list: &TokensList, i: usize) -> tokens::Generic {
    tokens::Generic(list.at(i as f64).unwrap().0.clone())
}

#[unsafe(no_mangle)]
pub extern "C" fn run() {
    // Markdown to HTML.
    let out = html(marked_with_opt_marked_options("# Title\n\nSome *emphasis* and **strong** text.", None));
    check!(out.contains("<h1>Title</h1>"));
    check!(out.contains("<em>emphasis</em>"));
    check!(out.contains("<strong>strong</strong>"));
    check!(out.contains("<p>Some "));

    // Lists, code, links, quotes.
    let doc = "- one\n- two\n\n1. first\n2. second\n\n```js\nlet x = 1;\n```\n\n> quoted\n\n[link](https://example.com \"t\")";
    let out = html(marked_with_opt_marked_options(doc, None));
    check!(out.contains("<ul>\n<li>one</li>\n<li>two</li>\n</ul>"));
    check!(out.contains("<ol>"));
    check!(out.contains("<pre><code class=\"language-js\">let x = 1;\n</code></pre>"));
    check!(out.contains("<blockquote>\n<p>quoted</p>\n</blockquote>"));
    check!(out.contains("<a href=\"https://example.com\" title=\"t\">link</a>"));

    // GFM tables and task lists; and gfm switched off.
    let table = "| a | b |\n|---|:-:|\n| 1 | 2 |";
    check!(html(marked_with_opt_marked_options(table, None)).contains("<table>"));
    let plain = MarkedOptions::<String, String>::create();
    plain.set_gfm(Some(false));
    check!(plain.gfm() == Some(false));
    check!(!html(marked_with_opt_marked_options(table, Some(&plain))).contains("<table>"));

    // Options: breaks turns a newline into <br>.
    let breaks = MarkedOptions::<String, String>::create();
    breaks.set_breaks(Some(true));
    check!(html(marked_with_opt_marked_options("a\nb", Some(&breaks))).contains("a<br>b"));
    check!(!html(marked_with_opt_marked_options("a\nb", None)).contains("<br>"));

    // The library's defaults.
    let defaults = get_defaults();
    check!(defaults.gfm() == Some(true));
    check!(defaults.breaks() == Some(false));

    // An inline parse has no paragraph.
    // marked's `markedInstance` is not exported, so an instance is made here.
    let plain_instance = Marked::<String, String>::new(Vec::<MarkedExtension<String, String>>::new());
    let inline = plain_instance.parse_inline().invoke_2("a *b* `c`", None);
    check!(html_of(inline) == "a <em>b</em> <code>c</code>");

    // The lexer: tokens of a document, the type of each, and their children.
    let list = Lexer::<String, String>::lex::<String, String>("# Head\n\npara with **bold**\n\n- a\n- b\n", None);
    let types: Vec<String> = (0..list.length() as usize).map(|i| token(&list, i).type_()).collect();
    check!(types == ["heading", "space", "paragraph", "space", "list"]);
    check!(token(&list, 0).type_() == "heading");
    let heading = tokens::Heading(list.at(0.0).unwrap().0.clone());
    check!(heading.depth() == 1.0 && heading.text() == "Head");
    let para = (0..list.length() as usize).map(|i| token(&list, i)).find(|t| t.type_() == "paragraph").unwrap();
    let children = para.tokens().unwrap();
    check!(children.len() == 2);
    let child_types: Vec<String> = children.iter().map(|c| tokens::Generic(c.0.clone()).type_()).collect();
    check!(child_types == ["text", "strong"]);
    check!(para.raw().starts_with("para with"));
    let items = (0..list.length() as usize).map(|i| token(&list, i)).find(|t| t.type_() == "list").unwrap();
    let list_token = tokens::List(items.0.clone());
    check!(list_token.ordered() == false && list_token.items().len() == 2);
    check!(list_token.items().get(1).text() == "b");

    // A Marked instance keeps its own options, apart from the global ones.
    let ext = MarkedExtension::<String, String>::create();
    ext.set_breaks(Some(true));
    let m = Marked::<String, String>::new(vec![ext]);
    check!(html_of(m.parse().invoke_2("x\ny", None)).contains("x<br>y"));
    check!(m.defaults().breaks() == Some(true));
    check!(!html(marked_with_opt_marked_options("x\ny", None)).contains("<br>"));
    let toks = m.lexer_2("hi *there*", None);
    check!(toks.length() == 1.0 && token(&toks, 0).type_() == "paragraph");

    // A walkTokens extension sees every token, and may change them.
    let seen = Rc::new(Cell::new(0));
    let ext = MarkedExtension::<String, String>::create();
    let walk = TokenToVoidOrPromiseOrVoidOrPromiseArrayFn::new({
        let seen = seen.clone();
        move |t: JsValue| {
            let g = tokens::Generic(t.0.clone());
            seen.set(seen.get() + 1);
            if g.type_() == "heading" {
                tokens::Heading(t.0.clone()).set_depth(2.0);
            }
            None
        }
    });
    ext.set_walk_tokens(Some(&walk));
    let m2 = Marked::<String, String>::new(vec![ext]);
    let out = html_of(m2.parse().invoke_2("# Top\n\ntext", None));
    check!(out.contains("<h2>Top</h2>"));
    check!(seen.get() >= 4);

    // A renderer extension replaces how one token is written.
    let ext = MarkedExtension::<String, String>::create();
    let renderer = MarkedExtensionRenderer::<String, String>::create();
    let heading = MarkedExtensionRendererHeading::<String, String>::new(|args: JsArray| {
        let h = tokens::Heading(args.get(0).0.clone());
        BooleanOrRendererOutput::RendererOutput(format!("<h{0} class=\"x\">{1}</h{0}>", h.depth() as u32, h.text()))
    });
    renderer.set_heading(Some(&heading));
    ext.set_renderer(Some(&renderer));
    let m3 = Marked::<String, String>::new(vec![ext]);
    check!(html_of(m3.parse().invoke_2("## Sub", None)) == "<h2 class=\"x\">Sub</h2>");
}

fn html_of(r: ParserOutputOrPromise<String>) -> String {
    match r {
        ParserOutputOrPromise::ParserOutput(s) => s,
        ParserOutputOrPromise::JsPromise(_) => String::from("<promise>"),
    }
}
