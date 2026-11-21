#include "dom/dom.h"
#include "date_fns/date_fns.h"

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static Df_Interval make_span(const char* s_iso, const char* e_iso) {
    Df_Interval span = Df_Interval_create();
    Df_Interval_set_start(span, Df_DateArg_of(df_parseISO(s_iso)));
    Df_Interval_set_end(span, Df_DateArg_of(df_parseISO(e_iso)));
    return span;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Date-fns Interval Overlap & Clamp (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "March 2024 \xC2\xB7 31 days"));
    HTMLElement_appendChild(app, head);

    Df_Interval win_span = make_span("2024-03-01", "2024-03-31");
    JsArray_Df_DateArg month_days = df_eachDayOfInterval(win_span);
    uint32_t day_count = JsArray_Df_DateArg_length(month_days);

    Df_Interval sprint_a = make_span("2024-03-08", "2024-03-18");
    const char* track_names[4] = { "Sprint A (Ref)", "Design Review", "QA Hardening", "Post-Release" };
    Df_Interval tracks[4] = {
        sprint_a,
        make_span("2024-03-03", "2024-03-10"),
        make_span("2024-03-15", "2024-03-25"),
        make_span("2024-03-22", "2024-03-29"),
    };

    HTMLElement gantt_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(gantt_card, make_el(doc, "div", "label", "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"));
    HTMLElement gantt = make_el(doc, "div", "gantt", 0);

    HTMLElement ghead = make_el(doc, "div", "gantt-head", 0);
    HTMLElement_appendChild(ghead, make_el(doc, "div", 0, "Track"));
    HTMLElement hslots = make_el(doc, "div", "slots", 0);
    for (uint32_t i = 0; i < day_count; i++) {
        Date d = JsArray_Df_DateArg_get(month_days, i).as.Date;
        HTMLElement lbl = make_el(doc, "div", "slot-lbl", 0);
        HTMLElement_set_textContent(lbl, df_formatDate_with_Date(d, "d"));
        HTMLElement_appendChild(hslots, lbl);
    }
    HTMLElement_appendChild(ghead, hslots);
    HTMLElement_appendChild(ghead, make_el(doc, "div", 0, "vs Ref"));
    HTMLElement_appendChild(gantt, ghead);

    for (int t = 0; t < 4; t++) {
        bool is_ref = (t == 0);
        HTMLElement row = make_el(doc, "div", "gantt-row", 0);
        HTMLElement meta = make_el(doc, "div", "track-meta", 0);
        HTMLElement_appendChild(meta, make_el(doc, "span", "track-name", track_names[t]));

        HTMLElement dur_el = make_el(doc, "span", "track-dur", 0);
        HTMLElement_set_textContent(dur_el, df_formatDuration(df_intervalToDuration(tracks[t])));
        HTMLElement_appendChild(meta, dur_el);
        HTMLElement_appendChild(row, meta);

        HTMLElement slots = make_el(doc, "div", "slots", 0);
        for (uint32_t i = 0; i < day_count; i++) {
            Date d = JsArray_Df_DateArg_get(month_days, i).as.Date;
            bool in_track = df_isWithinInterval_with_Date(d, tracks[t]);
            bool in_ref = df_isWithinInterval_with_Date(d, sprint_a);
            const char* cls = (in_track && !is_ref && in_ref) ? "slot overlap" : (in_track ? "slot active" : "slot");
            HTMLElement_appendChild(slots, make_el(doc, "div", cls, 0));
        }
        HTMLElement_appendChild(row, slots);

        bool overlaps = df_areIntervalsOverlapping(tracks[t], sprint_a);
        const char* b_text = is_ref ? "REF" : (overlaps ? "OVERLAP" : "DISJOINT");
        const char* b_cls = (is_ref || overlaps) ? "pill" : "pill-muted";
        HTMLElement_appendChild(row, make_el(doc, "span", b_cls, b_text));
        HTMLElement_appendChild(gantt, row);
    }

    HTMLElement_appendChild(gantt_card, gantt);
    HTMLElement_appendChild(app, gantt_card);

    HTMLElement clamp_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(clamp_card, make_el(doc, "div", "label", "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Probe Date"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Clamped Output"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Rule"));
    HTMLElement_appendChild(thead, hrow);
    HTMLElement_appendChild(tbl, thead);

    const char* probes[3] = { "2024-03-02", "2024-03-14", "2024-03-28" };
    const char* rules[3] = { "Clamped to start", "Inside interval", "Clamped to end" };
    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    for (int i = 0; i < 3; i++) {
        Df_DateArg res = df_clamp(Df_DateArg_of(df_parseISO(probes[i])), sprint_a);
        HTMLElement tr = make_el(doc, "tr", 0, 0);
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, probes[i]));
        HTMLElement td_clamped = make_el(doc, "td", 0, 0);
        HTMLElement_set_textContent(td_clamped, df_formatDate_with_Date(res.as.Date, "yyyy-MM-dd"));
        HTMLElement_appendChild(tr, td_clamped);
        HTMLElement_appendChild(tr, make_el(doc, "td", 0, rules[i]));
        HTMLElement_appendChild(tbody, tr);
    }
    HTMLElement_appendChild(tbl, tbody);
    HTMLElement_appendChild(clamp_card, tbl);
    HTMLElement_appendChild(app, clamp_card);

    js_scope_close();
}
