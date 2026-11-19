const dom = @import("dom");
const m = @import("marked");

var html_buf: [2048]u8 = undefined;

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

fn makeResultCard(doc: dom.Document, title: []const u8, val: m.JsValue) dom.HTMLElement {
    const s: m.JsString = .{ .h = val.h };
    const html = s.copy(&html_buf);
    const card = makeEl(doc, "div", "card", null);
    appendAndRelease(card, makeEl(doc, "div", "label", title));
    const out = makeEl(doc, "div", "md-out", null);
    out.asElement().setInnerHTML(html);
    appendAndRelease(card, out);
    appendAndRelease(card, makeEl(doc, "pre", "html-raw", html));
    return card;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Marked Inline vs Block (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "parse vs parseInline"));
    appendAndRelease(app, head);

    const inst = m.Marked.new(m.JsArrayOf(m.MarkedExtension).new());
    defer m.jsRelease(inst);
    const parse_fn = inst.getParse();
    defer m.jsRelease(parse_fn);
    const inline_fn = inst.getParseInline();
    defer m.jsRelease(inline_fn);

    const snippets = [_][]const u8{
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***",
    };

    for (snippets) |s| {
        const row = makeEl(doc, "div", "row-card", null);
        appendAndRelease(row, makeEl(doc, "div", "label", "Markdown Snippet"));
        appendAndRelease(row, makeEl(doc, "pre", "src", s));

        const grid = makeEl(doc, "div", "grid", null);
        const block_val = parse_fn.invoke2(s, null);
        defer m.jsRelease(block_val);
        const inl_val = inline_fn.invoke2(s, null);
        defer m.jsRelease(inl_val);

        appendAndRelease(grid, makeResultCard(doc, "Block (parse)", block_val));
        appendAndRelease(grid, makeResultCard(doc, "Inline (parseInline)", inl_val));
        appendAndRelease(row, grid);
        appendAndRelease(app, row);
    }
}
