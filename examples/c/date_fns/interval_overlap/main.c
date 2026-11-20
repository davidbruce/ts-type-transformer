#include "dom/dom.h"
#include "date_fns/date_fns.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag, (ElementCreationOptions){ 0 });
    if (cls && cls[0]) Element_set_className(el, cls);
    if (text) Node_set_textContent(el, text);
    return el;
}

static void append_and_release(HTMLElement parent, HTMLElement child) {
    js_release(Node_appendChild(parent, child));
    js_release(child);
}

static void copy_and_release(JsString s, char* buf, uint32_t cap) {
    JsString_copy(s, buf, cap);
    js_release(s);
}

static Interval make_span(const char* s_iso, const char* e_iso) {
    Date s = parseISO(s_iso);
    Date e = parseISO(e_iso);
    Interval span = Interval_create();
    Interval_set_start(span, DateArg_of_Date(s));
    Interval_set_end(span, DateArg_of_Date(e));
    js_release(s);
    js_release(e);
    return span;
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Date-fns Interval Overlap & Clamp (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "March 2024 \xC2\xB7 31 days"));
    append_and_release(app, head);

    Interval win_span = make_span("2024-03-01", "2024-03-31");
    JsArray_DateArg month_days = eachDayOfInterval(win_span);
    uint32_t day_count = JsArray_DateArg_length(month_days);

    Interval sprint_a = make_span("2024-03-08", "2024-03-18");
    const char* track_names[4] = { "Sprint A (Ref)", "Design Review", "QA Hardening", "Post-Release" };
    Interval tracks[4] = {
        sprint_a,
        make_span("2024-03-03", "2024-03-10"),
        make_span("2024-03-15", "2024-03-25"),
        make_span("2024-03-22", "2024-03-29"),
    };

    HTMLElement gantt_card = make_el(doc, "div", "card", 0);
    append_and_release(gantt_card, make_el(doc, "div", "label", "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"));
    HTMLElement gantt = make_el(doc, "div", "gantt", 0);

    HTMLElement ghead = make_el(doc, "div", "gantt-head", 0);
    append_and_release(ghead, make_el(doc, "div", 0, "Track"));
    HTMLElement hslots = make_el(doc, "div", "slots", 0);
    for (uint32_t i = 0; i < day_count; i++) {
        Date d = JsArray_DateArg_get(month_days, i).as.Date;
        char dbuf[8];
        copy_and_release(formatDate_with_Date(d, "d", (FormatDateOptions){ 0 }), dbuf, sizeof(dbuf));
        append_and_release(hslots, make_el(doc, "div", "slot-lbl", dbuf));
        js_release(d);
    }
    append_and_release(ghead, hslots);
    append_and_release(ghead, make_el(doc, "div", 0, "vs Ref"));
    append_and_release(gantt, ghead);

    for (int t = 0; t < 4; t++) {
        bool is_ref = (t == 0);
        HTMLElement row = make_el(doc, "div", "gantt-row", 0);
        HTMLElement meta = make_el(doc, "div", "track-meta", 0);
        append_and_release(meta, make_el(doc, "span", "track-name", track_names[t]));

        Duration dur = intervalToDuration(tracks[t]);
        char dur_buf[64];
        copy_and_release(formatDuration(dur), dur_buf, sizeof(dur_buf));
        js_release(dur);
        append_and_release(meta, make_el(doc, "span", "track-dur", dur_buf));
        append_and_release(row, meta);

        HTMLElement slots = make_el(doc, "div", "slots", 0);
        for (uint32_t i = 0; i < day_count; i++) {
            Date d = JsArray_DateArg_get(month_days, i).as.Date;
            bool in_track = isWithinInterval_with_Date(d, tracks[t], (IsWithinIntervalOptions){ 0 });
            bool in_ref = isWithinInterval_with_Date(d, sprint_a, (IsWithinIntervalOptions){ 0 });
            const char* cls = (in_track && !is_ref && in_ref) ? "slot overlap" : (in_track ? "slot active" : "slot");
            append_and_release(slots, make_el(doc, "div", cls, 0));
            js_release(d);
        }
        append_and_release(row, slots);

        bool overlaps = areIntervalsOverlapping(tracks[t], sprint_a, (AreIntervalsOverlappingOptions){ 0 });
        const char* b_text = is_ref ? "REF" : (overlaps ? "OVERLAP" : "DISJOINT");
        const char* b_cls = (is_ref || overlaps) ? "pill" : "pill-muted";
        append_and_release(row, make_el(doc, "span", b_cls, b_text));
        append_and_release(gantt, row);
    }

    append_and_release(gantt_card, gantt);
    append_and_release(app, gantt_card);

    HTMLElement clamp_card = make_el(doc, "div", "card", 0);
    append_and_release(clamp_card, make_el(doc, "div", "label", "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    append_and_release(hrow, make_el(doc, "th", 0, "Probe Date"));
    append_and_release(hrow, make_el(doc, "th", 0, "Clamped Output"));
    append_and_release(hrow, make_el(doc, "th", 0, "Rule"));
    append_and_release(thead, hrow);
    append_and_release(tbl, thead);

    const char* probes[3] = { "2024-03-02", "2024-03-14", "2024-03-28" };
    const char* rules[3] = { "Clamped to start", "Inside interval", "Clamped to end" };
    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    for (int i = 0; i < 3; i++) {
        Date p = parseISO(probes[i]);
        DateArg res = clamp(DateArg_of_Date(p), sprint_a, (ClampOptions){ 0 });
        char cbuf[32];
        copy_and_release(formatDate_with_Date(res.as.Date, "yyyy-MM-dd", (FormatDateOptions){ 0 }), cbuf, sizeof(cbuf));
        js_release(res.as.Date);
        js_release(p);

        HTMLElement tr = make_el(doc, "tr", 0, 0);
        append_and_release(tr, make_el(doc, "td", 0, probes[i]));
        append_and_release(tr, make_el(doc, "td", 0, cbuf));
        append_and_release(tr, make_el(doc, "td", 0, rules[i]));
        append_and_release(tbody, tr);
    }
    append_and_release(tbl, tbody);
    append_and_release(clamp_card, tbl);
    append_and_release(app, clamp_card);

    for (int t = 1; t < 4; t++) js_release(tracks[t]);
    js_release(sprint_a);
    js_release(month_days);
    js_release(win_span);
    js_release(app);
    js_release(doc);
}
