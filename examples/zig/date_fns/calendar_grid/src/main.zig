const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: anytype) dom.HTMLElement {
    const el = doc.createElementWithString(tag, null);
    if (cls.len > 0) el.setClassName(cls);
    if (@TypeOf(text) != @TypeOf(null)) el.setTextContent(text);
    return el;
}

fn addMetric(doc: dom.Document, metrics: dom.HTMLElement, label: []const u8, val: []const u8) void {
    const m = makeEl(doc, "div", "metric", null);
    _ = m.appendChild(makeEl(doc, "div", "label", label));
    _ = m.appendChild(makeEl(doc, "div", "val", val));
    _ = metrics.appendChild(m);
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Date-fns Calendar Grid (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "March 2024"));
    _ = app.appendChild(head);

    const ref_day = df.parseISO("2024-03-15", null);
    const month_start = df.startOfMonth(ref_day, null);
    const month_end = df.endOfMonth(ref_day, null);
    const offset: usize = @intFromFloat(df.getDay(month_start, null));

    var tmp_buf: [64]u8 = undefined;
    const metrics = makeEl(doc, "div", "metrics", null);
    addMetric(doc, metrics, "Days in Month", std.fmt.bufPrint(&tmp_buf, "{d}", .{@as(i32, @intFromFloat(df.getDaysInMonth(ref_day, null)))}) catch "");
    addMetric(doc, metrics, "Quarter", std.fmt.bufPrint(&tmp_buf, "Q{d}", .{@as(i32, @intFromFloat(df.getQuarter(ref_day, null)))}) catch "");
    addMetric(doc, metrics, "ISO Week", std.fmt.bufPrint(&tmp_buf, "W{d}", .{@as(i32, @intFromFloat(df.getISOWeek(ref_day, null)))}) catch "");
    const leap_prefix = if (df.isLeapYear(ref_day, null)) "Yes \xC2\xB7 #" else "No \xC2\xB7 #";
    addMetric(doc, metrics, "Leap Year / DOY", std.fmt.bufPrint(&tmp_buf, "{s}{d}", .{ leap_prefix, @as(i32, @intFromFloat(df.getDayOfYear(ref_day, null))) }) catch "");
    _ = app.appendChild(metrics);

    const card = makeEl(doc, "div", "card", null);
    _ = card.appendChild(makeEl(doc, "div", "label", "March 2024 \xE2\x80\x94 Reference 2024-03-15"));

    const grid = makeEl(doc, "div", "cal-grid", null);
    for ([_][]const u8{ "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" }) |dow| {
        _ = grid.appendChild(makeEl(doc, "div", "dow", dow));
    }
    for (0..offset) |_| {
        _ = grid.appendChild(makeEl(doc, "div", "day-cell empty", null));
    }

    const days = df.eachDayOfInterval(.{ .start = month_start, .end = month_end }, null);
    for (0..days.len()) |i| {
        const d = days.get(i).Date;
        const is_ref = df.isSameDay(d, ref_day, null);
        const is_wknd = df.isWeekend(d, null);
        const cls = if (is_ref) "day-cell today" else if (is_wknd) "day-cell weekend" else "day-cell";

        const cell = makeEl(doc, "div", cls, null);
        _ = cell.appendChild(makeEl(doc, "span", "day-num", df.formatDate(d, "d", null)));
        if (is_ref) {
            _ = cell.appendChild(makeEl(doc, "span", "sub", "REF"));
        } else {
            _ = cell.appendChild(makeEl(doc, "span", "sub", df.formatDate(d, "EEE", null)));
        }
        _ = grid.appendChild(cell);
    }

    _ = card.appendChild(grid);
    _ = app.appendChild(card);
}
