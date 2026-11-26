const std = @import("std");
const dom = @import("dom");
const m = @import("marked");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const sample =
        "# Heading\n\nParagraph with **bold** and `code`.\n\n- alpha\n- beta\n\n> quote\n\n---\n";

    const tokens = m.Lexer.lex2(sample);
    const count: usize = @intFromFloat(tokens.getLength());

    var bbuf: [32]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "{d} tokens", .{count}) catch "tokens";

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Marked Token Inspector (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", badge_txt));
    _ = app.appendChild(head);

    const src_card = makeEl(doc, "div", "card", null);
    _ = src_card.appendChild(makeEl(doc, "div", "label", "Markdown Input"));
    _ = src_card.appendChild(makeEl(doc, "pre", "src", sample));
    _ = app.appendChild(src_card);

    const tbl_card = makeEl(doc, "div", "card", null);
    _ = tbl_card.appendChild(makeEl(doc, "div", "label", "Lexer Token Stream"));

    const table = makeEl(doc, "table", "tok-table", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "#", "Type", "Bytes", "Raw Snippet" }) |col| {
        _ = hrow.appendChild(makeEl(doc, "th", "", col));
    }
    _ = thead.appendChild(hrow);
    _ = table.appendChild(thead);

    const tbody = makeEl(doc, "tbody", "", null);
    var raw_buf: [512]u8 = undefined;
    var snip_buf: [1024]u8 = undefined;
    var kind_buf: [64]u8 = undefined;
    var idx_buf: [16]u8 = undefined;
    var bytes_buf: [16]u8 = undefined;

    for (0..count) |i| {
        const tok = tokens.getAt(i).?;
        const kind_str = switch (tok) {
            .Tokens_Generic => |g| g.getType().copy(&kind_buf),
            inline else => |x| @TypeOf(x).type,
        };
        const raw_str = switch (tok) {
            inline else => |x| x.getRaw().copy(&raw_buf),
        };

        var sp: usize = 0;
        for (raw_str) |ch| {
            if (sp + 3 >= snip_buf.len) break;
            if (ch == '\n') {
                snip_buf[sp..][0..3].* = "\xE2\x86\xB5".*;
                sp += 3;
            } else {
                snip_buf[sp] = ch;
                sp += 1;
            }
        }

        const idx_s = std.fmt.bufPrint(&idx_buf, "{d}", .{i}) catch "0";
        const bytes_s = std.fmt.bufPrint(&bytes_buf, "{d}", .{raw_str.len}) catch "0";

        const tr = makeEl(doc, "tr", "", null);
        _ = tr.appendChild(makeEl(doc, "td", "", idx_s));
        _ = tr.appendChild(makeEl(doc, "td", "", kind_str));
        _ = tr.appendChild(makeEl(doc, "td", "", bytes_s));
        _ = tr.appendChild(makeEl(doc, "td", "", snip_buf[0..sp]));
        _ = tbody.appendChild(tr);
    }

    _ = table.appendChild(tbody);
    _ = tbl_card.appendChild(table);
    _ = app.appendChild(tbl_card);
}
