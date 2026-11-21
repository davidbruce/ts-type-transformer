// KaTeX from C, in Node: TeX rendered to HTML and MathML with options, and
// the error bad input gives. Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool contains(JsString s, const char* needle) {
    uint32_t n = JsString_len(s), m = ts_strlen(needle);
    static char buf[1 << 16];
    if (n >= sizeof buf) return false;
    JsString_copy(s, buf, sizeof buf);
    for (uint32_t i = 0; i + m <= n; i++) {
        uint32_t j = 0;
        while (j < m && buf[i + j] == needle[j]) j++;
        if (j == m) return true;
    }
    return false;
}

__attribute__((export_name("run"))) void run(void) {
    // The version is a string.
    JsString version = katex_get_version_2();
    CHECK(JsString_len(version) > 0 && contains(version, "0.19"));
    js_release(version);

    // Plain rendering: inline math, HTML and MathML.
    JsString html = katex_renderToString("x^2 + y^2 = z^2");
    CHECK(contains(html, "class=\"katex\""));
    CHECK(contains(html, "katex-html"));
    CHECK(contains(html, "katex-mathml"));
    CHECK(contains(html, "<annotation encoding=\"application/x-tex\">x^2 + y^2 = z^2</annotation>"));
    CHECK(!contains(html, "katex-display"));
    js_release(html);

    // displayMode wraps in a display block.
    KatexOptions display = KatexOptions_create();
    KatexOptions_set_displayMode(display, js_some_bool(true));
    JsString shown = katex_renderToString("\\sum_{i=1}^n i", display);
    CHECK(contains(shown, "katex-display"));
    CHECK(contains(shown, "mop op-limits"));
    js_release(shown);
    CHECK(KatexOptions_get_displayMode(display).has && KatexOptions_get_displayMode(display).value);

    // The output format picks the parts.
    KatexOptions html_only = KatexOptions_create();
    KatexOptions_set_output(html_only, KatexOptions_output_html);
    JsString h = katex_renderToString("a+b", html_only);
    CHECK(contains(h, "katex-html") && !contains(h, "katex-mathml"));
    js_release(h);
    KatexOptions mathml_only = KatexOptions_create();
    KatexOptions_set_output(mathml_only, KatexOptions_output_mathml);
    JsString m = katex_renderToString("\\frac{1}{2}", mathml_only);
    CHECK(contains(m, "<mfrac>") && !contains(m, "katex-html"));
    CHECK(contains(m, "<mn>1</mn>") && contains(m, "<mn>2</mn>"));
    js_release(m);

    // Structures: fractions, roots, greek.
    JsString frac = katex_renderToString("\\frac{a}{b}");
    CHECK(contains(frac, "mfrac") && contains(frac, "frac-line"));
    js_release(frac);
    JsString root = katex_renderToString("\\sqrt{x}");
    CHECK(contains(root, "msqrt") && contains(root, "sqrt"));
    js_release(root);
    JsString greek = katex_renderToString("\\alpha + \\beta");
    CHECK(contains(greek, "\xCE\xB1") && contains(greek, "\xCE\xB2"));
    js_release(greek);

    // A user macro, from an options object that is a string-keyed record.
    KatexOptions with_macro = KatexOptions_create();
    KatexOptions_macros macros = KatexOptions_macros_create();
    JsValue replacement = js_string("\\mathbb{R}");
    KatexOptions_macros_set_at(macros, "\\RR", replacement);
    js_release(replacement);
    KatexOptions_set_macros(with_macro, macros);
    JsString real = katex_renderToString("x \\in \\RR", with_macro);
    CHECK(contains(real, "mathbb"));
    CHECK(contains(real, "x \\in \\RR"));
    js_release(real);

    // throwOnError false: bad input is shown as an error, in the error colour.
    KatexOptions lax = KatexOptions_create();
    KatexOptions_set_throwOnError(lax, js_some_bool(false));
    KatexOptions_set_errorColor(lax, "#ff0000");
    JsString shown_error = katex_renderToString("\\badcommand{x}", lax);
    CHECK(contains(shown_error, "\\badcommand"));
    CHECK(contains(shown_error, "color:#ff0000"));
    CHECK(!js_failed());
    js_release(shown_error);

    // throwOnError (the default): a ParseError is thrown.
    JsString none = katex_renderToString("\\badcommand{x}");
    CHECK(none.h == 0 && js_failed());
    JsValue error = js_take_error();
    CHECK(error.h != 0);
    JsString name = js_error_name(error);
    CHECK(JsString_equals(name, "ParseError"));
    js_release(name);
    JsString message = js_error_message(error);
    CHECK(contains(message, "KaTeX parse error"));
    CHECK(contains(message, "Undefined control sequence: \\badcommand"));
    js_release(message);
    // The error is a ParseError: its own fields can be read.
    Katex_ParseError parse_error = { error.h };
    CHECK(Katex_ParseError_get_position(parse_error) == 0);
    CHECK(Katex_ParseError_get_length(parse_error) == 11);
    JsString raw = Katex_ParseError_get_rawMessage(parse_error);
    CHECK(JsString_equals(raw, "Undefined control sequence: \\badcommand"));
    js_release(raw);
    Error as_error = Katex_ParseError_as_Error(parse_error);
    JsString text = Error_message(as_error);
    CHECK(contains(text, "KaTeX parse error"));
    js_release(text);
    js_release(error);

    // Unbalanced braces.
    katex_renderToString("\\frac{1");
    CHECK(js_failed());
    js_release(js_take_error());

    // Strict mode is a boolean, string or function: a string here.
    KatexOptions strict = KatexOptions_create();
    KatexOptions_set_strict(strict, Katex_boolean_or_string_or_StrictFunction_of_string(JsString_from("ignore")));
    JsString ok = katex_renderToString("x", strict);
    CHECK(contains(ok, "katex"));
    js_release(ok);
    Katex_boolean_or_string_or_StrictFunction got = KatexOptions_get_strict(strict);
    CHECK(got.tag == Katex_boolean_or_string_or_StrictFunction_string && JsString_equals(got.as.string, "ignore"));
}
