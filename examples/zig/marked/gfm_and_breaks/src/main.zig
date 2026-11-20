const dom = @import("dom");
const m = @import("marked");

var html_buf: [4096]u8 = undefined;

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

fn makePane(doc: dom.Document, title: []const u8, s: m.JsString) dom.HTMLElement {
    const html = s.copy(&html_buf);
    const card = makeEl(doc, "div", "card", null);
    _ = card.appendChild(makeEl(doc, "div", "label", title));
    const out = makeEl(doc, "div", "md-out", null);
    out.setInnerHTML(html);
    _ = card.appendChild(out);
    _ = card.appendChild(makeEl(doc, "pre", "html-raw", html));
    return card;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Marked GFM & Line Breaks (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "MarkedOptions"));
    _ = app.appendChild(head);

    const gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    const gfm_sec = makeEl(doc, "div", "section", null);
    _ = gfm_sec.appendChild(makeEl(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    const gfm_grid = makeEl(doc, "div", "grid", null);
    _ = gfm_grid.appendChild(makePane(doc, "gfm: true", m.marked(gfm_src, .{ .gfm = true })));
    _ = gfm_grid.appendChild(makePane(doc, "gfm: false", m.marked(gfm_src, .{ .gfm = false })));
    _ = gfm_sec.appendChild(gfm_grid);
    _ = app.appendChild(gfm_sec);

    const breaks_src = "Line one\nLine two\nLine three";
    const br_sec = makeEl(doc, "div", "section", null);
    _ = br_sec.appendChild(makeEl(doc, "div", "label", "Soft Line Breaks (breaks)"));
    const br_grid = makeEl(doc, "div", "grid", null);
    _ = br_grid.appendChild(makePane(doc, "breaks: false", m.marked(breaks_src, .{ .breaks = false })));
    _ = br_grid.appendChild(makePane(doc, "breaks: true", m.marked(breaks_src, .{ .breaks = true })));
    _ = br_sec.appendChild(br_grid);
    _ = app.appendChild(br_sec);
}
