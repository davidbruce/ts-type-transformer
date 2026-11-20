const dom = @import("dom");
const df = @import("date_fns");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

fn addFmtRow(doc: dom.Document, tbody: dom.HTMLElement, api: []const u8, out: df.JsString, note: []const u8) void {
    var str_buf: [256]u8 = undefined;
    const tr = makeEl(doc, "tr", "", null);
    const td_api = makeEl(doc, "td", "", null);
    _ = td_api.appendChild(makeEl(doc, "code", "", api));
    _ = tr.appendChild(td_api);
    _ = tr.appendChild(makeEl(doc, "td", "", out.copy(&str_buf)));
    _ = tr.appendChild(makeEl(doc, "td", "", note));
    _ = tbody.appendChild(tr);
}

fn addValRow(doc: dom.Document, tbody: dom.HTMLElement, input: []const u8, check: []const u8, ok: bool, pill: []const u8) void {
    const tr = makeEl(doc, "tr", "", null);
    const td_in = makeEl(doc, "td", "", null);
    _ = td_in.appendChild(makeEl(doc, "code", "", input));
    _ = tr.appendChild(td_in);
    _ = tr.appendChild(makeEl(doc, "td", "", check));
    const td_res = makeEl(doc, "td", "", null);
    _ = td_res.appendChild(makeEl(doc, "span", if (ok) "pill" else "pill-muted", pill));
    _ = tr.appendChild(td_res);
    _ = tbody.appendChild(tr);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Date-fns Format & Parse Cookbook (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "2024-03-15T14:30:45"));
    _ = app.appendChild(head);

    const sample = df.parseISO("2024-03-15T14:30:45", null);
    const parsed = df.parse("15/03/2024", "dd/MM/yyyy", sample, null);

    const fmt_card = makeEl(doc, "div", "card", null);
    _ = fmt_card.appendChild(makeEl(doc, "div", "label", "Format Tokens & ISO Output"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "API / Pattern", "Output", "Notes" }) |h| {
        _ = hrow.appendChild(makeEl(doc, "th", "", h));
    }
    _ = thead.appendChild(hrow);
    _ = tbl.appendChild(thead);

    const tbody = makeEl(doc, "tbody", "", null);
    addFmtRow(doc, tbody, "formatDate(\"yyyy-MM-dd\")", df.formatDate(sample, "yyyy-MM-dd", null), "ISO date");
    addFmtRow(doc, tbody, "formatDate(\"EEEE, do MMMM yyyy\")", df.formatDate(sample, "EEEE, do MMMM yyyy", null), "Ordinal full date");
    addFmtRow(doc, tbody, "formatDate(\"EEE, d MMM yy\")", df.formatDate(sample, "EEE, d MMM yy", null), "Compact short date");
    addFmtRow(doc, tbody, "formatDate(\"HH:mm:ss\")", df.formatDate(sample, "HH:mm:ss", null), "24-hour clock");
    addFmtRow(doc, tbody, "formatDate(\"h:mm a\")", df.formatDate(sample, "h:mm a", null), "12-hour clock");
    addFmtRow(doc, tbody, "formatDate(\"'Q'Q \xC2\xB7 'W'I\")", df.formatDate(sample, "'Q'Q \xC2\xB7 'W'I", null), "Quarter & ISO week");
    addFmtRow(doc, tbody, "formatDate(\"DDD 'of' yyyy\")", df.formatDate(sample, "DDD 'of' yyyy", null), "Day of year");
    addFmtRow(doc, tbody, "lightFormat(\"yyyy/MM/dd HH:mm\")", df.lightFormat(sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter");
    addFmtRow(doc, tbody, "formatISO(sample)", df.formatISO(sample, null), "ISO 8601 timestamp");
    addFmtRow(doc, tbody, "parse(\"15/03/2024\", \"dd/MM/yyyy\")", df.formatDate(parsed, "yyyy-MM-dd", null), "Custom parse roundtrip");
    _ = tbl.appendChild(tbody);
    _ = fmt_card.appendChild(tbl);
    _ = app.appendChild(fmt_card);

    const val_card = makeEl(doc, "div", "card", null);
    _ = val_card.appendChild(makeEl(doc, "div", "label", "Validation & Calendar Existence"));

    const vtbl = makeEl(doc, "table", "", null);
    const vhead = makeEl(doc, "thead", "", null);
    const vhrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Input", "Check", "Result" }) |h| {
        _ = vhrow.appendChild(makeEl(doc, "th", "", h));
    }
    _ = vhead.appendChild(vhrow);
    _ = vtbl.appendChild(vhead);

    const vbody = makeEl(doc, "tbody", "", null);
    const good_iso = df.parseISO("2024-03-15", null);
    const bad_iso = df.parseISO("not-a-date", null);
    const ok_good = df.isValid(good_iso) and df.isMatch("2024-03-15", "yyyy-MM-dd", null);
    const ok_bad = df.isValid(bad_iso) and df.isMatch("not-a-date", "yyyy-MM-dd", null);
    const ok_2024 = df.isExists(2024, 1, 29);
    const ok_2023 = df.isExists(2023, 1, 29);

    addValRow(doc, vbody, "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_good, if (ok_good) "VALID" else "INVALID");
    addValRow(doc, vbody, "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", ok_bad, if (ok_bad) "VALID" else "INVALID");
    addValRow(doc, vbody, "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", ok_2024, if (ok_2024) "EXISTS" else "INVALID");
    addValRow(doc, vbody, "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", ok_2023, if (ok_2023) "EXISTS" else "INVALID");
    _ = vtbl.appendChild(vbody);
    _ = val_card.appendChild(vtbl);
    _ = app.appendChild(val_card);
}
