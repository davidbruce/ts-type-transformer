const dom = @import("dom");
const m = @import("marked");

var html_buf: [4096]u8 = undefined;

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

fn makePane(doc: dom.Document, title: []const u8, s: m.JsString) dom.HTMLElement {
    const html = s.copy(&html_buf);
    const card = makeEl(doc, "div", "card", null);
    appendAndRelease(card, makeEl(doc, "div", "label", title));
    const out = makeEl(doc, "div", "md-out", null);
    out.asElement().setInnerHTML(html);
    appendAndRelease(card, out);
    appendAndRelease(card, makeEl(doc, "pre", "html-raw", html));
    return card;
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Marked GFM & Line Breaks (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "MarkedOptions"));
    appendAndRelease(app, head);

    const gfm_src = "| Flag | Mode |\n|---|---|\n| gfm | table |\n\n~~deprecated~~\n";
    const gfm_on = m.marked_options_no_async.create();
    defer m.jsRelease(gfm_on);
    gfm_on.setGfm(true);
    const gfm_off = m.marked_options_no_async.create();
    defer m.jsRelease(gfm_off);
    gfm_off.setGfm(false);

    const gfm_on_html = m.marked(gfm_src, gfm_on);
    defer m.jsRelease(gfm_on_html);
    const gfm_off_html = m.marked(gfm_src, gfm_off);
    defer m.jsRelease(gfm_off_html);

    const gfm_sec = makeEl(doc, "div", "section", null);
    appendAndRelease(gfm_sec, makeEl(doc, "div", "label", "GitHub Flavored Markdown (gfm)"));
    const gfm_grid = makeEl(doc, "div", "grid", null);
    appendAndRelease(gfm_grid, makePane(doc, "gfm: true", gfm_on_html));
    appendAndRelease(gfm_grid, makePane(doc, "gfm: false", gfm_off_html));
    appendAndRelease(gfm_sec, gfm_grid);
    appendAndRelease(app, gfm_sec);

    const breaks_src = "Line one\nLine two\nLine three";
    const br_off = m.marked_options_no_async.create();
    defer m.jsRelease(br_off);
    br_off.setBreaks(false);
    const br_on = m.marked_options_no_async.create();
    defer m.jsRelease(br_on);
    br_on.setBreaks(true);

    const br_off_html = m.marked(breaks_src, br_off);
    defer m.jsRelease(br_off_html);
    const br_on_html = m.marked(breaks_src, br_on);
    defer m.jsRelease(br_on_html);

    const br_sec = makeEl(doc, "div", "section", null);
    appendAndRelease(br_sec, makeEl(doc, "div", "label", "Soft Line Breaks (breaks)"));
    const br_grid = makeEl(doc, "div", "grid", null);
    appendAndRelease(br_grid, makePane(doc, "breaks: false", br_off_html));
    appendAndRelease(br_grid, makePane(doc, "breaks: true", br_on_html));
    appendAndRelease(br_sec, br_grid);
    appendAndRelease(app, br_sec);
}
