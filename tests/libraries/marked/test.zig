// marked from Zig, in Node: markdown parsed to HTML, lexed to tokens, extended with a renderer. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

var buf: [65536]u8 = undefined;

fn has(s: b.JsString, needle: []const u8) bool {
    return std.mem.indexOf(u8, s.copy(&buf), needle) != null;
}

// FINDING 5: the package's own parse() and parseInline() functions cannot be called. `parseInline` is
// bound as a struct with invoke methods but nothing returns one (no getter), and parse is getParse() ->
// JsValue. A Marked instance's getParse()/getParseInline() are used instead.
export fn run() void {
    // marked(src, options): the options object is required by the bindings.
    const none = b.marked_options_no_async.create();
    const html = b.marked("# Title\n\nSome *emphasis* and **strong** text.\n", none);
    expect(has(html, "<h1>Title</h1>"), @src());
    expect(has(html, "<em>emphasis</em>"), @src());
    expect(has(html, "<strong>strong</strong>"), @src());
    expect(has(html, "<p>Some "), @src());

    // Lists, code, links.
    const rich = b.marked("- one\n- two\n\n1. a\n2. b\n\n`code` and [link](http://x.test \"T\")\n\n```js\nlet x = 1;\n```\n", none);
    expect(has(rich, "<ul>\n<li>one</li>\n<li>two</li>\n</ul>"), @src());
    expect(has(rich, "<ol>"), @src());
    expect(has(rich, "<code>code</code>"), @src());
    expect(has(rich, "<a href=\"http://x.test\" title=\"T\">link</a>"), @src());
    expect(has(rich, "<pre><code class=\"language-js\">let x = 1;"), @src());

    // GFM tables and strikethrough; gfm off turns them into text.
    const gfm = b.marked("|a|b|\n|-|-|\n|1|2|\n\n~~gone~~\n", none);
    expect(has(gfm, "<table>") and has(gfm, "<th>a</th>") and has(gfm, "<td>2</td>"), @src());
    expect(has(gfm, "<del>gone</del>"), @src());
    const plain = b.marked_options_no_async.create();
    plain.setGfm(false);
    expect(plain.getGfm().? == false, @src());
    const nogfm = b.marked("~~gone~~", plain);
    expect(!has(nogfm, "<del>"), @src());

    // breaks: a newline inside a paragraph becomes <br>.
    const br = b.marked_options_no_async.create();
    br.setBreaks(true);
    expect(has(b.marked("a\nb", br), "a<br>b"), @src());
    expect(!has(b.marked("a\nb", none), "<br>"), @src());

    // The defaults are an options object.
    const defaults = b.getDefaults();
    expect(defaults.getGfm().?, @src());
    expect(defaults.getBreaks().? == false, @src());
    expect(defaults.getPedantic().? == false, @src());

    // A Marked instance: the same, and its own options.
    const m = b.Marked.new(b.JsArrayOf(b.MarkedExtension).new());
    const out = m.getParse().invoke2("hello *world*", null);
    expect(has(.{ .h = out.h }, "<p>hello <em>world</em></p>"), @src());
    const inl = m.getParseInline().invoke2("hello *world*", null);
    expect(has(.{ .h = inl.h }, "<em>world</em>") and !has(.{ .h = inl.h }, "<p>"), @src());

    // The lexer: block tokens.
    const tokens = b.Lexer.lex("# Head\n\npara with `code`\n\n- x\n- y\n", .{});
    expect(tokens.getLength() == 5, @src());

    // The Token union decodes every token by its `type`.
    const head = tokens.getAt(0).?.Tokens_Heading;
    expect(head.getDepth() == 1 and head.getText().eql("Head"), @src());
    expect(head.getRaw().eql("# Head"), @src());
    expect(tokens.getAt(1).? == .Tokens_Space, @src());
    const para = tokens.getAt(2).?.Tokens_Paragraph;
    expect(para.getText().eql("para with `code`"), @src());
    const kinds = [_][]const u8{ "heading", "space", "paragraph", "space", "list" };
    var it = tokens.iterate();
    var kidx: usize = 0;
    while (it.next()) |tok| : (kidx += 1) {
        const tag_type: []const u8 = switch (tok) {
            .Tokens_Generic => |g| g.getType().copy(&buf),
            inline else => |x| @TypeOf(x).type,
        };
        expect(std.mem.eql(u8, tag_type, kinds[kidx]), @src());
    }
    expect(kidx == kinds.len, @src());
    const list = tokens.getAt(4).?.Tokens_List;
    expect(list.getItems().len() == 2 and list.getOrdered() == false, @src());
    // The heading's inline tokens: one text token.
    expect(head.getTokens().len() == 1, @src());
    const inner = head.getTokens().get(0).Tokens_Text;
    expect(inner.getText().eql("Head"), @src());

    // marked.lexer and marked.parser are function values: called through their call functions.
    const lexed = b.callStringMarkedOptionsToTokensListFn(b.marked_ns.getLexer_2(), "# T\n\ntext", .{});
    expect(lexed.getLength() == 3, @src());
    const rendered = b.callTokenArrayMarkedOptionsFn(b.marked_ns.getParser_2(), lexed.as(b.JsArrayOf(b.Token)), .{});
    expect(has(.{ .h = rendered.h }, "<h1>T</h1>") and has(.{ .h = rendered.h }, "<p>text</p>"), @src());

    // lexInline.
    expect(b.Lexer.lexInline("a *b*", .{}).len() == 2, @src());

    // A renderer extension: headings rendered here.
    const ext = b.MarkedExtension.create();
    const renderer = b.MarkedExtension_renderer.create();
    renderer.setHeading(b.MarkedExtension_renderer_heading.from(renderHeading, null));
    ext.setRenderer(renderer);
    const custom = b.Marked.new(b.JsArrayOf(b.MarkedExtension).of(&.{ext}));
    const styled = custom.getParse().invoke2("# Hi\n\ntext", null);
    expect(has(.{ .h = styled.h }, "<h1 class=\"mine\">Hi</h1>") and has(.{ .h = styled.h }, "<p>text</p>"), @src());

    // A codespan renderer on an existing instance, via use(); the instance is returned for chaining.
    const ext2 = b.MarkedExtension.create();
    const r2 = b.MarkedExtension_renderer.create();
    r2.setCodespan(b.MarkedExtension_renderer_codespan.from(renderCodespan, null));
    ext2.setRenderer(r2);
    const chained = custom.use(b.JsArrayOf(b.MarkedExtension).of(&.{ext2}));
    expect(chained.h == custom.h or b.jsSame(chained, custom), @src());
    const both = custom.getParse().invoke2("# Hi\n\nuse `x` here", null);
    expect(has(.{ .h = both.h }, "<h1 class=\"mine\">") and has(.{ .h = both.h }, "<tt>x</tt>"), @src());

    // A preprocess hook sees the markdown first.
    const ext3 = b.MarkedExtension.create();
    const hooks = b.MarkedExtension_hooks.create();
    hooks.setPreprocess(b.MarkedExtension_hooks_preprocess.from(preprocess, null));
    ext3.setHooks(hooks);
    const hooked = b.Marked.new(b.JsArrayOf(b.MarkedExtension).of(&.{ext3}));
    const hr = hooked.getParse().invoke2("original", null);
    expect(has(.{ .h = hr.h }, "<p>replaced</p>"), @src());
}

fn renderCodespan(_: ?*anyopaque, args: b.JsTuple(&.{b.tokens.Codespan})) b.JsValue {
    const t = args.get(0);
    var out: [64]u8 = undefined;
    var text: [32]u8 = undefined;
    const body = t.getText().copy(&text);
    return b.JsValue.string(std.fmt.bufPrint(&out, "<tt>{s}</tt>", .{body}) catch "");
}

// The hook's arguments arrive as the array: item 0 is the markdown.
fn preprocess(_: ?*anyopaque, args: b.JsArray) b.string_or_Promise {
    const md: b.JsString = .{ .h = args.get(0).h };
    return .{ .string = b.JsString.from(if (md.eql("original")) "replaced" else "other") };
}

fn renderHeading(_: ?*anyopaque, args: b.JsTuple(&.{b.tokens.Heading})) b.JsValue {
    const t = args.get(0);
    var out: [64]u8 = undefined;
    var title: [32]u8 = undefined;
    const text = std.fmt.bufPrint(&out, "<h{d} class=\"mine\">{s}</h{d}>", .{ @as(u32, @intFromFloat(t.getDepth())), t.getText().copy(&title), @as(u32, @intFromFloat(t.getDepth())) }) catch "";
    return b.JsValue.string(text);
}
