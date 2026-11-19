const std = @import("std");
const dom = @import("dom");
const m = @import("marked");

var walked_types: [64][32]u8 = undefined;
var walked_lens: [64]usize = undefined;
var walked_count: usize = 0;

fn walkToken(_: ?*anyopaque, tok: m.JsValue) ?m.void_or_void_or_Promise_array_or_Promise {
    if (walked_count < 64) {
        const g: m.tokens.Generic = .{ .h = tok.h };
        const ty = g.getType();
        defer m.jsRelease(ty);
        const s = ty.copy(&walked_types[walked_count]);
        walked_lens[walked_count] = s.len;
        walked_count += 1;
    }
    return null;
}

fn renderHeading(_: ?*anyopaque, args: m.JsTuple(&.{m.tokens.Heading})) m.JsValue {
    const h = args.get(0);
    defer m.jsRelease(h);
    const depth: u32 = @intFromFloat(h.getDepth());
    const text = h.getText();
    defer m.jsRelease(text);
    var title_buf: [128]u8 = undefined;
    var out_buf: [256]u8 = undefined;
    const out = std.fmt.bufPrint(
        &out_buf,
        "<h{d} class=\"wire-h\">\xC2\xA7 {s}</h{d}>\n",
        .{ depth, text.copy(&title_buf), depth },
    ) catch "";
    return m.JsValue.string(out);
}

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

var html_buf: [4096]u8 = undefined;

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const md =
        "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n";

    const ext = m.MarkedExtension.create();
    defer m.jsRelease(ext);
    const renderer = m.MarkedExtension_renderer.create();
    defer m.jsRelease(renderer);
    renderer.setHeading(m.MarkedExtension_renderer_heading.from(renderHeading, null));
    ext.setRenderer(renderer);
    ext.setWalkTokens(m.Token_to_void_or_void_or_Promise_array_or_Promise_fn.from(walkToken, null));

    const exts = m.JsArrayOf(m.MarkedExtension).of(&.{ext});
    defer m.jsRelease(exts);
    const custom_m = m.Marked.new(exts);
    defer m.jsRelease(custom_m);

    const none = m.marked_options_no_async.create();
    defer m.jsRelease(none);
    const def_html = m.marked(md, none);
    defer m.jsRelease(def_html);
    const cust_val = custom_m.getParse().invoke2(md, null);
    defer m.jsRelease(cust_val);
    const cust_str: m.JsString = .{ .h = cust_val.h };

    var bbuf: [48]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "{d} tokens visited", .{walked_count}) catch "visited";

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Marked Custom Renderer & Walker (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", badge_txt));
    appendAndRelease(app, head);

    const grid = makeEl(doc, "div", "grid", null);

    const def_card = makeEl(doc, "div", "card", null);
    appendAndRelease(def_card, makeEl(doc, "div", "label", "Default Renderer"));
    const def_out = makeEl(doc, "div", "md-out", null);
    def_out.asElement().setInnerHTML(def_html.copy(&html_buf));
    appendAndRelease(def_card, def_out);

    const cust_card = makeEl(doc, "div", "card", null);
    appendAndRelease(cust_card, makeEl(doc, "div", "label", "Custom Heading Renderer"));
    const cust_out = makeEl(doc, "div", "md-out", null);
    cust_out.asElement().setInnerHTML(cust_str.copy(&html_buf));
    appendAndRelease(cust_card, cust_out);

    appendAndRelease(grid, def_card);
    appendAndRelease(grid, cust_card);
    appendAndRelease(app, grid);

    const walk_card = makeEl(doc, "div", "card", null);
    appendAndRelease(walk_card, makeEl(doc, "div", "label", "Visited Tokens (walkTokens)"));
    const strip = makeEl(doc, "div", "tok-strip", null);
    for (0..walked_count) |i| {
        appendAndRelease(strip, makeEl(doc, "span", "tok-pill", walked_types[i][0..walked_lens[i]]));
    }
    appendAndRelease(walk_card, strip);
    appendAndRelease(app, walk_card);
}
