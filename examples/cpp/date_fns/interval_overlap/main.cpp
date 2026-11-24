#include "dom/dom.hpp"
#include "date_fns/date_fns.hpp"

using Span = date_fns::Interval;

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static Span make_span(std::string_view s_iso, std::string_view e_iso) {
    return Span({.start = date_fns::parseISO(s_iso), .end = date_fns::parseISO(e_iso)});
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Date-fns Interval Overlap & Clamp (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "March 2024 \xC2\xB7 31 days"));
    app.appendChild(head);

    Span win_span = make_span("2024-03-01", "2024-03-31");
    auto month_days = date_fns::eachDayOfInterval(win_span);

    Span sprint_a = make_span("2024-03-08", "2024-03-18");
    struct Track { const char* name; Span span; bool is_ref; };
    Track tracks[] = {
        { "Sprint A (Ref)", sprint_a, true },
        { "Design Review", make_span("2024-03-03", "2024-03-10"), false },
        { "QA Hardening", make_span("2024-03-15", "2024-03-25"), false },
        { "Post-Release", make_span("2024-03-22", "2024-03-29"), false },
    };

    dom::HTMLElement gantt_card = make_el(doc, "div", "card", "");
    gantt_card.appendChild(make_el(doc, "div", "label", "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"));
    dom::HTMLElement gantt = make_el(doc, "div", "gantt", "");

    dom::HTMLElement ghead = make_el(doc, "div", "gantt-head", "");
    ghead.appendChild(make_el(doc, "div", "", "Track"));
    dom::HTMLElement hslots = make_el(doc, "div", "slots", "");
    for (size_t i = 0; i < month_days.size(); i++) {
        hslots.appendChild(make_el(doc, "div", "slot-lbl", date_fns::formatDate(month_days[i], "d")));
    }
    ghead.appendChild(hslots);
    ghead.appendChild(make_el(doc, "div", "", "vs Ref"));
    gantt.appendChild(ghead);

    for (const auto& t : tracks) {
        dom::HTMLElement row = make_el(doc, "div", "gantt-row", "");
        dom::HTMLElement meta = make_el(doc, "div", "track-meta", "");
        meta.appendChild(make_el(doc, "span", "track-name", t.name));
        meta.appendChild(make_el(doc, "span", "track-dur", date_fns::formatDuration(date_fns::intervalToDuration(t.span))));
        row.appendChild(meta);

        dom::HTMLElement slots = make_el(doc, "div", "slots", "");
        for (size_t i = 0; i < month_days.size(); i++) {
            date_fns::DateArg d = month_days[i];
            bool in_track = date_fns::isWithinInterval(d, t.span);
            bool in_ref = date_fns::isWithinInterval(d, sprint_a);
            const char* cls = (in_track && !t.is_ref && in_ref) ? "slot overlap" : (in_track ? "slot active" : "slot");
            slots.appendChild(make_el(doc, "div", cls, ""));
        }
        row.appendChild(slots);

        bool overlaps = date_fns::areIntervalsOverlapping(t.span, sprint_a);
        const char* b_text = t.is_ref ? "REF" : (overlaps ? "OVERLAP" : "DISJOINT");
        const char* b_cls = (t.is_ref || overlaps) ? "pill" : "pill-muted";
        row.appendChild(make_el(doc, "span", b_cls, b_text));
        gantt.appendChild(row);
    }

    gantt_card.appendChild(gantt);
    app.appendChild(gantt_card);

    dom::HTMLElement clamp_card = make_el(doc, "div", "card", "");
    clamp_card.appendChild(make_el(doc, "div", "label", "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"));

    dom::HTMLElement tbl = make_el(doc, "table", "", "");
    dom::HTMLElement thead = make_el(doc, "thead", "", "");
    dom::HTMLElement hrow = make_el(doc, "tr", "", "");
    for (const char* h : { "Probe Date", "Clamped Output", "Rule" }) {
        hrow.appendChild(make_el(doc, "th", "", h));
    }
    thead.appendChild(hrow);
    tbl.appendChild(thead);

    struct Probe { const char* iso; const char* rule; };
    Probe probes[] = {
        { "2024-03-02", "Clamped to start" },
        { "2024-03-14", "Inside interval" },
        { "2024-03-28", "Clamped to end" },
    };
    dom::HTMLElement tbody = make_el(doc, "tbody", "", "");
    for (const auto& p : probes) {
        auto pd = date_fns::parseISO(p.iso);
        auto clamped = date_fns::clamp(pd, sprint_a);
        dom::HTMLElement tr = make_el(doc, "tr", "", "");
        tr.appendChild(make_el(doc, "td", "", p.iso));
        tr.appendChild(make_el(doc, "td", "", date_fns::formatDate(clamped, "yyyy-MM-dd")));
        tr.appendChild(make_el(doc, "td", "", p.rule));
        tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    clamp_card.appendChild(tbl);
    app.appendChild(clamp_card);
}
