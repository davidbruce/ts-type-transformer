const std = @import("std");
const dom = @import("dom");
const katex = @import("katex");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

fn addCard(doc: dom.Document, grid: dom.HTMLElement, label: []const u8, cls: []const u8, tex: []const u8, opts: anytype) void {
    const html = katex.renderToStringWith(tex, opts);
    var lbuf: [32]u8 = undefined;
    const len_txt = std.fmt.bufPrint(&lbuf, "{d} chars", .{html.len()}) catch "0 chars";

    const card = makeEl(doc, "div", cls, null);
    const row = makeEl(doc, "div", "head", null);
    _ = row.appendChild(makeEl(doc, "div", "label", label));
    _ = row.appendChild(makeEl(doc, "span", "badge", len_txt));
    _ = card.appendChild(row);
    const slot = makeEl(doc, "div", "slot", null);
    slot.setInnerHTML(html);
    _ = card.appendChild(slot);
    _ = grid.appendChild(card);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Output & Layout Options (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "html | mathml | leqno | fleqn | maxSize | strict"));
    _ = app.appendChild(head);

    const grid = makeEl(doc, "div", "grid", null);
    addCard(doc, grid, "output: html", "card mode-html", "a^2 + b^2 = c^2", .{ .output = .html });
    addCard(doc, grid, "output: mathml", "card", "\\frac{1}{1 + x^2}", .{ .output = .mathml });
    addCard(doc, grid, "output: htmlAndMathml", "card", "\\sqrt{\\alpha + \\beta}", .{ .output = .htmlAndMathml });
    addCard(doc, grid, "displayMode + leqno", "card", "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}", .{
        .displayMode = true,
        .leqno = true,
    });
    addCard(doc, grid, "fleqn + maxSize(2) + strict(false)", "card", "\\text{café} + \\rule{10em}{0.4em} + é", .{
        .displayMode = true,
        .fleqn = true,
        .maxSize = 2.0,
        .strict = false,
    });

    _ = app.appendChild(grid);
}
