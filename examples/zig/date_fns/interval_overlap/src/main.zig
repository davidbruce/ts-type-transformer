const dom = @import("dom");
const df = @import("date_fns");

fn makeSpan(s_iso: []const u8, e_iso: []const u8) df.Interval {
    return df.Interval.init(.{
        .start = df.parseISO(s_iso),
        .end = df.parseISO(e_iso),
    });
}

fn makeEl(doc: dom.Document, tag: []const u8, cls: []const u8, text: anytype) dom.HTMLElement {
    const el = doc.createElementWithString(tag);
    if (cls.len > 0) el.setClassName(cls);
    if (@TypeOf(text) != @TypeOf(null)) el.setTextContent(text);
    return el;
}

export fn start() void {
    const scope = dom.JsScope.open();
    defer scope.deinit();

    const doc = dom.getDocument();
    const app = doc.getElementById("app") orelse return;

    const head = makeEl(doc, "div", "head", null);
    _ = head.appendChild(makeEl(doc, "h1", "", "Date-fns Interval Overlap & Clamp (Zig)"));
    _ = head.appendChild(makeEl(doc, "span", "badge", "March 2024 \xC2\xB7 31 days"));
    _ = app.appendChild(head);

    const win_span = makeSpan("2024-03-01", "2024-03-31");
    const month_days = df.eachDayOfInterval(win_span);
    const day_count = month_days.len();

    const sprint_a = makeSpan("2024-03-08", "2024-03-18");
    const Track = struct { name: []const u8, span: df.Interval, is_ref: bool };
    const tracks = [_]Track{
        .{ .name = "Sprint A (Ref)", .span = sprint_a, .is_ref = true },
        .{ .name = "Design Review", .span = makeSpan("2024-03-03", "2024-03-10"), .is_ref = false },
        .{ .name = "QA Hardening", .span = makeSpan("2024-03-15", "2024-03-25"), .is_ref = false },
        .{ .name = "Post-Release", .span = makeSpan("2024-03-22", "2024-03-29"), .is_ref = false },
    };

    const gantt_card = makeEl(doc, "div", "card", null);
    _ = gantt_card.appendChild(makeEl(doc, "div", "label", "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"));
    const gantt = makeEl(doc, "div", "gantt", null);

    const ghead = makeEl(doc, "div", "gantt-head", null);
    _ = ghead.appendChild(makeEl(doc, "div", "", "Track"));
    const hslots = makeEl(doc, "div", "slots", null);
    for (0..day_count) |i| {
        const d = month_days.get(i).Date;
        _ = hslots.appendChild(makeEl(doc, "div", "slot-lbl", df.formatDate(d, "d")));
    }
    _ = ghead.appendChild(hslots);
    _ = ghead.appendChild(makeEl(doc, "div", "", "vs Ref"));
    _ = gantt.appendChild(ghead);

    for (tracks) |t| {
        const row = makeEl(doc, "div", "gantt-row", null);
        const meta = makeEl(doc, "div", "track-meta", null);
        _ = meta.appendChild(makeEl(doc, "span", "track-name", t.name));
        const dur = df.intervalToDuration(t.span);
        _ = meta.appendChild(makeEl(doc, "span", "track-dur", df.formatDuration(dur)));
        _ = row.appendChild(meta);

        const slots = makeEl(doc, "div", "slots", null);
        for (0..day_count) |i| {
            const d = month_days.get(i).Date;
            const in_track = df.isWithinInterval(d, t.span);
            const in_ref = df.isWithinInterval(d, sprint_a);
            const cls = if (in_track and !t.is_ref and in_ref) "slot overlap" else if (in_track) "slot active" else "slot";
            _ = slots.appendChild(makeEl(doc, "div", cls, null));
        }
        _ = row.appendChild(slots);

        const overlaps = df.areIntervalsOverlapping(t.span, sprint_a);
        const b_text = if (t.is_ref) "REF" else if (overlaps) "OVERLAP" else "DISJOINT";
        const b_cls = if (t.is_ref or overlaps) "pill" else "pill-muted";
        _ = row.appendChild(makeEl(doc, "span", b_cls, b_text));
        _ = gantt.appendChild(row);
    }
    _ = gantt_card.appendChild(gantt);
    _ = app.appendChild(gantt_card);

    const clamp_card = makeEl(doc, "div", "card", null);
    _ = clamp_card.appendChild(makeEl(doc, "div", "label", "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"));

    const tbl = makeEl(doc, "table", "", null);
    const thead = makeEl(doc, "thead", "", null);
    const hrow = makeEl(doc, "tr", "", null);
    for ([_][]const u8{ "Probe Date", "Clamped Output", "Rule" }) |h| {
        _ = hrow.appendChild(makeEl(doc, "th", "", h));
    }
    _ = thead.appendChild(hrow);
    _ = tbl.appendChild(thead);

    const Probe = struct { iso: []const u8, rule: []const u8 };
    const probes = [_]Probe{
        .{ .iso = "2024-03-02", .rule = "Clamped to start" },
        .{ .iso = "2024-03-14", .rule = "Inside interval" },
        .{ .iso = "2024-03-28", .rule = "Clamped to end" },
    };
    const tbody = makeEl(doc, "tbody", "", null);
    for (probes) |p| {
        const pd = df.parseISO(p.iso);
        const clamped = df.clamp(pd, sprint_a);
        const tr = makeEl(doc, "tr", "", null);
        _ = tr.appendChild(makeEl(doc, "td", "", p.iso));
        _ = tr.appendChild(makeEl(doc, "td", "", df.formatDate(clamped, "yyyy-MM-dd")));
        _ = tr.appendChild(makeEl(doc, "td", "", p.rule));
        _ = tbody.appendChild(tr);
    }
    _ = tbl.appendChild(tbody);
    _ = clamp_card.appendChild(tbl);
    _ = app.appendChild(clamp_card);
}
