const std = @import("std");
const dom = @import("dom");
const df = @import("date_fns");

var tmp_buf: [128]u8 = undefined;
var line_buf: [128]u8 = undefined;

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

export fn start() void {
    const doc = dom.getDocument();
    defer dom.jsRelease(doc);
    const app = doc.getElementById("app").?;
    defer dom.jsRelease(app);

    const head = makeEl(doc, "div", "head", null);
    appendAndRelease(head, makeEl(doc, "h1", "", "Date-fns Relative Timeline (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "Anchor 2024-03-15"));
    appendAndRelease(app, head);

    const anchor = df.parseISO("2024-03-15", .{});
    defer df.jsRelease(anchor);
    const opts = df.FormatDistanceOptions.create();
    defer df.jsRelease(opts);
    opts.setAddSuffix(true);

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
    appendAndRelease(tl_card, makeEl(doc, "div", "label", "Milestones Relative to 2024-03-15"));
    const timeline = makeEl(doc, "div", "timeline", null);

    const all_dates = df.JsArrayOf(df.DateArg).new();
    defer df.jsRelease(all_dates);

    for (entries) |e| {
        const d = df.parseISO(e.iso, .{});
        defer df.jsRelease(d);
        all_dates.push(day(d));

        const same = df.isSameDay(day(d), day(anchor), .{});
        const row = makeEl(doc, "div", if (same) "milestone anchor" else "milestone", null);

        const date_s = df.formatDate(day(d), "yyyy-MM-dd (EEE)", .{});
        appendAndRelease(row, makeEl(doc, "span", "muted", date_s.copy(&tmp_buf)));
        df.jsRelease(date_s);

        appendAndRelease(row, makeEl(doc, "span", "title", e.title));

        if (same) {
            appendAndRelease(row, makeEl(doc, "span", "", "today (anchor)"));
        } else {
            const rel = df.formatDistance(day(d), day(anchor), .{ .options = opts });
            appendAndRelease(row, makeEl(doc, "span", "", rel.copy(&tmp_buf)));
            df.jsRelease(rel);
        }

        const strict = df.formatDistanceStrict(day(d), day(anchor), .{});
        const strict_txt = strict.copy(&tmp_buf);
        const cmp: i32 = @intFromFloat(df.compareAsc(day(d), day(anchor)));
        const strict_full = std.fmt.bufPrint(&line_buf, "{s} \xC2\xB7 cmp {d}", .{ strict_txt, cmp }) catch strict_txt;
        df.jsRelease(strict);
        appendAndRelease(row, makeEl(doc, "span", "muted", strict_full));

        const status = if (same) "ANCHOR" else if (df.isBefore(day(d), day(anchor))) "PAST" else if (df.isAfter(day(d), day(anchor))) "FUTURE" else "OTHER";
        appendAndRelease(row, makeEl(doc, "span", "pill", status));

        appendAndRelease(timeline, row);
    }
    appendAndRelease(tl_card, timeline);
    appendAndRelease(app, tl_card);

    const earliest = df.min(all_dates, .{});
    defer df.jsRelease(earliest);
    const latest = df.max(all_dates, .{});
    defer df.jsRelease(latest);
    const total_span: i32 = @intFromFloat(df.differenceInCalendarDays(day(latest), day(earliest), .{}));

    const sum_card = makeEl(doc, "div", "card", null);
    appendAndRelease(sum_card, makeEl(doc, "div", "label", "Extremes & Total Span"));
    const summary = makeEl(doc, "div", "summary", null);

    const e_s = df.formatDate(day(earliest), "yyyy-MM-dd", .{});
    appendAndRelease(summary, makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Earliest: {s}", .{e_s.copy(&tmp_buf)}) catch ""));
    df.jsRelease(e_s);

    const l_s = df.formatDate(day(latest), "yyyy-MM-dd", .{});
    appendAndRelease(summary, makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Latest: {s}", .{l_s.copy(&tmp_buf)}) catch ""));
    df.jsRelease(l_s);

    appendAndRelease(summary, makeEl(doc, "span", "", std.fmt.bufPrint(&line_buf, "Total Span: {d} days", .{total_span}) catch ""));
    appendAndRelease(sum_card, summary);
    appendAndRelease(app, sum_card);
}
