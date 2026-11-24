// KaTeX from Rust, in Node: TeX rendered to HTML and MathML, with options,
// and the error a bad formula throws. Every check! must hold.
#![no_main]
use bindings::*;

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
    // A formula inline: the default output holds both HTML and MathML.
    let inline = render_to_string("x^2 + y^2 = z^2");
    check!(inline.contains("class=\"katex\""));
    check!(inline.contains("katex-html"));
    check!(inline.contains("<math"));
    check!(inline.contains("annotation encoding=\"application/x-tex\">x^2 + y^2 = z^2<"));
    check!(!inline.contains("katex-display"));

    // Display mode wraps it in a block.
    let opts = KatexOptions::create();
    opts.set_display_mode(Some(true));
    let display = render_to_string_with("\\int_0^1 x\\,dx", Some(&opts));
    check!(display.contains("katex-display"));
    check!(display.contains("\u{222b}"));
    check!(opts.display_mode() == Some(true));

    // The output choice: only MathML, or only HTML.
    let mathml = KatexOptions::create();
    mathml.set_output(Some(KatexOptionsOutput::Mathml));
    let m = render_to_string_with("\\frac{a}{b}", Some(&mathml));
    check!(m.contains("<mfrac>") && !m.contains("katex-html"));
    check!(mathml.output() == Some(KatexOptionsOutput::Mathml));
    let htmlonly = KatexOptions::create();
    htmlonly.set_output(Some(KatexOptionsOutput::Html));
    let h = render_to_string_with("\\frac{a}{b}", Some(&htmlonly));
    check!(!h.contains("<mfrac") && h.contains("katex-html") && h.contains("mord"));

    // leqno numbers an equation on the left.
    let leqno = KatexOptions::create();
    leqno.set_display_mode(Some(true));
    leqno.set_leqno(Some(true));
    check!(render_to_string_with("a", Some(&leqno)).contains("leqno"));

    // Fractions, roots and Greek letters.
    let root = render_to_string("\\sqrt{\\alpha}");
    check!(root.contains("\u{3b1}") && root.contains("sqrt"));

    // A bad formula throws a ParseError; the call returns "" and the
    // exception is kept.
    let bad = js::attempt(|| render_to_string("\\frac{1}{"));
    match bad {
        Err(e) => {
            check!(e.message().contains("KaTeX parse error"));
            check!(e.name() == "ParseError");
        }
        Ok(_) => check!(false),
    }
    let unknown = js::attempt(|| render_to_string("\\notacommand"));
    check!(unknown.is_err_and(|e| {
        e.message()
            .contains("Undefined control sequence: \\notacommand")
    }));

    // throwOnError false renders the error instead, in the error colour.
    let lenient = KatexOptions::create();
    lenient.set_throw_on_error(Some(false));
    lenient.set_error_color(Some("#00ff00"));
    let shown = js::attempt(|| render_to_string_with("\\notacommand", Some(&lenient)));
    check!(
        shown
            .as_ref()
            .is_ok_and(|s| s.contains("katex") && !s.contains("katex-error"))
    );
    check!(shown.as_ref().is_ok_and(|s| s.contains("#00ff00")));
    check!(shown.is_ok_and(|s| s.contains("\\notacommand")));
    check!(lenient.throw_on_error() == Some(false));
    check!(lenient.error_color().as_deref() == Some("#00ff00"));

    // User macros.
    let macros = KatexOptionsMacros::create();
    macros.set_at("\\RR", JsValue::from("\\mathbb{R}"));
    let custom = KatexOptions::create();
    custom.set_macros(Some(&macros));
    let r = render_to_string_with("x \\in \\RR", Some(&custom));
    check!(r.contains("mathbb"));
    check!(macros.at("\\RR").as_string() == "\\mathbb{R}");

    // Strict mode: a Unicode letter in math mode is only a warning unless strict.
    let strict = KatexOptions::create();
    strict.set_strict(BooleanOrStringOrStrictFunction::String("error".to_string()));
    let bad_strict = js::attempt(|| render_to_string_with("\u{e9}", Some(&strict)));
    check!(bad_strict.is_err_and(|e| e.message().contains("strict mode")));
    check!(render_to_string("\u{e9}").contains("\u{e9}"));

    // The library's version.
    let v = version();
    check!(v.split('.').count() == 3);
    check!(Katex::version() == v);
}
