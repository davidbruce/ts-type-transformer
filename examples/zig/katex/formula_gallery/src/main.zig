const std = @import("std");
const dom = @import("dom");
const katex = @import("katex");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
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
    _ = head.appendChild(makeEl(doc, "h1", "", "KaTeX Formula Gallery (Zig)"));
    var bbuf: [72]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "katex v{f}", .{katex.getVersion()}) catch "katex";
    _ = head.appendChild(makeEl(doc, "span", "badge", badge_txt));
    _ = app.appendChild(head);

    const grid = makeEl(doc, "div", "grid", null);
    const opts = katex.KatexOptions.init(.{ .displayMode = true });

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
        _ = card.appendChild(makeEl(doc, "div", "label", f.name));
        _ = card.appendChild(makeEl(doc, "pre", "tex", f.tex));
        const slot = makeEl(doc, "div", "slot", null);
        katex.render(f.tex, slot, opts);
        _ = card.appendChild(slot);
        _ = grid.appendChild(card);
    }

    _ = app.appendChild(grid);
}
