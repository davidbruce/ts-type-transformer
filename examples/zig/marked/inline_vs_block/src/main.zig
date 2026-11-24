const dom = @import("dom");
const m = @import("marked");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: anytype) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (@TypeOf(text) != @TypeOf(null)) el.setTextContent(text);
    return el;
}

fn makeResultCard(doc: dom.Document, title: []const u8, html: m.JsString) dom.HTMLElement {
    const card = makeEl(doc, "div", "card", null);
    _ = card.appendChild(makeEl(doc, "div", "label", title));
    const out = makeEl(doc, "div", "md-out", null);
    out.setInnerHTML(html);
    _ = card.appendChild(out);
    _ = card.appendChild(makeEl(doc, "pre", "html-raw", html));
    return card;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Marked Inline vs Block (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "parse vs parseInline"));
    _ = app.appendChild(head);

    const inst = m.Marked.new(.{});
    const parse_fn = inst.getParse();
    const inline_fn = inst.getParseInline();

    const snippets = [_][]const u8{
        "Hello **world** and *italics*",
        "# Heading stays literal inline",
        "Inline `code` and [link](https://example.com)",
        "~~strike~~ and **nested *em***",
    };

    for (snippets) |s| {
        const row = makeEl(doc, "div", "row-card", null);
        _ = row.appendChild(makeEl(doc, "div", "label", "Markdown Snippet"));
        _ = row.appendChild(makeEl(doc, "pre", "src", s));

        const grid = makeEl(doc, "div", "grid", null);
        _ = grid.appendChild(makeResultCard(doc, "Block (parse)", parse_fn.invoke(s, .{})));
        _ = grid.appendChild(makeResultCard(doc, "Inline (parseInline)", inline_fn.invoke(s, .{})));
        _ = row.appendChild(grid);
        _ = app.appendChild(row);
    }
}
