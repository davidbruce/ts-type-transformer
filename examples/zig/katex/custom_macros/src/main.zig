const dom = @import("dom");
const katex = @import("katex");

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
    _ = head.appendChild(makeEl(doc, "h1", "", "Custom Macros (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "\\RR, \\NN, \\dd, \\norm"));
    _ = app.appendChild(head);

    const macros = katex.JsRecordOf(katex.JsValue).create();
    macros.setAt("\\RR", "\\mathbb{R}");
    macros.setAt("\\NN", "\\mathbb{N}");
    macros.setAt("\\dd", "\\mathrm{d}");
    macros.setAt("\\norm", "\\left\\| #1 \\right\\|");

    const opts = katex.KatexOptions.init(.{
        .displayMode = true,
        .macros = macros,
    });

    const legend = makeEl(doc, "div", "card", null);
    _ = legend.appendChild(makeEl(doc, "div", "label", "Defined Macros"));
    _ = legend.appendChild(makeEl(doc, "pre", "tex", "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"));
    _ = app.appendChild(legend);

    const grid = makeEl(doc, "div", "grid", null);
    const Expr = struct { name: []const u8, tex: []const u8 };
    const exprs = [_]Expr{
        .{ .name = "Euclidean Space", .tex = "v \\in \\RR^n,\\quad k \\in \\NN" },
        .{ .name = "Vector Norm", .tex = "\\norm{x + y} \\le \\norm{x} + \\norm{y}" },
        .{ .name = "Differential Form", .tex = "\\int_a^b f(x)\\,\\dd x = F(b) - F(a)" },
        .{ .name = "Inner Product Norm", .tex = "\\norm{\\phi}_2 = \\sqrt{\\int_{\\RR} |\\phi(t)|^2\\,\\dd t}" },
    };

    for (exprs) |e| {
        const card = makeEl(doc, "div", "card", null);
        _ = card.appendChild(makeEl(doc, "div", "label", e.name));
        _ = card.appendChild(makeEl(doc, "pre", "tex", e.tex));
        const slot = makeEl(doc, "div", "slot", null);
        slot.setInnerHTML(katex.renderToStringWith(e.tex, opts));
        _ = card.appendChild(slot);
        _ = grid.appendChild(card);
    }

    _ = app.appendChild(grid);
}
