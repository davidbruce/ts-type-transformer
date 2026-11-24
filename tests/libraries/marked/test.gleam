// marked from Gleam, in Node: Markdown to HTML, inline Markdown, the lexer's
// tokens, and a Marked instance with options and an extension.
import bindings as marked
import bindings/lexer
import bindings/marked as marked_class
import bindings/marked_extension
import bindings/marked_extension_renderer
import bindings/marked_options_no_async
import bindings/namespaces/tokens/generic as token_generic
import bindings/namespaces/tokens/heading as token_heading
import bindings/namespaces/tokens/paragraph as token_paragraph
import bindings/runtime
import bindings/tokens_list
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

fn token_type(tok: t.Token) -> String {
  case tok {
    t.TokenTokensBlockquote(_) -> "blockquote"
    t.TokenTokensBr(_) -> "br"
    t.TokenTokensCheckbox(_) -> "checkbox"
    t.TokenTokensCode(_) -> "code"
    t.TokenTokensCodespan(_) -> "codespan"
    t.TokenTokensDef(_) -> "def"
    t.TokenTokensDel(_) -> "del"
    t.TokenTokensEm(_) -> "em"
    t.TokenTokensEscape(_) -> "escape"
    t.TokenTokensHeading(_) -> "heading"
    t.TokenTokensHr(_) -> "hr"
    t.TokenTokensHTML(_) -> "html"
    t.TokenTokensImage(_) -> "image"
    t.TokenTokensLink(_) -> "link"
    t.TokenTokensList2(_) -> "list"
    t.TokenTokensListItem(_) -> "list_item"
    t.TokenTokensParagraph(_) -> "paragraph"
    t.TokenTokensSpace(_) -> "space"
    t.TokenTokensStrong(_) -> "strong"
    t.TokenTokensTable(_) -> "table"
    t.TokenTokensTag(_) -> "html"
    t.TokenTokensText(_) -> "text"
    t.TokenTokensGeneric(g) -> token_generic.get_type(g)
  }
}

pub fn main() {
  let has = string.contains
  let plain = marked_options_no_async.create()

  // Block Markdown to HTML, through the module's function.
  let html = marked.marked("# Title\n\nHello **world** and *you*.", plain)
  let passed =
    check(
      html
        == "<h1>Title</h1>\n<p>Hello <strong>world</strong> and <em>you</em>.</p>\n",
      "marked",
    )
    + check(
      has(marked.marked("- a\n- b\n- c", plain), "<ul>\n<li>a</li>"),
      "a list",
    )
    + check(
      has(
        marked.marked("```js\nlet x = 1;\n```", plain),
        "<pre><code class=\"language-js\">let x = 1;\n</code></pre>",
      ),
      "a code block",
    )
    + check(
      has(
        marked.marked("[a link](http://example.com \"t\")", plain),
        "<a href=\"http://example.com\" title=\"t\">a link</a>",
      ),
      "a link",
    )
    + check(has(marked.marked("> quoted", plain), "<blockquote>"), "a quote")
    + check(
      has(marked.marked("| a | b |\n|---|---|\n| 1 | 2 |", plain), "<th>a</th>"),
      "a table (gfm)",
    )
    + check(has(marked.marked("a <b> & c", plain), "&amp;"), "escapes")

  // Options: made here, as a record of optional fields.
  let breaks = marked_options_no_async.create()
  marked_options_no_async.set_breaks(breaks, Some(True))
  let passed =
    passed
    + check(has(marked.marked("a\nb", breaks), "a<br>b"), "breaks: true")
    + check(!has(marked.marked("a\nb", plain), "<br>"), "no breaks by default")
    + check(
      marked_options_no_async.get_breaks(breaks) == Some(True),
      "option read back",
    )
  let no_gfm = marked_options_no_async.create()
  marked_options_no_async.set_gfm(no_gfm, Some(False))
  let passed =
    passed
    + check(
      has(marked.marked("~~gone~~", plain), "<del>gone</del>"),
      "gfm strikethrough",
    )
    + check(!has(marked.marked("~~gone~~", no_gfm), "<del>"), "gfm: false")

  // The optional-options form gives a string or a promise.
  let passed =
    passed
    + check(
      marked.marked_with_opt_marked_options("*x*")
        == t.StringOrPromiseString("<p><em>x</em></p>\n"),
      "marked with no options",
    )

  // An instance: parse and parseInline.
  // The module's parse is a Dynamic and its parseInline is not bound at all
  // (FINDING 2): an instance's parse and parseInline are used instead.
  let instance = marked_class.new([])
  let parse = marked_class.get_parse(instance)
  let parse_inline = marked_class.get_parse_inline(instance)
  let call = marked_options_no_async.create()
  let passed =
    passed
    + check(
      marked_class.parse_invoke(parse, "## Two", call) == "<h2>Two</h2>\n",
      "parse",
    )
    + check(
      marked_class.parse_invoke(parse_inline, "**bold** `code`", call)
        == "<strong>bold</strong> <code>code</code>",
      "parseInline",
    )
    + check(
      marked_class.parse_invoke(parse_inline, "# not a heading", call)
        == "# not a heading",
      "parseInline has no blocks",
    )

  // The lexer: tokens, their types and fields.
  let tokens = lexer.lex_2("# Head\n\nSome *text*.\n\n- one\n- two\n")
  let list_of = tokens_list.to_list(tokens)
  let types = list.map(list_of, token_type)
  let passed =
    passed
    + check(tokens_list.get_length(tokens) == 5.0, "token count")
    + check(
      types == ["heading", "space", "paragraph", "space", "list"],
      "token types",
    )
  let assert [t.TokenTokensHeading(heading), ..] = list_of
  let passed =
    passed
    + check(token_heading.get_depth(heading) == 1.0, "heading depth")
    + check(token_heading.get_text(heading) == "Head", "heading text")
    + check(token_heading.get_raw(heading) == "# Head", "heading raw")
  let assert Ok(t.TokenTokensParagraph(paragraph)) =
    list.drop(list_of, 2) |> list.first
  let inner = token_paragraph.get_tokens(paragraph)
  let passed =
    passed
    + check(
      list.map(inner, token_type) == ["text", "em", "text"],
      "inline tokens",
    )
  let assert Some(first) = tokens_list.get_at(tokens, 0.0)
  let passed =
    passed
    + check(token_type(first) == "heading", "get_at")
    + check(list.length(lexer.lex_inline("a *b* c")) == 3, "lex_inline")
    + check(
      tokens_list.get_length(marked.get_lexer()("a\n\nb", None)) == 3.0,
      "the lexer function",
    )

  // A Marked instance with options.
  let soft = marked_extension.create()
  marked_extension.set_breaks(soft, Some(True))
  let inst = marked_class.new([soft])
  let passed =
    passed
    + check(
      has(
        marked_class.parse_invoke(marked_class.get_parse(inst), "a\nb", call),
        "a<br>b",
      ),
      "an instance with breaks",
    )
    + check(
      !has(marked_class.parse_invoke(parse, "a\nb", call), "<br>"),
      "a plain instance is unchanged",
    )

  // A renderer function: called with the heading's token.
  let rendered = cell.new([])
  let renderer = marked_extension_renderer.create()
  marked_extension_renderer.set_heading(
    renderer,
    Some(fn(heading) {
      let token = runtime.dynamic(heading)
      cell.set(rendered, [
        #(field_string(token, "text"), field_float(token, "depth")),
        ..cell.get(rendered)
      ])
      t.StringOrBooleanString(
        "<h"
        <> int.to_string(float.round(field_float(token, "depth")))
        <> " class=\"x\">"
        <> field_string(token, "text")
        <> "</h>\n",
      )
    }),
  )
  let ext = marked_extension.create()
  marked_extension.set_renderer(ext, Some(renderer))
  let with_renderer = marked_class.new([ext])
  let heading_html =
    marked_class.parse_invoke(
      marked_class.get_parse(with_renderer),
      "# Hi\n\n## There",
      call,
    )
  let passed =
    passed
    + check(
      heading_html == "<h1 class=\"x\">Hi</h>\n<h2 class=\"x\">There</h>\n",
      "a renderer's output is used",
    )
    + check(
      list.reverse(cell.get(rendered)) == [#("Hi", 1.0), #("There", 2.0)],
      "a renderer gets its token",
    )

  // A preprocess hook: the markdown arrives as the one argument.
  let hooks = marked_extension.hooks_create()
  marked_extension.hooks_set_preprocess(
    hooks,
    Some(fn(text) {
      t.StringOrPromiseString(string.replace(text, "foo", "bar"))
    }),
  )
  let pre = marked_extension.create()
  marked_extension.set_hooks(pre, Some(hooks))
  let preprocessed = marked_class.new([pre])
  let passed =
    passed
    + check(
      marked_class.parse_invoke(
        marked_class.get_parse(preprocessed),
        "foo *foo*",
        call,
      )
        == "<p>bar <em>bar</em></p>\n",
      "a preprocess hook",
    )

  // walkTokens: called for each token, the nested ones too.
  let seen = cell.new([])
  let walker = marked_extension.create()
  marked_extension.set_walk_tokens(
    walker,
    Some(fn(token) {
      cell.set(seen, [token_type(token), ..cell.get(seen)])
      None
    }),
  )
  let walked = marked_class.new([walker])
  let _ =
    marked_class.parse_invoke(
      marked_class.get_parse(walked),
      "# A\n\n*b*",
      call,
    )
  let kinds = list.reverse(cell.get(seen))
  let passed =
    passed
    + check(
      list.contains(kinds, "heading")
        && list.contains(kinds, "paragraph")
        && list.contains(kinds, "em"),
      "walkTokens reaches nested tokens",
    )

  io.println(int.to_string(passed) <> " checks passed")
}
