// KaTeX from Gleam, in Node: TeX rendered to HTML and MathML, with options,
// and a ParseError for bad input. Every check must hold.
import bindings as katex
import bindings/katex_options
import bindings/katex_options_macros
import bindings/parse_error
import bindings/runtime
import bindings/types as t
import gleam/int
import gleam/io
import gleam/option.{Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() {
  let has = fn(s: String, part: String) { string.contains(s, part) }

  // Plain rendering: HTML and MathML together by default.
  let html = katex.render_to_string("x^2 + y^2 = z^2")
  let passed =
    check(has(html, "class=\"katex\""), "katex wrapper")
    + check(has(html, "katex-html"), "html part")
    + check(has(html, "katex-mathml"), "mathml part")
    + check(
      has(
        html,
        "<annotation encoding=\"application/x-tex\">x^2 + y^2 = z^2</annotation>",
      ),
      "the source is kept",
    )
    + check(!has(html, "katex-display"), "inline by default")
    + check(katex.get_version() == "0.19.1", "version")

  // Options: display mode, output kind.
  let opts = katex_options.create()
  katex_options.set_display_mode(opts, Some(True))
  let display = katex.render_to_string_with("\\frac{a}{b}", opts)
  let passed =
    passed
    + check(has(display, "katex-display"), "display mode")
    + check(has(display, "mfrac"), "a fraction in MathML")
    + check(
      katex_options.get_display_mode(opts) == Some(True),
      "option read back",
    )
  let ml = katex_options.create()
  katex_options.set_output(ml, Some(t.KatexOptionsOutputMathml))
  let mathml = katex.render_to_string_with("a+b", ml)
  let passed =
    passed
    + check(has(mathml, "<math"), "mathml output")
    + check(!has(mathml, "katex-html"), "no html in mathml output")
  let only_html = katex_options.create()
  katex_options.set_output(only_html, Some(t.KatexOptionsOutputHtml))
  let h = katex.render_to_string_with("a+b", only_html)
  let passed =
    passed + check(has(h, "katex-html") && !has(h, "<math"), "html output")

  // Left-hand equation numbers; colours.
  let leq = katex_options.create()
  katex_options.set_display_mode(leq, Some(True))
  katex_options.set_leqno(leq, Some(True))
  let passed =
    passed
    + check(has(katex.render_to_string_with("x", leq), "leqno"), "leqno")
    + check(has(katex.render_to_string("\\color{red} x"), "red"), "color")
    + check(has(katex.render_to_string("\\sqrt{2}"), "sqrt"), "sqrt")

  // Macros: an options object whose keys are made here.
  let macros = katex_macros()
  let with_macro = katex_options.create()
  katex_options.set_macros(with_macro, Some(macros))
  let passed =
    passed
    + check(
      has(
        katex.render_to_string_with("\\RR", with_macro),
        "double-struck",
      ),
      "macro expands",
    )
    + check(
      katex_options_macros.get_at(macros, "\\RR")
        == runtime.dynamic("\\mathbb{R}"),
      "macro read back",
    )

  // Bad input throws a ParseError.
  let failed = runtime.attempt(fn() { katex.render_to_string("\\badmacro") })
  let passed = case failed {
    Ok(_) -> panic as "bad TeX should throw"
    Error(e) -> {
      let message = runtime.js_error_message(e)
      let passed =
        passed
        + check(has(message, "KaTeX parse error"), "message")
        + check(has(message, "\\badmacro"), "message names the macro")
      case parse_error.cast(runtime.dynamic(e)) {
        Ok(pe) ->
          passed
          + check(parse_error.get_position(pe) == 0.0, "position")
          + check(parse_error.get_length(pe) == 9.0, "length")
          + check(
            has(parse_error.get_raw_message(pe), "Undefined control sequence"),
            "raw message",
          )
        Error(_) -> panic as "the exception is a ParseError"
      }
    }
  }

  // throwOnError false: the source is shown as an error, in red.
  let soft = katex_options.create()
  katex_options.set_throw_on_error(soft, Some(False))
  let shown = katex.render_to_string_with("\\badmacro", soft)
  let passed =
    passed
    + check(has(shown, "#cc0000"), "error output")
    + check(has(shown, "\\badmacro"), "error shows the source")
  let colour = katex_options.create()
  katex_options.set_throw_on_error(colour, Some(False))
  katex_options.set_error_color(colour, Some("#00ff00"))
  let passed =
    passed
    + check(
      has(katex.render_to_string_with("\\badmacro", colour), "#00ff00"),
      "error colour",
    )

  io.println(int.to_string(passed) <> " checks passed")
}

fn katex_macros() {
  let m = katex_options_macros.create()
  katex_options_macros.set_at(m, "\\RR", runtime.dynamic("\\mathbb{R}"))
  m
}
