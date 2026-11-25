const dom = @import("dom");
const m = @import("marked");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: anytype) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (@TypeOf(text) != @TypeOf(null)) el.setTextContent(text);
    return el;
}

fn makePane(doc: dom.Document, title: []const u8, html: m.JsString) dom.HTMLElement {
    const card = makeEl(doc, "div", "card", null);
    _ = card.appendChild(makeEl(doc, "div", "label", title));
    const out = makeEl(doc, "div", "md-out", null);
    out.setInnerHTML(html);
    _ = card.appendChild(out);
    _ = card.appendChild(makeEl(doc, "pre", "html-raw", html));
    return card;
}

// An instance with these options, set with use.
fn withOptions(opts: m.MarkedExtension) m.Marked {
    const inst = m.Marked.new(.{});
    _ = inst.use(.{opts});
    return inst;
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
    _ = gfm_grid.appendChild(makePane(doc, "gfm: true", withOptions(m.MarkedExtension.init(.{ .gfm = true })).parse(gfm_src)));
    _ = gfm_grid.appendChild(makePane(doc, "gfm: false", withOptions(m.MarkedExtension.init(.{ .gfm = false })).parse(gfm_src)));
    _ = gfm_sec.appendChild(gfm_grid);
    _ = app.appendChild(gfm_sec);

    const breaks_src = "Line one\nLine two\nLine three";
    const br_sec = makeEl(doc, "div", "section", null);
    _ = br_sec.appendChild(makeEl(doc, "div", "label", "Soft Line Breaks (breaks)"));
    const br_grid = makeEl(doc, "div", "grid", null);
    _ = br_grid.appendChild(makePane(doc, "breaks: false", withOptions(m.MarkedExtension.init(.{ .breaks = false })).parse(breaks_src)));
    _ = br_grid.appendChild(makePane(doc, "breaks: true", withOptions(m.MarkedExtension.init(.{ .breaks = true })).parse(breaks_src)));
    _ = br_sec.appendChild(br_grid);
    _ = app.appendChild(br_sec);
}
