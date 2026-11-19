const dom = @import("dom");
const m = @import("marked");

var html_buf: [8192]u8 = undefined;

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
    appendAndRelease(head, makeEl(doc, "h1", "", "Marked Document Preview (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "marked"));
    appendAndRelease(app, head);

    const markdown_src =
        "# Marked Wireframe Spec\n\n" ++
        "## Features\n\n" ++
        "Typed bindings with *emphasis*, **strong**, `inline code`, and [reference](https://example.com \"Spec\").\n\n" ++
        "- Lexer AST\n" ++
        "- Custom renderer\n" ++
        "- GFM tables\n\n" ++
        "1. Parse\n" ++
        "2. Walk\n" ++
        "3. Render\n\n" ++
        "> Typed bindings from TypeScript definitions.\n\n" ++
        "```ts\n" ++
        "const ok: boolean = true;\n" ++
        "```\n\n" ++
        "---\n";

    const grid = makeEl(doc, "div", "grid", null);

    const src_card = makeEl(doc, "div", "card", null);
    appendAndRelease(src_card, makeEl(doc, "div", "label", "Markdown Source"));
    appendAndRelease(src_card, makeEl(doc, "pre", "src", markdown_src));

    const out_card = makeEl(doc, "div", "card", null);
    appendAndRelease(out_card, makeEl(doc, "div", "label", "Rendered HTML"));
    const md_out = makeEl(doc, "div", "md-out", null);
    const none = m.marked_options_no_async.create();
    defer m.jsRelease(none);
    const html = m.marked(markdown_src, none);
    defer m.jsRelease(html);
    md_out.asElement().setInnerHTML(html.copy(&html_buf));

    appendAndRelease(out_card, md_out);
    appendAndRelease(grid, src_card);
    appendAndRelease(grid, out_card);
    appendAndRelease(app, grid);
}
