#include "dom/dom.hpp"
#include "date_fns/date_fns.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Date-fns Relative Timeline (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "Anchor 2024-03-15"));
    app.appendChild(head);

    auto anchor = date_fns::parseISO("2024-03-15");
    auto opts = date_fns::FormatDistanceOptions::create().addSuffix(true);

    struct Entry { const char* iso; const char* title; };
    Entry entries[] = {
        { "2024-01-10", "Spec Draft" },
        { "2024-02-29", "Leap Day Freeze" },
        { "2024-03-12", "Beta Tag" },
        { "2024-03-15", "Release Anchor" },
        { "2024-03-18", "Patch Window" },
        { "2024-06-15", "Q2 Audit" },
    };

    auto tl_card = make_el(doc, "div", "card", "");
    tl_card.appendChild(make_el(doc, "div", "label", "Milestones Relative to 2024-03-15"));
    auto timeline = make_el(doc, "div", "timeline", "");

    auto all_dates = ts::Array<date_fns::DateArg<>>::create();
    for (const auto& e : entries) {
        auto d = date_fns::parseISO(e.iso);
        all_dates.push(d);

        bool same = date_fns::isSameDay(d, anchor);
        dom::HTMLElement row = make_el(doc, "div", same ? "milestone anchor" : "milestone", "");
        row.appendChild(make_el(doc, "span", "muted", date_fns::formatDate(d, "yyyy-MM-dd (EEE)")));
        row.appendChild(make_el(doc, "span", "title", e.title));
        std::string rel = same ? "today (anchor)" : date_fns::formatDistance(d, anchor, opts);
        row.appendChild(make_el(doc, "span", "", rel));
        int cmp = static_cast<int>(date_fns::compareAsc(d, anchor));
        std::string strict = date_fns::formatDistanceStrict(d, anchor) + " \xC2\xB7 cmp " + std::to_string(cmp);
        row.appendChild(make_el(doc, "span", "muted", strict));
        const char* status = same ? "ANCHOR" : (date_fns::isBefore(d, anchor) ? "PAST" : (date_fns::isAfter(d, anchor) ? "FUTURE" : "OTHER"));
        row.appendChild(make_el(doc, "span", "pill", status));
        timeline.appendChild(row);
    }
    tl_card.appendChild(timeline);
    app.appendChild(tl_card);

    auto earliest = date_fns::min(all_dates);
    auto latest = date_fns::max(all_dates);
    int total_span = static_cast<int>(date_fns::differenceInCalendarDays(latest, earliest));

    dom::HTMLElement sum_card = make_el(doc, "div", "card", "");
    sum_card.appendChild(make_el(doc, "div", "label", "Extremes & Total Span"));
    dom::HTMLElement summary = make_el(doc, "div", "summary", "");
    summary.appendChild(make_el(doc, "span", "", "Earliest: " + date_fns::formatDate(earliest, "yyyy-MM-dd")));
    summary.appendChild(make_el(doc, "span", "", "Latest: " + date_fns::formatDate(latest, "yyyy-MM-dd")));
    summary.appendChild(make_el(doc, "span", "", "Total Span: " + std::to_string(total_span) + " days"));
    sum_card.appendChild(summary);
    app.appendChild(sum_card);
}
