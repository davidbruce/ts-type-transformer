const dom = @import("dom");
const m = @import("marked");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Marked Document Preview (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "marked"));
    _ = app.appendChild(head);

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
    _ = src_card.appendChild(makeEl(doc, "div", "label", "Markdown Source"));
    _ = src_card.appendChild(makeEl(doc, "pre", "src", markdown_src));

    const out_card = makeEl(doc, "div", "card", null);
    _ = out_card.appendChild(makeEl(doc, "div", "label", "Rendered HTML"));
    const md_out = makeEl(doc, "div", "md-out", null);
    md_out.setInnerHTML(m.marked(markdown_src));

    _ = out_card.appendChild(md_out);
    _ = grid.appendChild(src_card);
    _ = grid.appendChild(out_card);
    _ = app.appendChild(grid);
}
