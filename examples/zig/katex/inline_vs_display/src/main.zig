const dom = @import("dom");
const katex = @import("katex");

var html_buf: [65536]u8 = undefined;

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

fn renderCell(doc: dom.Document, label: []const u8, tex: []const u8, opts: katex.KatexOptions) dom.HTMLElement {
    const cell = makeEl(doc, "div", "cell", null);
    appendAndRelease(cell, makeEl(doc, "div", "label", label));
    const slot = makeEl(doc, "div", "slot", null);
    const html = katex.renderToString(tex, .{ .options = opts });
    defer katex.jsRelease(html);
    slot.asElement().setInnerHTML(html.copy(&html_buf));
    appendAndRelease(cell, slot);
    return cell;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Inline vs Display Mode (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "displayMode: false vs true"));
    appendAndRelease(app, head);

    const inline_opts = katex.KatexOptions.create();
    defer katex.jsRelease(inline_opts);
    inline_opts.setDisplayMode(false);
    const display_opts = katex.KatexOptions.create();
    defer katex.jsRelease(display_opts);
    display_opts.setDisplayMode(true);

    const Item = struct { in_lbl: []const u8, disp_lbl: []const u8, tex: []const u8 };
    const items = [_]Item{
        .{ .in_lbl = "Basel Series — inline", .disp_lbl = "Basel Series — display", .tex = "\\sum_{k=1}^{n} \\frac{1}{k^2} = \\frac{\\pi^2}{6}" },
        .{ .in_lbl = "Dilogarithm — inline", .disp_lbl = "Dilogarithm — display", .tex = "\\int_0^1 \\frac{\\ln(1+x)}{x}\\,dx" },
        .{ .in_lbl = "Sinc Limit — inline", .disp_lbl = "Sinc Limit — display", .tex = "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1" },
        .{ .in_lbl = "Binomial — inline", .disp_lbl = "Binomial — display", .tex = "\\binom{n}{k} = \\frac{n!}{k!(n-k)!}" },
    };

    for (items) |item| {
        const row = makeEl(doc, "div", "row", null);
        appendAndRelease(row, renderCell(doc, item.in_lbl, item.tex, inline_opts));
        appendAndRelease(row, renderCell(doc, item.disp_lbl, item.tex, display_opts));
        appendAndRelease(app, row);
    }
}
