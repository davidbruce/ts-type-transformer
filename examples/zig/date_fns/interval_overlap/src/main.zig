const dom = @import("dom");
const df = @import("date_fns");

var tmp_buf: [64]u8 = undefined;

fn day(d: df.Date) df.DateArg {
    return .{ .Date = d };
}

fn makeSpan(s_iso: []const u8, e_iso: []const u8) df.Interval {
    const s = df.parseISO(s_iso, .{});
    defer df.jsRelease(s);
    const e = df.parseISO(e_iso, .{});
    defer df.jsRelease(e);
    const span = df.Interval.create();
    span.setStart(day(s));
    span.setEnd(day(e));
    return span;
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
    appendAndRelease(head, makeEl(doc, "h1", "", "Date-fns Interval Overlap & Clamp (Zig)"));
    appendAndRelease(head, makeEl(doc, "span", "badge", "March 2024 \xC2\xB7 31 days"));
    appendAndRelease(app, head);

    const win_span = makeSpan("2024-03-01", "2024-03-31");
    defer df.jsRelease(win_span);
    const month_days = df.eachDayOfInterval(win_span, .{});
    defer df.jsRelease(month_days);
    const day_count = month_days.len();

    const sprint_a = makeSpan("2024-03-08", "2024-03-18");
    defer df.jsRelease(sprint_a);
    const t_review = makeSpan("2024-03-03", "2024-03-10");
    defer df.jsRelease(t_review);
    const t_qa = makeSpan("2024-03-15", "2024-03-25");
    defer df.jsRelease(t_qa);
    const t_post = makeSpan("2024-03-22", "2024-03-29");
    defer df.jsRelease(t_post);

    const Track = struct { name: []const u8, span: df.Interval, is_ref: bool };
    const tracks = [_]Track{
        .{ .name = "Sprint A (Ref)", .span = sprint_a, .is_ref = true },
        .{ .name = "Design Review", .span = t_review, .is_ref = false },
        .{ .name = "QA Hardening", .span = t_qa, .is_ref = false },
        .{ .name = "Post-Release", .span = t_post, .is_ref = false },
    };

    const gantt_card = makeEl(doc, "div", "card", null);
    appendAndRelease(gantt_card, makeEl(doc, "div", "label", "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"));
    const gantt = makeEl(doc, "div", "gantt", null);

    const ghead = makeEl(doc, "div", "gantt-head", null);
    appendAndRelease(ghead, makeEl(doc, "div", "", "Track"));
    const hslots = makeEl(doc, "div", "slots", null);
    for (0..day_count) |i| {
        const d = month_days.get(i).Date;
        defer df.jsRelease(d);
        const s = df.formatDate(day(d), "d", .{});
        appendAndRelease(hslots, makeEl(doc, "div", "slot-lbl", s.copy(&tmp_buf)));
        df.jsRelease(s);
    }
    appendAndRelease(ghead, hslots);
    appendAndRelease(ghead, makeEl(doc, "div", "", "vs Ref"));
    appendAndRelease(gantt, ghead);

    for (tracks) |t| {
        const row = makeEl(doc, "div", "gantt-row", null);
        const meta = makeEl(doc, "div", "track-meta", null);
        appendAndRelease(meta, makeEl(doc, "span", "track-name", t.name));
        const dur = df.intervalToDuration(t.span, .{});
        const dur_s = df.formatDuration(dur, .{});
        appendAndRelease(meta, makeEl(doc, "span", "track-dur", dur_s.copy(&tmp_buf)));
        df.jsRelease(dur_s);
        df.jsRelease(dur);
        appendAndRelease(row, meta);

        const slots = makeEl(doc, "div", "slots", null);
        for (0..day_count) |i| {
            const d = month_days.get(i).Date;
            defer df.jsRelease(d);
            const in_track = df.isWithinInterval(day(d), t.span, .{});
            const in_ref = df.isWithinInterval(day(d), sprint_a, .{});
            const cls = if (in_track and !t.is_ref and in_ref) "slot overlap" else if (in_track) "slot active" else "slot";
            appendAndRelease(slots, makeEl(doc, "div", cls, null));
        }
        appendAndRelease(row, slots);

        const overlaps = df.areIntervalsOverlapping(t.span, sprint_a, .{});
        const b_text = if (t.is_ref) "REF" else if (overlaps) "OVERLAP" else "DISJOINT";
        const b_cls = if (t.is_ref or overlaps) "pill" else "pill-muted";
        appendAndRelease(row, makeEl(doc, "span", b_cls, b_text));
        appendAndRelease(gantt, row);
    }
    appendAndRelease(gantt_card, gantt);
    appendAndRelease(app, gantt_card);

    const clamp_card = makeEl(doc, "div", "card", null);
    appendAndRelease(clamp_card, makeEl(doc, "div", "label", "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Probe Date", "Clamped Output", "Rule" }) |h| {
        appendAndRelease(hrow, makeEl(doc, "th", "", h));
    }
    appendAndRelease(thead, hrow);
    appendAndRelease(tbl, thead);

    const Probe = struct { iso: []const u8, rule: []const u8 };
    const probes = [_]Probe{
        .{ .iso = "2024-03-02", .rule = "Clamped to start" },
        .{ .iso = "2024-03-14", .rule = "Inside interval" },
        .{ .iso = "2024-03-28", .rule = "Clamped to end" },
    };
    const tbody = makeEl(doc, "tbody", "", null);
    for (probes) |p| {
        const pd = df.parseISO(p.iso, .{});
        defer df.jsRelease(pd);
        const clamped = df.clamp(day(pd), sprint_a, .{});
        defer switch (clamped) {
            .Date => |cd| df.jsRelease(cd),
            .string => |cs| df.jsRelease(cs),
            .number => {},
        };
        const out_s = df.formatDate(clamped, "yyyy-MM-dd", .{});
        defer df.jsRelease(out_s);

        const tr = makeEl(doc, "tr", "", null);
        appendAndRelease(tr, makeEl(doc, "td", "", p.iso));
        appendAndRelease(tr, makeEl(doc, "td", "", out_s.copy(&tmp_buf)));
        appendAndRelease(tr, makeEl(doc, "td", "", p.rule));
        appendAndRelease(tbody, tr);
    }
    appendAndRelease(tbl, tbody);
    appendAndRelease(clamp_card, tbl);
    appendAndRelease(app, clamp_card);
}
