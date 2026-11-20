// marked from Gleam, in Node: Markdown to HTML, inline Markdown, the lexer's
// tokens, and a Marked instance with options and an extension.
import bindings/areas/main as marked
import bindings/areas/main/lexer
import bindings/areas/main/marked as marked_class
import bindings/areas/main/marked_extension
import bindings/areas/main/marked_extension_hooks
import bindings/areas/main/marked_extension_renderer
import bindings/areas/main/marked_options_no_async
import bindings/areas/main/marked_parse
import bindings/areas/main/marked_parse_call_options_no_async
import bindings/areas/main/tokens_list
import bindings/runtime
import bindings/types as t
import cell
import gleam/dynamic.{type Dynamic}
import gleam/dynamic/decode
import gleam/float
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}
import gleam/string

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

fn field_string(token: Dynamic, name: String) -> String {
  let assert Ok(s) = decode.run(token, decode.at([name], decode.string))
  s
}

fn field_float(token: Dynamic, name: String) -> Float {
  let assert Ok(s) = decode.run(token, decode.at([name], decode.float))
  s
}

pub fn main() {
  let has = string.contains
  let plain = marked_options_no_async.create()

  // Block Markdown to HTML, through the module's function.
  let html = marked.marked("# Title\n\nHello **world** and *you*.", plain)
  let passed =
    check(html == "<h1>Title</h1>\n<p>Hello <strong>world</strong> and <em>you</em>.</p>\n", "marked")
    + check(has(marked.marked("- a\n- b\n- c", plain), "<ul>\n<li>a</li>"), "a list")
    + check(has(marked.marked("```js\nlet x = 1;\n```", plain), "<pre><code class=\"language-js\">let x = 1;\n</code></pre>"), "a code block")
    + check(has(marked.marked("[a link](http://example.com \"t\")", plain), "<a href=\"http://example.com\" title=\"t\">a link</a>"), "a link")
    + check(has(marked.marked("> quoted", plain), "<blockquote>"), "a quote")
    + check(has(marked.marked("| a | b |\n|---|---|\n| 1 | 2 |", plain), "<th>a</th>"), "a table (gfm)")
    + check(has(marked.marked("a <b> & c", plain), "&amp;"), "escapes")

  // Options: made here, as a record of optional fields.
  let breaks = marked_options_no_async.create()
  marked_options_no_async.set_breaks(breaks, Some(True))
  let passed =
    passed
    + check(has(marked.marked("a\nb", breaks), "a<br>b"), "breaks: true")
    + check(!has(marked.marked("a\nb", plain), "<br>"), "no breaks by default")
    + check(marked_options_no_async.get_breaks(breaks) == Some(True), "option read back")
  let no_gfm = marked_options_no_async.create()
  marked_options_no_async.set_gfm(no_gfm, Some(False))
  let passed =
    passed
    + check(has(marked.marked("~~gone~~", plain), "<del>gone</del>"), "gfm strikethrough")
    + check(!has(marked.marked("~~gone~~", no_gfm), "<del>"), "gfm: false")

  // The optional-options form gives a string or a promise.
  let passed =
    passed
    + check(marked.marked_with_opt_marked_options("*x*", None) == t.StringOrPromiseString("<p><em>x</em></p>\n"), "marked with no options")

  // An instance: parse and parseInline.
  // The module's parse is a Dynamic and its parseInline is not bound at all
  // (FINDING 2): an instance's parse and parseInline are used instead.
  let instance = marked_class.new([])
  let parse = marked_class.get_parse(instance)
  let parse_inline = marked_class.get_parse_inline(instance)
  let call = marked_parse_call_options_no_async.create()
  let passed =
    passed
    + check(marked_parse.invoke(parse, "## Two", call, runtime.codec_string()) == "<h2>Two</h2>\n", "parse")
    + check(marked_parse.invoke(parse_inline, "**bold** `code`", call, runtime.codec_string()) == "<strong>bold</strong> <code>code</code>", "parseInline")
    + check(marked_parse.invoke(parse_inline, "# not a heading", call, runtime.codec_string()) == "# not a heading", "parseInline has no blocks")

  // The lexer: tokens, their types and fields.
  let tokens = lexer.lex("# Head\n\nSome *text*.\n\n- one\n- two\n", None)
  let list_of = tokens_list.to_list(tokens)
  let types = list.map(list_of, field_string(_, "type"))
  let passed =
    passed
    + check(tokens_list.get_length(tokens) == 5.0, "token count")
    + check(types == ["heading", "space", "paragraph", "space", "list"], "token types")
  let assert [heading, ..] = list_of
  let passed =
    passed
    + check(field_float(heading, "depth") == 1.0, "heading depth")
    + check(field_string(heading, "text") == "Head", "heading text")
    + check(field_string(heading, "raw") == "# Head", "heading raw")
  let assert Ok(paragraph) = list.drop(list_of, 2) |> list.first
  let assert Ok(inner) = decode.run(paragraph, decode.at(["tokens"], decode.list(decode.dynamic)))
  let passed =
    passed
    + check(list.map(inner, field_string(_, "type")) == ["text", "em", "text"], "inline tokens")
  let assert Some(first) = tokens_list.get_at(tokens, 0.0)
  let passed =
    passed
    + check(field_string(first, "type") == "heading", "get_at")
    + check(list.length(lexer.lex_inline("a *b* c", None)) == 3, "lex_inline")
    + check(tokens_list.get_length(marked.get_lexer()("a\n\nb", None)) == 3.0, "the lexer function")

  // A Marked instance with options.
  let soft = marked_extension.create()
  marked_extension.set_breaks(soft, Some(True))
  let inst = marked_class.new([soft])
  let passed =
    passed
    + check(has(marked_parse.invoke(marked_class.get_parse(inst), "a\nb", call, runtime.codec_string()), "a<br>b"), "an instance with breaks")
    + check(!has(marked_parse.invoke(parse, "a\nb", call, runtime.codec_string()), "<br>"), "a plain instance is unchanged")

  // A renderer function: called with the heading's token.
  let rendered = cell.new([])
  let renderer = marked_extension_renderer.create()
  marked_extension_renderer.set_heading(
    renderer,
    Some(fn(args) {
      let #(heading) = args
      let token = runtime.dynamic(heading)
      cell.set(rendered, [#(field_string(token, "text"), field_float(token, "depth")), ..cell.get(rendered)])
      t.BooleanOrRendererOutputRendererOutput("<h" <> int.to_string(float.round(field_float(token, "depth"))) <> " class=\"x\">" <> field_string(token, "text") <> "</h>\n")
    }),
    runtime.codec_string(),
  )
  let ext = marked_extension.create()
  marked_extension.set_renderer(ext, Some(renderer))
  let with_renderer = marked_class.new([ext])
  let heading_html = marked_parse.invoke(marked_class.get_parse(with_renderer), "# Hi\n\n## There", call, runtime.codec_string())
  let passed =
    passed
    + check(heading_html == "<h1 class=\"x\">Hi</h>\n<h2 class=\"x\">There</h>\n", "a renderer's output is used")
    + check(list.reverse(cell.get(rendered)) == [#("Hi", 1.0), #("There", 2.0)], "a renderer gets its token")

  // A preprocess hook: the markdown arrives as the one argument.
  let hooks = marked_extension_hooks.create()
  marked_extension_hooks.set_preprocess(
    hooks,
    Some(fn(args) {
      let assert [first] = args
      let assert Ok(text) = decode.run(first, decode.string)
      t.StringOrPromiseString(string.replace(text, "foo", "bar"))
    }),
  )
  let pre = marked_extension.create()
  marked_extension.set_hooks(pre, Some(hooks))
  let preprocessed = marked_class.new([pre])
  let passed =
    passed
    + check(
      marked_parse.invoke(marked_class.get_parse(preprocessed), "foo *foo*", call, runtime.codec_string()) == "<p>bar <em>bar</em></p>\n",
      "a preprocess hook",
    )

  // walkTokens: called for each token, the nested ones too.
  let seen = cell.new([])
  let walker = marked_extension.create()
  marked_extension.set_walk_tokens(
    walker,
    Some(fn(token) {
      cell.set(seen, [field_string(token, "type"), ..cell.get(seen)])
      None
    }),
  )
  let walked = marked_class.new([walker])
  let _ = marked_parse.invoke(marked_class.get_parse(walked), "# A\n\n*b*", call, runtime.codec_string())
  let kinds = list.reverse(cell.get(seen))
  let passed =
    passed
    + check(list.contains(kinds, "heading") && list.contains(kinds, "paragraph") && list.contains(kinds, "em"), "walkTokens reaches nested tokens")

  io.println(int.to_string(passed) <> " checks passed")
}
