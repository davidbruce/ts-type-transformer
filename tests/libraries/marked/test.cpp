// marked from C++, in Node: Markdown to HTML, the lexer's tokens, a Marked
// instance with options, and an extension with a renderer and walkTokens.
// Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static bool has(const std::string& s, std::string_view part) { return s.find(part) != std::string::npos; }

static std::string token_type(const Token& tok) {
    return std::visit([](const auto& x) -> std::string {
        using X = std::decay_t<decltype(x)>;
        if constexpr (std::is_same_v<X, Tokens_Generic>) return x.type();
        else return std::string(X::type);
    }, tok);
}

extern "C" __attribute__((export_name("run"))) void run() {
    // The whole document to HTML.
    std::string html = marked("# Hello *world*\n\nSome `code` and [link](http://x.com \"t\")\n\n- a\n- b\n");
    CHECK(has(html, "<h1>Hello <em>world</em></h1>"));
    CHECK(has(html, "<code>code</code>"));
    CHECK(has(html, "<a href=\"http://x.com\" title=\"t\">link</a>"));
    CHECK(has(html, "<ul>\n<li>a</li>\n<li>b</li>\n</ul>"));

    // Options made here, given an instance with use: breaks turns a newline into <br>.
    MarkedExtension breaks = MarkedExtension::create();
    breaks.breaks(true);
    CHECK(breaks.breaks() == true);
    CHECK(!breaks.gfm());
    Marked soft = Marked::create();
    soft.use(breaks);
    CHECK(soft.parse("a\nb") == "<p>a<br>b</p>\n");
    CHECK(marked("a\nb") == "<p>a\nb</p>\n");

    // gfm off: no strikethrough.
    MarkedExtension plain = MarkedExtension::create();
    plain.gfm(false);
    Marked flat = Marked::create();
    flat.use(plain);
    CHECK(flat.parse("~~x~~") == "<p>~~x~~</p>\n");
    CHECK(marked("~~x~~") == "<p><del>x</del></p>\n");

    // The function-typed consts are called via operator().
    TokensList viaConst = marked_ns::get_lexer()("# T\n\npara", std::nullopt);
    CHECK(viaConst.length() == 3);
    CHECK(std::holds_alternative<Tokens_Heading>(*viaConst.at(0)));
    ts::Array<Token> tokenArray;
    static_cast<ts::Handle&>(tokenArray) = viaConst;
    ts::Value out = marked_ns::get_parser()(tokenArray, std::nullopt);
    CHECK(out.to_string() == "<h1>T</h1>\n<p>para</p>\n");
    Marked md = Marked::create(ts::Array<MarkedExtension>::create());
    TokensList list = md.lexer("# H *x*\n\npara **b**\n\n- x\n- y\n\n```js\nlet a\n```\n\n| a | b |\n|---|---|\n| 1 | 2 |\n");
    CHECK(list.length() == 9);
    std::string types;
    for (const Token& v : list) types += token_type(v) + " ";
    CHECK(types == "heading space paragraph space list space code space table ");
    for (const Token& v : list) CHECK(v.type() == token_type(v));

    Tokens_Heading heading = std::get<Tokens_Heading>(*list.at(0));
    CHECK(heading.depth() == 1);
    CHECK(heading.text() == "H *x*");
    CHECK(heading.raw() == "# H *x*");
    CHECK(Tokens_Heading::type == "heading");
    CHECK(heading.tokens().size() == 2);

    // The nested inline tokens.
    ts::Array<Token> inner = heading.tokens();
    CHECK(std::holds_alternative<Tokens_Text>(inner[0]));
    Token emv = inner[1];
    CHECK(std::holds_alternative<Tokens_Em>(emv));
    CHECK(std::get<Tokens_Em>(emv).text() == "x");

    Tokens_Paragraph para = std::get<Tokens_Paragraph>(*list.at(2));
    CHECK(para.text() == "para **b**");
    Tokens_List ul = std::get<Tokens_List>(*list.at(4));
    CHECK(!ul.ordered());
    CHECK(ul.items().size() == 2);
    CHECK(ul.items()[1].text() == "y");
    Tokens_Code code = std::get<Tokens_Code>(*list.at(6));
    CHECK(code.lang() == "js");
    CHECK(code.text() == "let a");
    Tokens_Table table = std::get<Tokens_Table>(*list.at(8));
    CHECK(table.header().size() == 2);
    CHECK(table.header()[0].text() == "a");
    CHECK(table.rows().size() == 1);
    CHECK(table.rows()[0][1].text() == "2");

    // The static lexer, and inline tokens.
    TokensList quick = ::Lexer::lex_static("*a* b");
    CHECK(quick.length() == 1);
    ts::Array<Token> inlineTokens = ::Lexer::lexInline("`c` and **d**");
    CHECK(inlineTokens.size() == 3);

    // A Marked instance made with an extension holding options.
    MarkedExtension opts = MarkedExtension::create();
    opts.breaks(true);
    Marked instance = Marked::create({opts});
    CHECK(instance.parse("a\nb") == "<p>a<br>b</p>\n");
    CHECK(instance.parseInline("**bold** and _it_") == "<strong>bold</strong> and <em>it</em>");
    // The shared instance is untouched.
    CHECK(md.parse("a\nb") == "<p>a\nb</p>\n");

    // use with a renderer: heading is given its token.
    MarkedExtension ext = MarkedExtension::create();
    RendererObject rend = RendererObject::create();
    rend.heading(RendererObjectHeadingFn([](ts::Tuple<Tokens_Heading> args) {
        Tokens_Heading t = args.get<0>();
        return std::string("H") + std::to_string(int(t.depth())) + ":" + t.text() + "|";
    }));
    ext.renderer(rend);
    Marked custom = Marked::create({ext});
    CHECK(custom.parse("## x") == "H2:x|");

    // walkTokens visits every token, nested ones too.
    int visited = 0;
    MarkedExtension walker = MarkedExtension::create();
    walker.walkTokens(MarkedExtensionWalkTokensFn([&](const Token& token) {
        visited++;
        (void)token;
    }));
    Marked walked = Marked::create({walker});
    CHECK(walked.parse("# a *b*") == "<h1>a <em>b</em></h1>\n");
    CHECK(visited >= 4);
}
