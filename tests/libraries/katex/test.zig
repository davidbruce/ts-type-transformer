// KaTeX from Zig, in Node: expressions rendered with options, and a parse error. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

var buf: [65536]u8 = undefined;

// Whether a JavaScript string contains the text (the string is copied, up to the buffer).
fn has(s: b.JsString, needle: []const u8) bool {
    return std.mem.indexOf(u8, s.copy(&buf), needle) != null;
}

fn render(tex: []const u8, options: ?b.KatexOptions) b.JsString {
    return b.renderToString(tex, .{ .options = options });
}


fn onTrust(ctx: b.TrustContext) bool {
    var ubuf: [128]u8 = undefined;
    return switch (ctx) {
        .TrustContext_href => |h| std.mem.startsWith(u8, h.getUrl().copy(&ubuf), "https://example.com"),
        else => false,
    };
}

export fn run() void {
    // The version is a string.
    const version = b.getVersion();
    expect(version.len() > 0 and version.eql("0.19.1"), @src());
    expect(b.katex.getVersion().eql("0.19.1"), @src());

    // A plain expression: HTML and MathML both.
    const html = render("x^2 + y^2 = z^2", null);
    expect(html.len() > 100, @src());
    expect(has(html, "class=\"katex\""), @src());
    expect(has(html, "katex-mathml"), @src());
    expect(has(html, "katex-html"), @src());
    expect(has(html, "<annotation encoding=\"application/x-tex\">x^2 + y^2 = z^2</annotation>"), @src());
    expect(!has(html, "katex-display"), @src());

    // displayMode wraps it as a block.
    const opts = b.KatexOptions.create();
    expect(opts.getDisplayMode() == null, @src());
    opts.setDisplayMode(true);
    expect(opts.getDisplayMode().?, @src());
    const display = render("\\sum_{i=0}^n i", opts);
    expect(has(display, "katex-display"), @src());
    expect(has(display, "display=\"block\""), @src());
    expect(has(display, "&#x2211;") or has(display, "∑"), @src());

    // Only the MathML.
    const mathml = b.KatexOptions.create();
    mathml.setOutput(.mathml);
    expect(mathml.getOutput().? == .mathml, @src());
    const ml = render("a+b", mathml);
    expect(has(ml, "<math"), @src());
    expect(!has(ml, "katex-html"), @src());

    // Only the HTML.
    const only_html = b.KatexOptions.create();
    only_html.setOutput(.html);
    const hh = render("a+b", only_html);
    expect(has(hh, "katex-html") and !has(hh, "<math"), @src());

    // Bad input throws a ParseError; the call returns nothing and the exception is kept.
    const bad = render("\\frac{1", null);
    expect(bad.h == 0, @src());
    const err = b.jsTakeError();
    expect(err != null, @src());
    if (err) |e| {
        defer b.jsRelease(e);
        const name = b.jsErrorName(e);
        defer b.jsRelease(name);
        expect(name.eql("ParseError"), @src());
        const msg = b.jsErrorMessage(e);
        defer b.jsRelease(msg);
        expect(has(msg, "KaTeX parse error"), @src());
        // A thrown ParseError is a ParseError object.
        const pe: b.ParseError = .{ .h = e.h };
        expect(pe.getPosition() >= 0, @src());
        expect(has(pe.getMessage(), "KaTeX parse error"), @src());
        expect(!has(pe.getRawMessage(), "KaTeX parse error"), @src());
    }
    expect(b.jsTakeError() == null, @src());

    // The same as a Zig error.
    if (b.jsTry(render("\\unknowncommand", null))) |_| expect(false, @src()) else |e| {
        expect(e == error.JsException, @src());
        const ex = b.jsTakeError().?;
        defer b.jsRelease(ex);
        const m = b.jsErrorMessage(ex);
        defer b.jsRelease(m);
        expect(has(m, "Undefined control sequence: \\unknowncommand"), @src());
    }

    // throwOnError off: the source comes out in the error colour instead.
    const soft = b.KatexOptions.create();
    soft.setThrowOnError(false);
    soft.setErrorColor("#ff0000");
    expect(soft.getErrorColor().?.eql("#ff0000"), @src());
    const shown = render("\\frac{1", soft);
    expect(shown.h != 0, @src());
    expect(has(shown, "katex-error"), @src());
    expect(has(shown, "color:#ff0000"), @src());
    expect(has(shown, "\\frac{1"), @src());
    expect(b.jsTakeError() == null, @src());

    // strict as a union: a string, or a flag.
    const strict = b.KatexOptions.create();
    strict.setStrict(.{ .boolean = false });
    expect(strict.getStrict().?.boolean == false, @src());
    strict.setStrict(.{ .string = b.JsString.from("ignore") });
    expect(strict.getStrict().?.string.eql("ignore"), @src());
    expect(has(render("é", strict), "katex"), @src());

    // Macros are an index-signature object.
    const macros = b.KatexOptions_macros.create();
    const o = b.KatexOptions.create();
    o.setMacros(macros);
    expect(o.getMacros() != null, @src());
    // FINDING 1: KatexOptions_macros has no setAt/getAt, so \RR cannot be defined.

    // leqno / fleqn / maxExpand are plain properties.
    o.setLeqno(true);
    o.setMaxExpand(5);
    expect(o.getLeqno().? and o.getMaxExpand().? == 5, @src());
    o.setMinRuleThickness(0.1);
    expect(o.getMinRuleThickness().? == 0.1, @src());
    o.setMaxExpand(null);
    expect(o.getMaxExpand() == null, @src());

    // Trust: \href is refused unless trusted. Untrusted it renders in the error colour, trusted a link.
    const href = "\\href{https://example.com}{link}";
    const untrusted = b.KatexOptions.create();
    untrusted.setThrowOnError(false);
    expect(!has(render(href, untrusted), "<a href"), @src());
    const trusted = b.KatexOptions.create();
    trusted.setTrust(.{ .boolean = true });
    const link = render(href, trusted);
    expect(has(link, "<a href=\"https://example.com\""), @src());

    const trust_opts = b.KatexOptions.create();
    const cb = b.TrustContext_to_boolean_fn.from(onTrust, null);
    trust_opts.setTrust(.{ .TrustContext_to_boolean_fn = cb });
    expect(has(render(href, trust_opts), "<a href=\"https://example.com\""), @src());
    expect(!has(render("\\href{https://other.org}{link}", trust_opts), "<a href"), @src());

    // Output is deterministic.
    expect(render("e^{i\\pi}", null).eql(render("e^{i\\pi}", null).copy(&buf)), @src());
}
