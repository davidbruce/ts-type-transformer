// KaTeX from C++, in Node: TeX rendered to HTML and MathML, options, errors.
// Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool has(const std::string& s, std::string_view part) { return s.find(part) != std::string::npos; }

extern "C" __attribute__((export_name("run"))) void run() {
    // Plain rendering: a span of class katex, with the MathML beside the HTML.
    std::string simple = renderToString("x^2 + y^2 = z^2");
    CHECK(has(simple, "class=\"katex\""));
    CHECK(has(simple, "katex-mathml"));
    CHECK(has(simple, "katex-html"));
    CHECK(has(simple, "<annotation encoding=\"application/x-tex\">x^2 + y^2 = z^2</annotation>"));
    CHECK(has(simple, "<msup>"));
    CHECK(!has(simple, "katex-display"));

    // The version, as a function and as a property of the katex object.
    CHECK(!get_version().empty());
    CHECK(katex::version() == get_version());

    // Display mode wraps in katex-display.
    KatexOptions display = KatexOptions::create();
    display.displayMode(true);
    CHECK(display.displayMode() == true);
    CHECK(!display.leqno());
    std::string shown = renderToString("\\sum_{i=1}^n i", display);
    CHECK(has(shown, "katex-display"));
    CHECK(has(shown, "<mo>∑</mo>") || has(shown, "∑"));
    CHECK(shown.size() > simple.size() / 2);

    // Output format: mathml alone has no HTML part, html alone no MathML.
    KatexOptions ml = KatexOptions::create();
    ml.output(KatexOptions_output::mathml);
    CHECK(ml.output() == KatexOptions_output::mathml);
    std::string mathml = renderToString("a+b", ml);
    CHECK(has(mathml, "<math"));
    CHECK(!has(mathml, "katex-html"));
    KatexOptions html = KatexOptions::create();
    html.output(KatexOptions_output::html);
    std::string htmlOnly = renderToString("a+b", html);
    CHECK(has(htmlOnly, "katex-html"));
    CHECK(!has(htmlOnly, "<math"));

    // Equation numbers on the left.
    KatexOptions leq = KatexOptions::create();
    leq.displayMode(true);
    leq.leqno(true);
    CHECK(has(renderToString("E=mc^2", leq), "leqno"));

    // A bad control sequence throws a ParseError: the call returns nothing.
    CHECK(!ts::take_error());
    std::string bad = renderToString("\\notacommand{x}");
    CHECK(bad.empty());
    std::optional<ts::Error> e = ts::take_error();
    CHECK(e.has_value());
    CHECK(has(e->message(), "KaTeX parse error"));
    CHECK(has(e->message(), "Undefined control sequence"));
    CHECK(e->name() == "ParseError" || e->name() == "Error");
    CHECK(!ts::take_error());

    // The same through attempt, and as the generated ParseError type.
    ts::Result<std::string> tried = ts::attempt([] { return renderToString("\\frac{1}{"); });
    CHECK(!tried);
    CHECK(has(tried.error().message(), "KaTeX parse error"));
    // FINDING 1: a caught ts::Error has no conversion to the generated ParseError
    // (which extends the generated ::Error, not ts::Error); copy the handle over.
    ParseError pe;
    static_cast<ts::Handle&>(pe) = tried.error();
    CHECK(pe.length() >= 0);
    CHECK(has(pe.rawMessage(), "expected '}'"));
    CHECK(has(pe.message(), pe.rawMessage()));
    // A ParseError where an Error is wanted.
    ::Error asError = pe;
    CHECK(asError.message() == pe.message());

    // With throwOnError off the source is shown in the error colour instead.
    KatexOptions lenient = KatexOptions::create();
    lenient.throwOnError(false);
    lenient.errorColor("#ff0000");
    CHECK(lenient.throwOnError() == false);
    CHECK(lenient.errorColor() == "#ff0000");
    std::string shownError = renderToString("\\notacommand{x}", lenient);
    CHECK(!ts::take_error());
    CHECK(has(shownError, "mathcolor=\"#ff0000\""));
    CHECK(has(shownError, "#ff0000"));
    CHECK(has(shownError, "<annotation encoding=\"application/x-tex\">\\notacommand{x}</annotation>"));

    // Numbers: minimum rule thickness and size limits.
    KatexOptions thin = KatexOptions::create();
    thin.minRuleThickness(0.2);
    thin.maxSize(10);
    CHECK(thin.minRuleThickness() == 0.2 && thin.maxSize() == 10.0);
    CHECK(has(renderToString("\\frac{a}{b}", thin), "frac-line"));

    // strict is a union of a flag, a string or a function.
    KatexOptions strictOff = KatexOptions::create();
    strictOff.strict(false);
    CHECK(has(renderToString("é", strictOff), "class=\"katex\""));
    KatexOptions strictWord = KatexOptions::create();
    strictWord.strict(std::string("ignore"));
    CHECK(!renderToString("é", strictWord).empty());
    CHECK(!ts::take_error());

    // strict as a function: told the error code, message and token.
    int asked = 0;
    KatexOptions strictFn = KatexOptions::create();
    strictFn.strict(StrictFunction([&](StrictFunction_errorCode code, std::string, std::optional<Token>) {
        asked++;
        return StrictFunctionResult(code == StrictFunction_errorCode::unicodeTextInMathMode ? std::string("ignore") : std::string("error"));
    }));
    renderToString("é", strictFn);
    CHECK(asked >= 1);
    (void)ts::take_error();

    // Macros: an object of name to expansion.
    KatexOptions withMacros = KatexOptions::create();
    ts::Record<ts::Value> macros{{"\\RR", ts::Value::string("\\mathbb{R}")}};
    withMacros.macros(macros);
    std::string real = renderToString("x \\in \\RR", withMacros);
    CHECK(!ts::take_error());
    CHECK(has(real, "mathbb"));
    CHECK(has(real, "ℝ") || has(real, "R"));
    CHECK(macros.at("\\RR").to_string() == "\\mathbb{R}");

    // trust: a flag, or a function of the context.
    KatexOptions trusting = KatexOptions::create();
    trusting.trust(true);
    std::string link = renderToString("\\href{https://example.com}{link}", trusting);
    CHECK(has(link, "href=\"https://example.com\""));
    KatexOptions untrusting = KatexOptions::create();
    untrusting.trust(false);
    std::string nolink = renderToString("\\href{https://example.com}{link}", untrusting);
    CHECK(!has(nolink, "href=\"https://example.com\""));
    (void)ts::take_error();

    // A ParseError made here.
    ParseError made = ParseError::create("custom failure");
    CHECK(has(made.message(), "custom failure"));
    CHECK(made.name == "ParseError");
}
