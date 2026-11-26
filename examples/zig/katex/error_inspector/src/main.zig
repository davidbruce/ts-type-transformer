const std = @import("std");
const dom = @import("dom");
const katex = @import("katex");

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

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "ParseError Inspector (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "throwOnError: false vs true"));
    _ = app.appendChild(head);

    const lenient = katex.KatexOptions.init(.{
        .throwOnError = false,
        .errorColor = "#555555",
    });
    const strict = katex.KatexOptions.init(.{
        .throwOnError = true,
    });

    const bad_inputs = [_][]const u8{
        "\\frac{1}{",
        "\\unknowncmd{x}",
        "x^{2 + ",
        "\\sqrt{a + b",
    };

    var err_buf: [1024]u8 = undefined;
    for (bad_inputs) |tex| {
        const row = makeEl(doc, "div", "row", null);

        const left = makeEl(doc, "div", "cell mode-html", null);
        _ = left.appendChild(makeEl(doc, "div", "label", "throwOnError: false (#555555)"));
        const l_slot = makeEl(doc, "div", "slot", null);
        l_slot.setInnerHTML(katex.renderToStringWith(tex, lenient));
        _ = left.appendChild(l_slot);
        _ = row.appendChild(left);

        const right = makeEl(doc, "div", "cell", null);
        _ = right.appendChild(makeEl(doc, "div", "label", "throwOnError: true (caught ParseError)"));
        var err_text: []const u8 = "(did not throw)";
        if (katex.jsTry(katex.renderToStringWith(tex, strict))) |_| {} else |_| {
            if (katex.jsTakeError()) |err| {
                if (katex.ParseError.cast(err)) |pe| {
                    err_text = std.fmt.bufPrint(&err_buf, "{f}", .{pe.getMessage()}) catch "(error)";
                }
            }
        }
        _ = right.appendChild(makeEl(doc, "div", "err", err_text));
        _ = row.appendChild(right);

        _ = app.appendChild(row);
    }
}
