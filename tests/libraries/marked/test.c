// marked from C, in Node: Markdown to HTML, inline Markdown, the lexer's tokens,
// an instance with options, and extensions (a renderer, walkTokens).
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// marked(src) has no default for its options: none given.
#define NO_OPTIONS ((marked_options_no_async){ 0 })

static bool contains(JsString s, const char* needle) {
    static char buf[1 << 16];
    uint32_t n = JsString_len(s), m = ts_strlen(needle);
    if (n >= sizeof buf) return false;
    JsString_copy(s, buf, sizeof buf);
    for (uint32_t i = 0; i + m <= n; i++) {
        uint32_t j = 0;
        while (j < m && buf[i + j] == needle[j]) j++;
        if (j == m) return true;
    }
    return false;
}

static uint32_t token_h(Marked_Token tok) {
    switch (tok.tag) {
    case Marked_Token_Marked_Tokens_Blockquote: return tok.as.Marked_Tokens_Blockquote.h;
    case Marked_Token_Marked_Tokens_Br: return tok.as.Marked_Tokens_Br.h;
    case Marked_Token_Marked_Tokens_Checkbox: return tok.as.Marked_Tokens_Checkbox.h;
    case Marked_Token_Marked_Tokens_Code: return tok.as.Marked_Tokens_Code.h;
    case Marked_Token_Marked_Tokens_Codespan: return tok.as.Marked_Tokens_Codespan.h;
    case Marked_Token_Marked_Tokens_Def: return tok.as.Marked_Tokens_Def.h;
    case Marked_Token_Marked_Tokens_Del: return tok.as.Marked_Tokens_Del.h;
    case Marked_Token_Marked_Tokens_Em: return tok.as.Marked_Tokens_Em.h;
    case Marked_Token_Marked_Tokens_Escape: return tok.as.Marked_Tokens_Escape.h;
    case Marked_Token_Marked_Tokens_Heading: return tok.as.Marked_Tokens_Heading.h;
    case Marked_Token_Marked_Tokens_Hr: return tok.as.Marked_Tokens_Hr.h;
    case Marked_Token_Marked_Tokens_HTML: return tok.as.Marked_Tokens_HTML.h;
    case Marked_Token_Marked_Tokens_Image: return tok.as.Marked_Tokens_Image.h;
    case Marked_Token_Marked_Tokens_Link: return tok.as.Marked_Tokens_Link.h;
    case Marked_Token_Marked_Tokens_List: return tok.as.Marked_Tokens_List.h;
    case Marked_Token_Marked_Tokens_ListItem: return tok.as.Marked_Tokens_ListItem.h;
    case Marked_Token_Marked_Tokens_Paragraph: return tok.as.Marked_Tokens_Paragraph.h;
    case Marked_Token_Marked_Tokens_Space: return tok.as.Marked_Tokens_Space.h;
    case Marked_Token_Marked_Tokens_Strong: return tok.as.Marked_Tokens_Strong.h;
    case Marked_Token_Marked_Tokens_Table: return tok.as.Marked_Tokens_Table.h;
    case Marked_Token_Marked_Tokens_Tag: return tok.as.Marked_Tokens_Tag.h;
    case Marked_Token_Marked_Tokens_Text: return tok.as.Marked_Tokens_Text.h;
    case Marked_Token_Marked_Tokens_Generic: return tok.as.Marked_Tokens_Generic.h;
    default: return 0;
    }
}

// What a token is: its `type`, whatever kind it is.
static bool token_is(JsValue token, const char* type) {
    JsString t = Marked_Tokens_Generic_get_type((Marked_Tokens_Generic){ token.h });
    bool ok = JsString_equals(t, type);
    js_release(t);
    return ok;
}

// The renderer for headings: a class by depth, the heading's own text.
static int headings_rendered;
static JsValue render_heading(void* ctx, Marked_Tokens_Heading h) {
    headings_rendered++;
    char html[128];
    uint32_t depth = (uint32_t)Marked_Tokens_Heading_get_depth(h);
    JsString text = Marked_Tokens_Heading_get_text(h);
    uint32_t n = JsString_copy(text, html + 40, 64);
    js_release(text);
    const char* open = "<h  class=\"title\">";
    uint32_t k = 0;
    for (; open[k]; k++) html[k] = open[k];
    html[2] = '0' + depth;
    for (uint32_t i = 0; i < n; i++) html[k + i] = html[40 + i];
    const char* close = "</h >";
    for (uint32_t i = 0; close[i]; i++) html[k + n + i] = close[i];
    html[k + n + 3] = '0' + depth;
    html[k + n + 5] = 0;
    return js_string(html);
}

// walkTokens: every heading goes one level down.
static int walked, walked_headings;
static JsValue demote(void* ctx, JsValue token) {
    walked++;
    if (token_is(token, "heading")) {
        Marked_Tokens_Heading h = { token.h };
        Marked_Tokens_Heading_set_depth(h, Marked_Tokens_Heading_get_depth(h) + 1);
        walked_headings++;
    }
    return (JsValue){ 0 };
}

// A promise from the async option.
static bool async_ok;
static void on_async(void* ctx, bool ok, JsValue v) {
    if (!ok) return;
    JsString s = js_to_string(v);
    async_ok = contains(s, "<strong>async</strong>");
}
static Promise pending;
__attribute__((export_name("finish"))) void finish(void) { CHECK(async_ok); }

__attribute__((export_name("run"))) void run(void) {
    // Block Markdown to HTML.
    JsString html = marked("# Title\n\nHello *world* and **bold**.\n\n- one\n- two\n", NO_OPTIONS);
    CHECK(contains(html, "<h1>Title</h1>"));
    CHECK(contains(html, "<em>world</em>"));
    CHECK(contains(html, "<strong>bold</strong>"));
    CHECK(contains(html, "<ul>") && contains(html, "<li>one</li>") && contains(html, "<li>two</li>"));
    js_release(html);

    // Code, links, images, quotes.
    JsString rich = marked("`x`\n\n```js\nlet a = 1;\n```\n\n[site](https://example.com \"T\") ![alt](a.png)\n\n> quoted\n", NO_OPTIONS);
    CHECK(contains(rich, "<code>x</code>"));
    CHECK(contains(rich, "<pre><code class=\"language-js\">let a = 1;"));
    CHECK(contains(rich, "<a href=\"https://example.com\" title=\"T\">site</a>"));
    CHECK(contains(rich, "<img src=\"a.png\" alt=\"alt\">"));
    CHECK(contains(rich, "<blockquote>"));
    js_release(rich);

    // GitHub tables, on by default; off with gfm false.
    const char* table = "| a | b |\n|---|---|\n| 1 | 2 |\n";
    JsString t = marked(table, NO_OPTIONS);
    CHECK(contains(t, "<table>") && contains(t, "<th>a</th>") && contains(t, "<td>2</td>"));
    js_release(t);
    marked_options_no_async plain = marked_options_no_async_create();
    marked_options_no_async_set_gfm(plain, js_some_bool(false));
    JsString flat = marked(table, plain);
    CHECK(!contains(flat, "<table>"));
    js_release(flat);

    // breaks: a newline is a <br>.
    marked_options_no_async soft = marked_options_no_async_create();
    marked_options_no_async_set_breaks(soft, js_some_bool(true));
    JsString br = marked("one\ntwo", soft);
    CHECK(contains(br, "one<br>two"));
    js_release(br);
    JsString nobr = marked("one\ntwo", NO_OPTIONS);
    CHECK(!contains(nobr, "<br>"));
    js_release(nobr);

    // An instance, with its own parse and parseInline.
    JsArray_MarkedExtension none = JsArray_MarkedExtension_new();
    Marked m = Marked_new(none);
    JsString inline_html = Marked_parse_invoke(Marked_get_parseInline(m), "a *b* `c`", (marked_options_no_async){ 0 });
    CHECK(JsString_equals(inline_html, "a <em>b</em> <code>c</code>"));
    js_release(inline_html);
    JsString block = Marked_parse_invoke(Marked_get_parse(m), "para *x*", (marked_options_no_async){ 0 });
    CHECK(JsString_equals(block, "<p>para <em>x</em></p>\n"));
    js_release(block);

    // The lexer: the tokens of a document.
    Marked_TokensList tokens = Marked_lexer(m, "# Head\n\ntext **b**\n\n1. a\n2. b\n\n---\n");
    CHECK(Marked_TokensList_get_length(tokens) >= 4);
    Marked_Token first = Marked_TokensList_get_at(tokens, 0);
    CHECK(first.tag == Marked_Token_Marked_Tokens_Heading);
    Marked_Tokens_Heading heading = first.as.Marked_Tokens_Heading;
    CHECK(Marked_Tokens_Heading_get_depth(heading) == 1);
    JsString heading_text = Marked_Tokens_Heading_get_text(heading);
    CHECK(JsString_equals(heading_text, "Head"));
    js_release(heading_text);
    JsString raw = Marked_Tokens_Heading_get_raw(heading);
    CHECK(contains(raw, "# Head"));
    js_release(raw);
    // The inline tokens of a heading.
    JsArray_Marked_Token inner = Marked_Tokens_Heading_get_tokens(heading);
    CHECK(JsArray_Marked_Token_length(inner) == 1);
    Marked_Token inner0 = JsArray_Marked_Token_get(inner, 0);
    CHECK(inner0.tag == Marked_Token_Marked_Tokens_Text);
    js_release((JsValue){ token_h(inner0) });
    js_release(inner);
    // Walk the list for a paragraph with strong text and an ordered list.
    int paragraphs = 0, lists = 0, hrs = 0;
    uint32_t count = (uint32_t)Marked_TokensList_get_length(tokens);
    for (uint32_t i = 0; i < count; i++) {
        Marked_Token tok = Marked_TokensList_get_at(tokens, i);
        if (tok.tag == Marked_Token_Marked_Tokens_Paragraph) paragraphs++;
        if (tok.tag == Marked_Token_Marked_Tokens_List) {
            lists++;
            Marked_Tokens_List list = tok.as.Marked_Tokens_List;
            CHECK(Marked_Tokens_List_get_ordered(list));
            JsArray_Marked_Tokens_ListItem items = Marked_Tokens_List_get_items(list);
            CHECK(JsArray_Marked_Tokens_ListItem_length(items) == 2);
            js_release(items);
        }
        if (tok.tag == Marked_Token_Marked_Tokens_Hr) hrs++;
        js_release((JsValue){ token_h(tok) });
    }
    CHECK(paragraphs == 1 && lists == 1 && hrs == 1);
    js_release((JsValue){ token_h(first) });
    js_release(tokens);

    // Options on the instance: the lexer takes them too.
    MarkedOptions opts = MarkedOptions_create();
    MarkedOptions_set_gfm(opts, js_some_bool(false));
    Marked_TokensList raw_table = Marked_lexer(m, table, opts);
    Marked_Token tab0 = Marked_TokensList_get_at(raw_table, 0);
    CHECK(tab0.tag == Marked_Token_Marked_Tokens_Paragraph);
    js_release((JsValue){ token_h(tab0) });
    js_release(raw_table);

    // `use` with a renderer: headings made by a function here.
    MarkedExtension ext = MarkedExtension_create();
    MarkedExtension_renderer rend = MarkedExtension_renderer_create();
    MarkedExtension_renderer_set_heading(rend, MarkedExtension_renderer_heading_from(render_heading, 0));
    MarkedExtension_set_renderer(ext, rend);
    JsArray_MarkedExtension exts = JsArray_MarkedExtension_new();
    JsArray_MarkedExtension_push(exts, ext);
    Marked styled = Marked_new(exts);
    JsString custom = Marked_parse_invoke(Marked_get_parse(styled), "## Sub\n\nbody", (marked_options_no_async){ 0 });
    CHECK(contains(custom, "<h2 class=\"title\">Sub</h2>"));
    CHECK(contains(custom, "<p>body</p>"));
    CHECK(headings_rendered == 1);
    js_release(custom);
    // The default instance is untouched.
    JsString default_h = Marked_parse_invoke(Marked_get_parse(m), "## Sub", (marked_options_no_async){ 0 });
    CHECK(JsString_equals(default_h, "<h2>Sub</h2>\n"));
    js_release(default_h);

    // `use` with walkTokens: every token is visited, headings change.
    MarkedExtension walker = MarkedExtension_create();
    MarkedExtension_set_walkTokens(walker, MarkedExtension_walkTokens_from(demote, 0));
    JsArray_MarkedExtension wexts = JsArray_MarkedExtension_new();
    JsArray_MarkedExtension_push(wexts, walker);
    Marked walking = Marked_new(wexts);
    JsString demoted = Marked_parse_invoke(Marked_get_parse(walking), "# Big\n\ntext", (marked_options_no_async){ 0 });
    CHECK(contains(demoted, "<h2>Big</h2>"));
    CHECK(walked_headings == 1 && walked >= 4);
    js_release(demoted);

    // Marked_use adds an extension to an existing instance.
    Marked grown = Marked_new(JsArray_MarkedExtension_new());
    JsArray_MarkedExtension more = JsArray_MarkedExtension_new();
    MarkedExtension gfm_off = MarkedExtension_create();
    MarkedExtension_set_breaks(gfm_off, js_some_bool(true));
    JsArray_MarkedExtension_push(more, gfm_off);
    Marked_use(grown, more);
    JsString broken = Marked_parse_invoke(Marked_get_parse(grown), "x\ny", (marked_options_no_async){ 0 });
    CHECK(contains(broken, "x<br>y"));
    js_release(broken);

    // The async option gives a promise.
    // marked_options is `MarkedOptions & { async: true }`: _create sets async.
    pending = marked_async("**async**", marked_options_create());
    Promise_then(pending, on_async, 0);

    // The defaults are an options object.
    MarkedOptions defaults = marked_getDefaults();
    CHECK(MarkedOptions_get_gfm(defaults).has && MarkedOptions_get_gfm(defaults).value);
    CHECK(MarkedOptions_get_breaks(defaults).has && !MarkedOptions_get_breaks(defaults).value);

    // The top-level lexer and parser are function values: call_<Type> calls them.
    Marked_TokensList lexed = marked_call_string_MarkedOptions_to_TokensList_fn(marked_get_lexer(), "# H\n\ntext *x*");
    CHECK(Marked_TokensList_get_length(lexed) >= 2);
    Marked_Token lexed0 = Marked_TokensList_get_at(lexed, 0);
    CHECK(lexed0.tag == Marked_Token_Marked_Tokens_Heading);
    js_release((JsValue){ token_h(lexed0) });
    JsString parsed = js_to_string(marked_call_Token_array_MarkedOptions_fn(marked_get_parser(), (JsArray_Marked_Token){ lexed.h }));
    CHECK(contains(parsed, "<h1>H</h1>") && contains(parsed, "<em>x</em>"));
    js_release(parsed);

    // marked's own `use` changes the defaults for marked(): last, so nothing above sees it.
    JsArray_MarkedExtension global = JsArray_MarkedExtension_new();
    MarkedExtension global_breaks = MarkedExtension_create();
    MarkedExtension_set_breaks(global_breaks, js_some_bool(true));
    JsArray_MarkedExtension_push(global, global_breaks);
    js_release(marked_use(global));
    JsString global_br = marked("p\nq", NO_OPTIONS);
    CHECK(contains(global_br, "p<br>q"));
    js_release(global_br);
}
