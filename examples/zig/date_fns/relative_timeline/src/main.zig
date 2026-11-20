const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

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

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Date-fns Relative Timeline (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "Anchor 2024-03-15"));
    _ = app.appendChild(head);

    const anchor = df.parseISO("2024-03-15", null);
    const Entry = struct { iso: []const u8, title: []const u8 };
    const entries = [_]Entry{
        .{ .iso = "2024-01-10", .title = "Spec Draft" },
        .{ .iso = "2024-02-29", .title = "Leap Day Freeze" },
        .{ .iso = "2024-03-12", .title = "Beta Tag" },
        .{ .iso = "2024-03-15", .title = "Release Anchor" },
        .{ .iso = "2024-03-18", .title = "Patch Window" },
        .{ .iso = "2024-06-15", .title = "Q2 Audit" },
    };

    const tl_card = makeEl(doc, "div", "card", null);
    _ = tl_card.appendChild(makeEl(doc, "div", "label", "Milestones Relative to 2024-03-15"));
    const timeline = makeEl(doc, "div", "timeline", null);

    const all_dates = df.JsArrayOf(df.DateArg).new();
    var tmp_buf: [128]u8 = undefined;
    var line_buf: [128]u8 = undefined;

    for (entries) |e| {
        const d = df.parseISO(e.iso, null);
        all_dates.push(d);

        const same = df.isSameDay(d, anchor, null);
        const row = makeEl(doc, "div", if (same) "milestone anchor" else "milestone", null);

        _ = row.appendChild(makeEl(doc, "span", "muted", df.formatDate(d, "yyyy-MM-dd (EEE)", null).copy(&tmp_buf)));
        _ = row.appendChild(makeEl(doc, "span", "title", e.title));

        if (same) {
            _ = row.appendChild(makeEl(doc, "span", "", "today (anchor)"));
        } else {
            _ = row.appendChild(makeEl(doc, "span", "", df.formatDistance(d, anchor, .{ .addSuffix = true }).copy(&tmp_buf)));
        }

        const strict = df.formatDistanceStrict(d, anchor, null);
        const cmp: i32 = @intFromFloat(df.compareAsc(d, anchor));
        const strict_full = std.fmt.bufPrint(&line_buf, "{f} \xC2\xB7 cmp {d}", .{ strict, cmp }) catch "";
        _ = row.appendChild(makeEl(doc, "span", "muted", strict_full));

        const status = if (same) "ANCHOR" else if (df.isBefore(d, anchor)) "PAST" else if (df.isAfter(d, anchor)) "FUTURE" else "OTHER";
        _ = row.appendChild(makeEl(doc, "span", "pill", status));

        _ = timeline.appendChild(row);
    }
    _ = tl_card.appendChild(timeline);
    _ = app.appendChild(tl_card);

    const earliest = df.min(all_dates, null);
    const latest = df.max(all_dates, null);
    const total_span: i32 = @intFromFloat(df.differenceInCalendarDays(latest, earliest, null));

    const sum_card = makeEl(doc, "div", "card", null);
    _ = sum_card.appendChild(makeEl(doc, "div", "label", "Extremes & Total Span"));
    const summary = makeEl(doc, "div", "summary", null);

    _ = summary.appendChild(makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Earliest: {f}", .{df.formatDate(earliest, "yyyy-MM-dd", null)}) catch ""));
    _ = summary.appendChild(makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Latest: {f}", .{df.formatDate(latest, "yyyy-MM-dd", null)}) catch ""));
    _ = summary.appendChild(makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Total Span: {d} days", .{total_span}) catch ""));
    _ = sum_card.appendChild(summary);
    _ = app.appendChild(sum_card);
}
