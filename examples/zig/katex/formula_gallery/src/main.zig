const std = @import("std");
const dom = @import("dom");
const katex = @import("katex");

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
    appendAndRelease(head, makeEl(doc, "h1", "", "KaTeX Formula Gallery (Zig)"));
    const ver = katex.getVersion();
    defer katex.jsRelease(ver);
    var vbuf: [64]u8 = undefined;
    var bbuf: [72]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "katex v{s}", .{ver.copy(&vbuf)}) catch "katex";
    appendAndRelease(head, makeEl(doc, "span", "badge", badge_txt));
    appendAndRelease(app, head);

    const grid = makeEl(doc, "div", "grid", null);
    const opts = katex.KatexOptions.create();
    defer katex.jsRelease(opts);
    opts.setDisplayMode(true);

    const Entry = struct { name: []const u8, tex: []const u8 };
    const formulas = [_]Entry{
        .{ .name = "Euler's Identity", .tex = "e^{i\\pi} + 1 = 0" },
        .{ .name = "Quadratic Formula", .tex = "x = \\frac{-b \\pm \\sqrt{b^2 - 4ac}}{2a}" },
        .{ .name = "Gaussian Integral", .tex = "\\int_{-\\infty}^{\\infty} e^{-x^2}\\,dx = \\sqrt{\\pi}" },
        .{ .name = "Maxwell-Faraday", .tex = "\\nabla \\times \\mathbf{E} = -\\frac{\\partial \\mathbf{B}}{\\partial t}" },
        .{ .name = "Fourier Transform", .tex = "\\hat{f}(\\xi) = \\int_{-\\infty}^{\\infty} f(x)e^{-2\\pi i x \\xi}\\,dx" },
    };

    for (formulas) |f| {
        const card = makeEl(doc, "div", "card", null);
        appendAndRelease(card, makeEl(doc, "div", "label", f.name));
        appendAndRelease(card, makeEl(doc, "pre", "tex", f.tex));
        const slot = makeEl(doc, "div", "slot", null);
        katex.render(f.tex, slot, .{ .options = opts });
        appendAndRelease(card, slot);
        appendAndRelease(grid, card);
    }

    appendAndRelease(app, grid);
}
