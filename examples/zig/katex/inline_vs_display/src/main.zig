const dom = @import("dom");
const katex = @import("katex");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

fn renderCell(doc: dom.Document, label: []const u8, tex: []const u8, opts: katex.KatexOptions) dom.HTMLElement {
    const cell = makeEl(doc, "div", "cell", null);
    _ = cell.appendChild(makeEl(doc, "div", "label", label));
    const slot = makeEl(doc, "div", "slot", null);
    slot.setInnerHTML(katex.renderToString(tex, opts));
    _ = cell.appendChild(slot);
    return cell;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Inline vs Display Mode (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "displayMode: false vs true"));
    _ = app.appendChild(head);

    const inline_opts = katex.KatexOptions.init(.{ .displayMode = false });
    const display_opts = katex.KatexOptions.init(.{ .displayMode = true });

    const Item = struct { in_lbl: []const u8, disp_lbl: []const u8, tex: []const u8 };
    const items = [_]Item{
        .{ .in_lbl = "Basel Series — inline", .disp_lbl = "Basel Series — display", .tex = "\\sum_{k=1}^{n} \\frac{1}{k^2} = \\frac{\\pi^2}{6}" },
        .{ .in_lbl = "Dilogarithm — inline", .disp_lbl = "Dilogarithm — display", .tex = "\\int_0^1 \\frac{\\ln(1+x)}{x}\\,dx" },
        .{ .in_lbl = "Sinc Limit — inline", .disp_lbl = "Sinc Limit — display", .tex = "\\lim_{x \\to 0} \\frac{\\sin x}{x} = 1" },
        .{ .in_lbl = "Binomial — inline", .disp_lbl = "Binomial — display", .tex = "\\binom{n}{k} = \\frac{n!}{k!(n-k)!}" },
    };

    for (items) |item| {
        const row = makeEl(doc, "div", "row", null);
        _ = row.appendChild(renderCell(doc, item.in_lbl, item.tex, inline_opts));
        _ = row.appendChild(renderCell(doc, item.disp_lbl, item.tex, display_opts));
        _ = app.appendChild(row);
    }
}
