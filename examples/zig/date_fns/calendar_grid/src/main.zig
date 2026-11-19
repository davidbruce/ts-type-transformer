const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

var tmp_buf: [64]u8 = undefined;

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

fn addMetric(doc: dom.Document, metrics: dom.HTMLElement, label: []const u8, val: []const u8) void {
    const m = makeEl(doc, "div", "metric", null);
    appendAndRelease(m, makeEl(doc, "div", "label", label));
    appendAndRelease(m, makeEl(doc, "div", "val", val));
    appendAndRelease(metrics, m);
}

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Date-fns Calendar Grid (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "March 2024"));
    appendAndRelease(app, head);

    const ref_day = df.parseISO("2024-03-15", .{});
    defer df.jsRelease(ref_day);
    const month_start = df.startOfMonth(day(ref_day), .{});
    defer df.jsRelease(month_start);
    const month_end = df.endOfMonth(day(ref_day), .{});
    defer df.jsRelease(month_end);
    const offset: usize = @intFromFloat(df.getDay(day(month_start), .{}));

    const metrics = makeEl(doc, "div", "metrics", null);
    addMetric(doc, metrics, "Days in Month", std.fmt.bufPrint(&tmp_buf, "{d}", .{@as(i32, @intFromFloat(df.getDaysInMonth(day(ref_day), .{})))}) catch "");
    addMetric(doc, metrics, "Quarter", std.fmt.bufPrint(&tmp_buf, "Q{d}", .{@as(i32, @intFromFloat(df.getQuarter(day(ref_day), .{})))}) catch "");
    addMetric(doc, metrics, "ISO Week", std.fmt.bufPrint(&tmp_buf, "W{d}", .{@as(i32, @intFromFloat(df.getISOWeek(day(ref_day), .{})))}) catch "");
    const leap_prefix = if (df.isLeapYear(day(ref_day), .{})) "Yes \xC2\xB7 #" else "No \xC2\xB7 #";
    addMetric(doc, metrics, "Leap Year / DOY", std.fmt.bufPrint(&tmp_buf, "{s}{d}", .{ leap_prefix, @as(i32, @intFromFloat(df.getDayOfYear(day(ref_day), .{}))) }) catch "");
    appendAndRelease(app, metrics);

    const card = makeEl(doc, "div", "card", null);
    appendAndRelease(card, makeEl(doc, "div", "label", "March 2024 \xE2\x80\x94 Reference 2024-03-15"));

    const grid = makeEl(doc, "div", "cal-grid", null);
    const dows = [_][]const u8{ "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
    for (dows) |dow| {
        appendAndRelease(grid, makeEl(doc, "div", "dow", dow));
    }
    for (0..offset) |_| {
        appendAndRelease(grid, makeEl(doc, "div", "day-cell empty", null));
    }

    const span = df.Interval.create();
    defer df.jsRelease(span);
    span.setStart(day(month_start));
    span.setEnd(day(month_end));
    const days = df.eachDayOfInterval(span, .{});
    defer df.jsRelease(days);

    const count = days.len();
    for (0..count) |i| {
        const d = days.get(i).Date;
        defer df.jsRelease(d);
        const is_ref = df.isSameDay(day(d), day(ref_day), .{});
        const is_wknd = df.isWeekend(day(d), .{});
        const cls = if (is_ref) "day-cell today" else if (is_wknd) "day-cell weekend" else "day-cell";

        const cell = makeEl(doc, "div", cls, null);
        const day_num = df.formatDate(day(d), "d", .{});
        defer df.jsRelease(day_num);
        appendAndRelease(cell, makeEl(doc, "span", "day-num", day_num.copy(&tmp_buf)));

        if (is_ref) {
            appendAndRelease(cell, makeEl(doc, "span", "sub", "REF"));
        } else {
            const sub = df.formatDate(day(d), "EEE", .{});
            defer df.jsRelease(sub);
            appendAndRelease(cell, makeEl(doc, "span", "sub", sub.copy(&tmp_buf)));
        }
        appendAndRelease(grid, cell);
    }

    appendAndRelease(card, grid);
    appendAndRelease(app, card);
}
