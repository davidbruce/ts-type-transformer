const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: ?[]const u8) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (text) |t| el.setTextContent(t);
    return el;
}

fn fmtSigned(v: i32, buf: []u8) []const u8 {
    return if (v > 0)
        std.fmt.bufPrint(buf, "+{d}", .{v}) catch ""
    else
        std.fmt.bufPrint(buf, "{d}", .{v}) catch "";
}

fn addOpRow(doc: dom.Document, tbody: dom.HTMLElement, name: []const u8, res: df.Date, base_d: df.Date, fmt: []const u8) void {
    var out_buf: [128]u8 = undefined;
    var dd_buf: [24]u8 = undefined;
    var dh_buf: [24]u8 = undefined;
    const dd: i32 = @intFromFloat(df.differenceInCalendarDays(res, base_d, null));
    const dh: i32 = @intFromFloat(df.differenceInHours(res, base_d, null));

    const tr = makeEl(doc, "tr", "", null);
    const td_op = makeEl(doc, "td", "", null);
    _ = td_op.appendChild(makeEl(doc, "code", "", name));
    _ = tr.appendChild(td_op);
    _ = tr.appendChild(makeEl(doc, "td", "", df.formatDate(res, fmt, null).copy(&out_buf)));
    _ = tr.appendChild(makeEl(doc, "td", "", fmtSigned(dd, &dd_buf)));
    _ = tr.appendChild(makeEl(doc, "td", "", fmtSigned(dh, &dh_buf)));
    _ = tbody.appendChild(tr);
}

fn addKv(doc: dom.Document, row: dom.HTMLElement, label: []const u8, val: []const u8) void {
    const box = makeEl(doc, "div", "kv", null);
    _ = box.appendChild(makeEl(doc, "span", "label", label));
    _ = box.appendChild(makeEl(doc, "strong", "", val));
    _ = row.appendChild(box);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Date-fns Arithmetic & Durations (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "Base 2024-03-15"));
    _ = app.appendChild(head);

    const base_d = df.parseISO("2024-03-15", null);
    const jan31 = df.parseISO("2024-01-31", null);
    const dur = df.Duration.init(.{ .months = 1, .days = 2 });

    const card = makeEl(doc, "div", "card", null);
    _ = card.appendChild(makeEl(doc, "div", "label", "Immutable Date Stepping"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Operation", "Result", "\xCE\x94 Days (vs base)", "\xCE\x94 Hours" }) |h| {
        _ = hrow.appendChild(makeEl(doc, "th", "", h));
    }
    _ = thead.appendChild(hrow);
    _ = tbl.appendChild(thead);

    const tbody = makeEl(doc, "tbody", "", null);
    addOpRow(doc, tbody, "addDays(base, 3)", df.addDays(base_d, 3, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "subDays(base, 15)", df.subDays(base_d, 15, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "addWeeks(base, 2)", df.addWeeks(base_d, 2, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "addMonths(jan31, 1)", df.addMonths(jan31, 1, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "subMonths(base, 3)", df.subMonths(base_d, 3, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "addYears(base, 1)", df.addYears(base_d, 1, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "startOfWeek(base)", df.startOfWeek(base_d, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "endOfMonth(base)", df.endOfMonth(base_d, null), base_d, "yyyy-MM-dd HH:mm:ss");
    addOpRow(doc, tbody, "add(base, { months: 1, days: 2 })", df.add(base_d, dur, null), base_d, "yyyy-MM-dd");
    addOpRow(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd");
    _ = tbl.appendChild(tbody);
    _ = card.appendChild(tbl);
    _ = app.appendChild(card);

    const footer = makeEl(doc, "div", "card", null);
    _ = footer.appendChild(makeEl(doc, "div", "label", "Duration & Unit Conversions"));
    const srow = makeEl(doc, "div", "summary-row", null);

    var tmp_buf: [128]u8 = undefined;
    addKv(doc, srow, "formatDuration(dur)", df.formatDuration(dur, null).copy(&tmp_buf));
    addKv(doc, srow, "formatISODuration(dur)", df.formatISODuration(dur).copy(&tmp_buf));
    addKv(doc, srow, "hoursToMinutes(2)", std.fmt.bufPrint(&tmp_buf, "{d} min", .{@as(i32, @intFromFloat(df.hoursToMinutes(2)))}) catch "");
    addKv(doc, srow, "daysToWeeks(14)", std.fmt.bufPrint(&tmp_buf, "{d} wk", .{@as(i32, @intFromFloat(df.daysToWeeks(14)))}) catch "");
    _ = footer.appendChild(srow);
    _ = app.appendChild(footer);
}
