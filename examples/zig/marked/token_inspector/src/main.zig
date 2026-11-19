const std = @import("std");
const dom = @import("dom");
const m = @import("marked");

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

var raw_buf: [512]u8 = undefined;
var snip_buf: [1024]u8 = undefined;

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";

    const tokens = m.Lexer.lex(sample, .{});
    defer m.jsRelease(tokens);
    const count: usize = @intFromFloat(tokens.getLength());

    var bbuf: [32]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "{d} tokens", .{count}) catch "tokens";

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Marked Token Inspector (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", badge_txt));
    appendAndRelease(app, head);

    const src_card = makeEl(doc, "div", "card", null);
    appendAndRelease(src_card, makeEl(doc, "div", "label", "Markdown Input"));
    appendAndRelease(src_card, makeEl(doc, "pre", "src", sample));
    appendAndRelease(app, src_card);

    const tbl_card = makeEl(doc, "div", "card", null);
    appendAndRelease(tbl_card, makeEl(doc, "div", "label", "Lexer Token Stream"));

    const table = makeEl(doc, "table", "tok-table", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "#", "Type", "Bytes", "Raw Snippet" }) |col| {
        appendAndRelease(hrow, makeEl(doc, "th", "", col));
    }
    appendAndRelease(thead, hrow);
    appendAndRelease(table, thead);

    const tbody = makeEl(doc, "tbody", "", null);
    for (0..count) |i| {
        const tv = tokens.getAt(@floatFromInt(i));
        defer m.jsRelease(tv);
        const g: m.tokens.Generic = .{ .h = tv.h };

        const kind = g.getType();
        defer m.jsRelease(kind);
        var kind_buf: [64]u8 = undefined;
        const kind_str = kind.copy(&kind_buf);

        const raw = g.getRaw();
        defer m.jsRelease(raw);
        const raw_str = raw.copy(&raw_buf);

        var sp: usize = 0;
        for (raw_str) |ch| {
            if (sp + 3 >= snip_buf.len) break;
            if (ch == '\n') {
                snip_buf[sp] = 0xE2;
                snip_buf[sp + 1] = 0x86;
                snip_buf[sp + 2] = 0xB5;
                sp += 3;
            } else {
                snip_buf[sp] = ch;
                sp += 1;
            }
        }

        var idx_buf: [16]u8 = undefined;
        var bytes_buf: [16]u8 = undefined;
        const idx_s = std.fmt.bufPrint(&idx_buf, "{d}", .{i}) catch "0";
        const bytes_s = std.fmt.bufPrint(&bytes_buf, "{d}", .{raw_str.len}) catch "0";

        const tr = makeEl(doc, "tr", "", null);
        appendAndRelease(tr, makeEl(doc, "td", "", idx_s));
        appendAndRelease(tr, makeEl(doc, "td", "", kind_str));
        appendAndRelease(tr, makeEl(doc, "td", "", bytes_s));
        appendAndRelease(tr, makeEl(doc, "td", "", snip_buf[0..sp]));
        appendAndRelease(tbody, tr);
    }

    appendAndRelease(table, tbody);
    appendAndRelease(tbl_card, table);
    appendAndRelease(app, tbl_card);
}
