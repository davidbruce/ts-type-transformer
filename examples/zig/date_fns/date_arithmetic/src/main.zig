const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

var tmp_buf: [128]u8 = undefined;

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

fn fmtSigned(v: i32, buf: []u8) []const u8 {
    return if (v > 0)
        std.fmt.bufPrint(buf, "+{d}", .{v}) catch ""
    else
        std.fmt.bufPrint(buf, "{d}", .{v}) catch "";
}

fn addOpRow(doc: dom.Document, tbody: dom.HTMLElement, name: []const u8, res: df.Date, base_d: df.Date, fmt: []const u8, release_res: bool) void {
    defer if (release_res) df.jsRelease(res);
    const out = df.formatDate(day(res), fmt, .{});
    defer df.jsRelease(out);

    const dd: i32 = @intFromFloat(df.differenceInCalendarDays(day(res), day(base_d), .{}));
    const dh: i32 = @intFromFloat(df.differenceInHours(day(res), day(base_d), .{}));
    var dd_buf: [24]u8 = undefined;
    var dh_buf: [24]u8 = undefined;

    const tr = makeEl(doc, "tr", "", null);
    const td_op = makeEl(doc, "td", "", null);
    appendAndRelease(td_op, makeEl(doc, "code", "", name));
    appendAndRelease(tr, td_op);
    appendAndRelease(tr, makeEl(doc, "td", "", out.copy(&tmp_buf)));
    appendAndRelease(tr, makeEl(doc, "td", "", fmtSigned(dd, &dd_buf)));
    appendAndRelease(tr, makeEl(doc, "td", "", fmtSigned(dh, &dh_buf)));
    appendAndRelease(tbody, tr);
}

fn addKv(doc: dom.Document, row: dom.HTMLElement, label: []const u8, val: []const u8) void {
    const box = makeEl(doc, "div", "kv", null);
    appendAndRelease(box, makeEl(doc, "span", "label", label));
    appendAndRelease(box, makeEl(doc, "strong", "", val));
    appendAndRelease(row, box);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Date-fns Arithmetic & Durations (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "Base 2024-03-15"));
    appendAndRelease(app, head);

    const base_d = df.parseISO("2024-03-15", .{});
    defer df.jsRelease(base_d);
    const jan31 = df.parseISO("2024-01-31", .{});
    defer df.jsRelease(jan31);
    const dur = df.Duration.create();
    defer df.jsRelease(dur);
    dur.setMonths(1);
    dur.setDays(2);

    const card = makeEl(doc, "div", "card", null);
    appendAndRelease(card, makeEl(doc, "div", "label", "Immutable Date Stepping"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Operation", "Result", "\xCE\x94 Days (vs base)", "\xCE\x94 Hours" }) |h| {
        appendAndRelease(hrow, makeEl(doc, "th", "", h));
    }
    appendAndRelease(thead, hrow);
    appendAndRelease(tbl, thead);

    const tbody = makeEl(doc, "tbody", "", null);
    addOpRow(doc, tbody, "addDays(base, 3)", df.addDays(day(base_d), 3, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "subDays(base, 15)", df.subDays(day(base_d), 15, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "addWeeks(base, 2)", df.addWeeks(day(base_d), 2, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "addMonths(jan31, 1)", df.addMonths(day(jan31), 1, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "subMonths(base, 3)", df.subMonths(day(base_d), 3, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "addYears(base, 1)", df.addYears(day(base_d), 1, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "startOfWeek(base)", df.startOfWeek(day(base_d), .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "endOfMonth(base)", df.endOfMonth(day(base_d), .{}), base_d, "yyyy-MM-dd HH:mm:ss", true);
    addOpRow(doc, tbody, "add(base, { months: 1, days: 2 })", df.add(day(base_d), dur, .{}), base_d, "yyyy-MM-dd", true);
    addOpRow(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd", false);
    appendAndRelease(tbl, tbody);
    appendAndRelease(card, tbl);
    appendAndRelease(app, card);

    const footer = makeEl(doc, "div", "card", null);
    appendAndRelease(footer, makeEl(doc, "div", "label", "Duration & Unit Conversions"));
    const srow = makeEl(doc, "div", "summary-row", null);

    const d_words = df.formatDuration(dur, .{});
    defer df.jsRelease(d_words);
    addKv(doc, srow, "formatDuration(dur)", d_words.copy(&tmp_buf));

    const d_iso = df.formatISODuration(dur);
    defer df.jsRelease(d_iso);
    addKv(doc, srow, "formatISODuration(dur)", d_iso.copy(&tmp_buf));

    addKv(doc, srow, "hoursToMinutes(2)", std.fmt.bufPrint(&tmp_buf, "{d} min", .{@as(i32, @intFromFloat(df.hoursToMinutes(2)))}) catch "");
    addKv(doc, srow, "daysToWeeks(14)", std.fmt.bufPrint(&tmp_buf, "{d} wk", .{@as(i32, @intFromFloat(df.daysToWeeks(14)))}) catch "");
    appendAndRelease(footer, srow);
    appendAndRelease(app, footer);
}
