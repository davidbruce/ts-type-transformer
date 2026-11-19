const dom = @import("dom");
const katex = @import("katex");

var html_buf: [65536]u8 = undefined;
var err_buf: [1024]u8 = undefined;

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, .{});
    if (cls.len > 0) el.asElement().setClassName(cls);
    if (text) |t| el.asNode().setTextContent(t);
    return el;
}

fn appendAndRelease(parent: dom.HTMLElement, child: dom.HTMLElement) void {
    const res = parent.asNode().appendChild(child.asNode());
    dom.jsRelease(res);
    dom.jsRelease(child);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "ParseError Inspector (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "throwOnError: false vs true"));
    appendAndRelease(app, head);

    const lenient = katex.KatexOptions.create();
    defer katex.jsRelease(lenient);
    lenient.setThrowOnError(false);
    lenient.setErrorColor("#f87171");

    const strict = katex.KatexOptions.create();
    defer katex.jsRelease(strict);
    strict.setThrowOnError(true);

    const bad_inputs = [_][]const u8{
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b",
    };

    for (bad_inputs) |tex| {
        const row = makeEl(doc, "div", "row", null);

        const left = makeEl(doc, "div", "cell mode-html", null);
        appendAndRelease(left, makeEl(doc, "div", "label", "throwOnError: false (#f87171)"));
        const l_slot = makeEl(doc, "div", "slot", null);
        const html = katex.renderToString(tex, .{ .options = lenient });
        defer katex.jsRelease(html);
        l_slot.asElement().setInnerHTML(html.copy(&html_buf));
        appendAndRelease(left, l_slot);
        appendAndRelease(row, left);

        const right = makeEl(doc, "div", "cell", null);
        appendAndRelease(right, makeEl(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        var err_text: []const u8 = "(did not throw)";
        if (katex.jsTry(katex.renderToString(tex, .{ .options = strict }))) |ok_str| {
            katex.jsRelease(ok_str);
        } else |_| {
            if (katex.jsTakeError()) |err| {
                defer katex.jsRelease(err);
                const pe: katex.ParseError = .{ .h = err.h };
                err_text = pe.getMessage().copy(&err_buf);
            }
        }
        appendAndRelease(right, makeEl(doc, "div", "err", err_text));
        appendAndRelease(row, right);

        appendAndRelease(app, row);
    }
}
