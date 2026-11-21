const std = @import("std");
const dom = @import("dom");
const m = @import("marked");

const WalkState = struct {
    types: [64][32]u8 = undefined,
    lens: [64]usize = undefined,
    count: usize = 0,
};

fn walkToken(state: *WalkState, tok: m.Token) ?m.void_or_void_or_Promise_array_or_Promise {
    if (state.count < state.types.len) {
        const s = switch (tok) {
            .Tokens_Generic => |g| g.getType().copy(&state.types[state.count]),
            inline else => |x| blk: {
                const name = @TypeOf(x).type;
                @memcpy(state.types[state.count][0..name.len], name);
                break :blk state.types[state.count][0..name.len];
            },
        };
        state.lens[state.count] = s.len;
        state.count += 1;
    }
    return null;
}

fn renderHeading(args: m.JsTuple(&.{m.tokens.Heading})) m.JsValue {
    const h = args.get(0);
    const depth: u32 = @intFromFloat(h.getDepth());
    var out_buf: [256]u8 = undefined;
    const out = std.fmt.bufPrint(
        &out_buf,
        "<h{d} class=\"wire-h\">\xC2\xA7 {f}</h{d}>\n",
        .{ depth, h.getText(), depth },
    ) catch "";
    return m.JsValue.string(out);
}

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

    const md =
        "# Wireframe Spec\n\n## Custom Pipeline\n\nParagraph with *italic* and **bold** and `code`.\n";

    var walk: WalkState = .{};
    const ext = m.MarkedExtension.init(.{
        .renderer = .{ .heading = renderHeading },
        .walkTokens = .{ walkToken, &walk },
    });

    const custom_m = m.Marked.new(.{ext});
    const def_html = m.marked(md, .{});
    const cust_str = custom_m.getParse().invoke2(md, null).as(m.JsString);

    var bbuf: [48]u8 = undefined;
    const badge_txt = std.fmt.bufPrint(&bbuf, "{d} tokens visited", .{walk.count}) catch "visited";

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Marked Custom Renderer & Walker (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", badge_txt));
    _ = app.appendChild(head);

    const grid = makeEl(doc, "div", "grid", null);

    const def_card = makeEl(doc, "div", "card", null);
    _ = def_card.appendChild(makeEl(doc, "div", "label", "Default Renderer"));
    const def_out = makeEl(doc, "div", "md-out", null);
    def_out.setInnerHTML(def_html);
    _ = def_card.appendChild(def_out);

    const cust_card = makeEl(doc, "div", "card", null);
    _ = cust_card.appendChild(makeEl(doc, "div", "label", "Custom Heading Renderer"));
    const cust_out = makeEl(doc, "div", "md-out", null);
    cust_out.setInnerHTML(cust_str);
    _ = cust_card.appendChild(cust_out);

    _ = grid.appendChild(def_card);
    _ = grid.appendChild(cust_card);
    _ = app.appendChild(grid);

    const walk_card = makeEl(doc, "div", "card", null);
    _ = walk_card.appendChild(makeEl(doc, "div", "label", "Visited Tokens (walkTokens)"));
    const strip = makeEl(doc, "div", "tok-strip", null);
    for (0..walk.count) |i| {
        _ = strip.appendChild(makeEl(doc, "span", "tok-pill", walk.types[i][0..walk.lens[i]]));
    }
    _ = walk_card.appendChild(strip);
    _ = app.appendChild(walk_card);
}
