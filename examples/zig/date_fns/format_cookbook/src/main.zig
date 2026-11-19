const dom = @import("dom");
const df = @import("date_fns");

var str_buf: [256]u8 = undefined;

fn day(d: df.Date) df.DateArg {
    return .{ .Date = d };
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

fn addFmtRow(doc: dom.Document, tbody: dom.HTMLElement, api: []const u8, out: df.JsString, note: []const u8) void {
    defer df.jsRelease(out);
    const tr = makeEl(doc, "tr", "", null);
    const td_api = makeEl(doc, "td", "", null);
    appendAndRelease(td_api, makeEl(doc, "code", "", api));
    appendAndRelease(tr, td_api);
    appendAndRelease(tr, makeEl(doc, "td", "", out.copy(&str_buf)));
    appendAndRelease(tr, makeEl(doc, "td", "", note));
    appendAndRelease(tbody, tr);
}

fn addValRow(doc: dom.Document, tbody: dom.HTMLElement, input: []const u8, check: []const u8, ok: bool, pill: []const u8) void {
    const tr = makeEl(doc, "tr", "", null);
    const td_in = makeEl(doc, "td", "", null);
    appendAndRelease(td_in, makeEl(doc, "code", "", input));
    appendAndRelease(tr, td_in);
    appendAndRelease(tr, makeEl(doc, "td", "", check));
    const td_res = makeEl(doc, "td", "", null);
    appendAndRelease(td_res, makeEl(doc, "span", if (ok) "pill" else "pill-muted", pill));
    appendAndRelease(tr, td_res);
    appendAndRelease(tbody, tr);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Date-fns Format & Parse Cookbook (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "2024-03-15T14:30:45"));
    appendAndRelease(app, head);

    const sample = df.parseISO("2024-03-15T14:30:45", .{});
    defer df.jsRelease(sample);
    const parsed = df.parse("15/03/2024", "dd/MM/yyyy", day(sample), .{});
    defer df.jsRelease(parsed);

    const fmt_card = makeEl(doc, "div", "card", null);
    appendAndRelease(fmt_card, makeEl(doc, "div", "label", "Format Tokens & ISO Output"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "API / Pattern", "Output", "Notes" }) |h| {
        appendAndRelease(hrow, makeEl(doc, "th", "", h));
    }
    appendAndRelease(thead, hrow);
    appendAndRelease(tbl, thead);

    const tbody = makeEl(doc, "tbody", "", null);
    addFmtRow(doc, tbody, "formatDate(\"yyyy-MM-dd\")", df.formatDate(day(sample), "yyyy-MM-dd", .{}), "ISO date");
    addFmtRow(doc, tbody, "formatDate(\"EEEE, do MMMM yyyy\")", df.formatDate(day(sample), "EEEE, do MMMM yyyy", .{}), "Ordinal full date");
    addFmtRow(doc, tbody, "formatDate(\"EEE, d MMM yy\")", df.formatDate(day(sample), "EEE, d MMM yy", .{}), "Compact short date");
    addFmtRow(doc, tbody, "formatDate(\"HH:mm:ss\")", df.formatDate(day(sample), "HH:mm:ss", .{}), "24-hour clock");
    addFmtRow(doc, tbody, "formatDate(\"h:mm a\")", df.formatDate(day(sample), "h:mm a", .{}), "12-hour clock");
    addFmtRow(doc, tbody, "formatDate(\"'Q'Q \xC2\xB7 'W'I\")", df.formatDate(day(sample), "'Q'Q \xC2\xB7 'W'I", .{}), "Quarter & ISO week");
    addFmtRow(doc, tbody, "formatDate(\"DDD 'of' yyyy\")", df.formatDate(day(sample), "DDD 'of' yyyy", .{}), "Day of year");
    addFmtRow(doc, tbody, "lightFormat(\"yyyy/MM/dd HH:mm\")", df.lightFormat(day(sample), "yyyy/MM/dd HH:mm"), "Zero-locale light formatter");
    addFmtRow(doc, tbody, "formatISO(sample)", df.formatISO(day(sample), .{}), "ISO 8601 timestamp");
    addFmtRow(doc, tbody, "parse(\"15/03/2024\", \"dd/MM/yyyy\")", df.formatDate(day(parsed), "yyyy-MM-dd", .{}), "Custom parse roundtrip");
    appendAndRelease(tbl, tbody);
    appendAndRelease(fmt_card, tbl);
    appendAndRelease(app, fmt_card);

    const val_card = makeEl(doc, "div", "card", null);
    appendAndRelease(val_card, makeEl(doc, "div", "label", "Validation & Calendar Existence"));

    const vtbl = makeEl(doc, "table", "", null);
    const vhead = makeEl(doc, "thead", "", null);
    const vhrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Input", "Check", "Result" }) |h| {
        appendAndRelease(vhrow, makeEl(doc, "th", "", h));
    }
    appendAndRelease(vhead, vhrow);
    appendAndRelease(vtbl, vhead);

    const vbody = makeEl(doc, "tbody", "", null);
    const good_iso = df.parseISO("2024-03-15", .{});
    defer df.jsRelease(good_iso);
    const bad_iso = df.parseISO("not-a-date", .{});
    defer df.jsRelease(bad_iso);
    const ok_good = df.isValid(.{ .h = good_iso.h }) and df.isMatch("2024-03-15", "yyyy-MM-dd", .{});
    const ok_bad = df.isValid(.{ .h = bad_iso.h }) and df.isMatch("not-a-date", "yyyy-MM-dd", .{});
    const ok_2024 = df.isExists(2024, 1, 29);
    const ok_2023 = df.isExists(2023, 1, 29);

    addValRow(doc, vbody, "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, if (ok_good) "VALID" else "INVALID");
    addValRow(doc, vbody, "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, if (ok_bad) "VALID" else "INVALID");
    addValRow(doc, vbody, "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, if (ok_2024) "EXISTS" else "INVALID");
    addValRow(doc, vbody, "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, if (ok_2023) "EXISTS" else "INVALID");
    appendAndRelease(vtbl, vbody);
    appendAndRelease(val_card, vtbl);
    appendAndRelease(app, val_card);
}
