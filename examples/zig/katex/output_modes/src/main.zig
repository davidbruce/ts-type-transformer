const std = @import("std");
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

fn addCard(doc: dom.Document, grid: dom.HTMLElement, label: []const u8, cls: []const u8, tex: []const u8, opts: katex.KatexOptions) void {
    const html = katex.renderToString(tex, .{ .options = opts });
    defer katex.jsRelease(html);
    var lbuf: [32]u8 = undefined;
    const len_txt = std.fmt.bufPrint(&lbuf, "{d} chars", .{html.len()}) catch "0 chars";

    const card = makeEl(doc, "div", cls, null);
    const row = makeEl(doc, "div", "head", null);
    appendAndRelease(row, makeEl(doc, "div", "label", label));
    appendAndRelease(row, makeEl(doc, "span", "badge", len_txt));
    appendAndRelease(card, row);
    const slot = makeEl(doc, "div", "slot", null);
    slot.asElement().setInnerHTML(html.copy(&html_buf));
    appendAndRelease(card, slot);
    appendAndRelease(grid, card);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Output & Layout Options (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "html | mathml | leqno | fleqn | maxSize | strict"));
    appendAndRelease(app, head);

    const grid = makeEl(doc, "div", "grid", null);

    const o_html = katex.KatexOptions.create();
    defer katex.jsRelease(o_html);
    o_html.setOutput(.html);
    addCard(doc, grid, "output: html", "card mode-html", "a^2 + b^2 = c^2", o_html);

    const o_math = katex.KatexOptions.create();
    defer katex.jsRelease(o_math);
    o_math.setOutput(.mathml);
    addCard(doc, grid, "output: mathml", "card", "\\frac{1}{1 + x^2}", o_math);

    const o_both = katex.KatexOptions.create();
    defer katex.jsRelease(o_both);
    o_both.setOutput(.htmlAndMathml);
    addCard(doc, grid, "output: htmlAndMathml", "card", "\\sqrt{\\alpha + \\beta}", o_both);

    const o_leq = katex.KatexOptions.create();
    defer katex.jsRelease(o_leq);
    o_leq.setDisplayMode(true);
    o_leq.setLeqno(true);
    addCard(doc, grid, "displayMode + leqno", "card", "\\int_0^1 x^2\\,dx = \\frac{1}{3} \\tag{1}", o_leq);

    const o_fl = katex.KatexOptions.create();
    defer katex.jsRelease(o_fl);
    o_fl.setDisplayMode(true);
    o_fl.setFleqn(true);
    o_fl.setMaxSize(2.0);
    o_fl.setStrict(.{ .boolean = false });
    addCard(doc, grid, "fleqn + maxSize(2) + strict(false)", "card", "\\text{café} + \\rule{10em}{0.4em} + é", o_fl);

    appendAndRelease(app, grid);
}
