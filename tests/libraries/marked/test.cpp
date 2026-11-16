// marked from C++, in Node: Markdown to HTML, the lexer's tokens, a Marked
// instance with options, and an extension with a renderer and walkTokens.
// Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool has(const std::string& s, std::string_view part) { return s.find(part) != std::string::npos; }

// A string out of `string | Promise<string>`: marked is synchronous unless asked.
static std::string text_of(const string_or_Promise& r) {
    const std::string* s = std::get_if<std::string>(&r);
    return s ? *s : std::string("<promise>");
}

// A token rewrapped as the type wanted: the same JavaScript object.
template <class T>
static T as(const ts::Value& v) {
    T t;
    static_cast<ts::Handle&>(t) = v;
    return t;
}

extern "C" __attribute__((export_name("run"))) void run() {
    // The whole document to HTML.
    std::string html = text_of(marked("# Hello *world*\n\nSome `code` and [link](http://x.com \"t\")\n\n- a\n- b\n"));
    CHECK(has(html, "<h1>Hello <em>world</em></h1>"));
    CHECK(has(html, "<code>code</code>"));
    CHECK(has(html, "<a href=\"http://x.com\" title=\"t\">link</a>"));
    CHECK(has(html, "<ul>\n<li>a</li>\n<li>b</li>\n</ul>"));

    // An options object made here: breaks turns a newline into <br>.
    MarkedOptions<std::string, std::string> breaks = MarkedOptions<std::string, std::string>::create();
    breaks.breaks(true);
    CHECK(breaks.breaks() == true);
    CHECK(!breaks.gfm());
    CHECK(text_of(marked("a\nb", breaks)) == "<p>a<br>b</p>\n");
    CHECK(text_of(marked("a\nb")) == "<p>a\nb</p>\n");

    // With async: false the result is a string, with no union to look into.
    marked_options_no_async plain = marked_options_no_async::create();
    plain.gfm(false);
    CHECK(marked("~~x~~", plain) == "<p>~~x~~</p>\n");
    CHECK(marked("~~x~~", marked_options_no_async::create()) == "<p><del>x</del></p>\n");

    // FINDING 2: `marked.lexer`, `marked.parser`, `marked.options`, `walkTokens`...
    // are function-typed consts bound as bare Callbacks: they can be made from a
    // lambda but there is no way to call one. The Lexer class and a Marked
    // instance are used instead. (Likewise `parse` is a ts::Value, and the
    // `parseInline` const has a struct but no accessor to get it.)
    using Mk = Marked<std::string, std::string>;
    // FINDING 4: get_markedInstance() reads `markedInstance`, a top-level
    // declaration that marked.d.ts does not export: null at runtime.
    Mk md = Mk::create(ts::Array<MarkedExtension<std::string, std::string>>::create());
    TokensList list = md.lexer("# H *x*\n\npara **b**\n\n- x\n- y\n\n```js\nlet a\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
    // FINDING 3: iterating or indexing a TokensList gives untyped ts::Value, and
    // the `Token` union has no decoder (types/Token.hpp declares ts_raw_Token_tag
    // but defines no decode_Token), so ts::Array<Token> does not compile and a
    // token cannot be told apart by type. Workaround: rewrap the Value as the
    // type wanted, and read `type` through Tokens_Generic.
    CHECK(list.length() == 9);
    std::string types;
    for (ts::Value v : list) types += as<Tokens_Generic>(v).type() + " ";
    CHECK(types == "heading space paragraph space list space code space table ");

    Tokens_Heading heading = as<Tokens_Heading>(*list.at(0));
    CHECK(heading.depth() == 1);
    CHECK(heading.text() == "H *x*");
    CHECK(heading.raw() == "# H *x*");
    CHECK(Tokens_Heading::type == "heading");
    CHECK(heading.tokens().size() == 2);

    // The nested inline tokens.
    ts::Array<> inner = heading.tokens();
    CHECK(as<Tokens_Generic>(inner[0]).type() == "text");
    ts::Value emv = inner[1];
    CHECK(as<Tokens_Generic>(emv).type() == "em");
    CHECK(as<Tokens_Em>(emv).text() == "x");

    Tokens_Paragraph para = as<Tokens_Paragraph>(*list.at(2));
    CHECK(para.text() == "para **b**");
    Tokens_List ul = as<Tokens_List>(*list.at(4));
    CHECK(!ul.ordered());
    CHECK(ul.items().size() == 2);
    CHECK(ul.items()[1].text() == "y");
    Tokens_Code code = as<Tokens_Code>(*list.at(6));
    CHECK(code.lang() == "js");
    CHECK(code.text() == "let a");
    Tokens_Table table = as<Tokens_Table>(*list.at(8));
    CHECK(table.header().size() == 2);
    CHECK(table.header()[0].text() == "a");
    CHECK(table.rows().size() == 1);
    CHECK(table.rows()[0][1].text() == "2");

    // The static lexer, and inline tokens.
    TokensList quick = ::Lexer<std::string, std::string>::lex_static("*a* b");
    CHECK(quick.length() == 1);
    ts::Array<> inlineTokens = ::Lexer<std::string, std::string>::lexInline("`c` and **d**");
    CHECK(inlineTokens.size() == 3);

    // A Marked instance made with an extension holding options.
    MarkedExtension<std::string, std::string> opts = MarkedExtension<std::string, std::string>::create();
    opts.breaks(true);
    Mk instance = Mk::create({opts});
    CHECK(std::get<std::string>(instance.parse().invoke("a\nb")) == "<p>a<br>b</p>\n");
    CHECK(std::get<std::string>(instance.parseInline().invoke("**bold** and _it_")) == "<strong>bold</strong> and <em>it</em>");
    // The shared instance is untouched.
    CHECK(std::get<std::string>(md.parse().invoke("a\nb")) == "<p>a\nb</p>\n");

    // use with a renderer: heading is given its token.
    MarkedExtension<std::string, std::string> ext = MarkedExtension<std::string, std::string>::create();
    MarkedExtension_renderer<std::string, std::string> rend = MarkedExtension_renderer<std::string, std::string>::create();
    // FINDING 5: a renderer taking `...args: [Tokens.Heading]` is typed
    // ts::Tuple<Tokens_Heading>, but the glue calls it spread: what arrives is
    // the heading itself, so args.get<0>() is null. Read the Tuple's handle
    // as the heading.
    rend.heading(MarkedExtension_renderer_heading<std::string, std::string>([](ts::Tuple<Tokens_Heading> args) {
        Tokens_Heading t;
        static_cast<ts::Handle&>(t) = args;
        return std::string("H") + std::to_string(int(t.depth())) + ":" + t.text() + "|";
    }));
    ext.renderer(rend);
    Mk custom = Mk::create({ext});
    CHECK(std::get<std::string>(custom.parse().invoke("## x")) == "H2:x|");

    // walkTokens visits every token, nested ones too.
    int visited = 0;
    MarkedExtension<std::string, std::string> walker = MarkedExtension<std::string, std::string>::create();
    walker.walkTokens(Token_to_void_or_Promise_or_void_or_Promise_array_fn([&](ts::Value token) {
        visited++;
        (void)token;
        return void_or_Promise_or_void_or_Promise_array(std::in_place_index<1>, ts::Array<>());
    }));
    Mk walked = Mk::create({walker});
    CHECK(std::get<std::string>(walked.parse().invoke("# a *b*")) == "<h1>a <em>b</em></h1>\n");
    CHECK(visited >= 4);
}
