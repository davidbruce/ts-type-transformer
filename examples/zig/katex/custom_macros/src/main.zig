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

fn addMacro(macros: katex.KatexOptions_macros, name: []const u8, expansion: []const u8) void {
    const s = katex.JsString.from(expansion);
    defer katex.jsRelease(s);
    macros.setAt(name, .{ .h = s.h });
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Custom Macros (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "\\RR, \\NN, \\dd, \\norm"));
    appendAndRelease(app, head);

    const macros = katex.KatexOptions_macros.create();
    defer katex.jsRelease(macros);
    addMacro(macros, "\\RR", "\\mathbb{R}");
    addMacro(macros, "\\NN", "\\mathbb{N}");
    addMacro(macros, "\\dd", "\\mathrm{d}");
    addMacro(macros, "\\norm", "\\left\\| #1 \\right\\|");

    const opts = katex.KatexOptions.create();
    defer katex.jsRelease(opts);
    opts.setDisplayMode(true);
    opts.setMacros(macros);

    const legend = makeEl(doc, "div", "card", null);
    appendAndRelease(legend, makeEl(doc, "div", "label", "Defined Macros"));
    appendAndRelease(legend, makeEl(doc, "pre", "tex", "\\RR -> \\mathbb{R}\n\\NN -> \\mathbb{N}\n\\dd -> \\mathrm{d}\n\\norm -> \\left\\| #1 \\right\\|"));
    appendAndRelease(app, legend);

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
        appendAndRelease(card, makeEl(doc, "div", "label", e.name));
        appendAndRelease(card, makeEl(doc, "pre", "tex", e.tex));
        const slot = makeEl(doc, "div", "slot", null);
        const html = katex.renderToString(e.tex, .{ .options = opts });
        defer katex.jsRelease(html);
        slot.asElement().setInnerHTML(html.copy(&html_buf));
        appendAndRelease(card, slot);
        appendAndRelease(grid, card);
    }

    appendAndRelease(app, grid);
}
