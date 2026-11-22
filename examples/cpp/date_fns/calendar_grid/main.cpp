#include "dom/dom.hpp"
#include "date_fns/date_fns.hpp"

using Span = date_fns::Interval<date_fns::DateArg<>, date_fns::DateArg<>>;

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static void add_metric(dom::Document doc, dom::HTMLElement metrics, std::string_view label, std::string_view val) {
    dom::HTMLElement m = make_el(doc, "div", "metric", "");
    m.appendChild(make_el(doc, "div", "label", label));
    m.appendChild(make_el(doc, "div", "val", val));
    metrics.appendChild(m);
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Date-fns Calendar Grid (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "March 2024"));
    app.appendChild(head);

    auto ref_day = date_fns::parseISO("2024-03-15");
    auto month_start = date_fns::startOfMonth(ref_day);
    auto month_end = date_fns::endOfMonth(ref_day);
    int offset = static_cast<int>(date_fns::getDay(month_start));

    auto metrics = make_el(doc, "div", "metrics", "");
    add_metric(doc, metrics, "Days in Month", std::to_string(static_cast<int>(date_fns::getDaysInMonth(ref_day))));
    add_metric(doc, metrics, "Quarter", "Q" + std::to_string(static_cast<int>(date_fns::getQuarter(ref_day))));
    add_metric(doc, metrics, "ISO Week", "W" + std::to_string(static_cast<int>(date_fns::getISOWeek(ref_day))));
    std::string doy = std::string(date_fns::isLeapYear(ref_day) ? "Yes \xC2\xB7 #" : "No \xC2\xB7 #") +
                      std::to_string(static_cast<int>(date_fns::getDayOfYear(ref_day)));
    add_metric(doc, metrics, "Leap Year / DOY", doy);
    app.appendChild(metrics);

    auto card = make_el(doc, "div", "card", "");
    card.appendChild(make_el(doc, "div", "label", "March 2024 \xE2\x80\x94 Reference 2024-03-15"));

    auto grid = make_el(doc, "div", "cal-grid", "");
    const char* dows[] = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
    for (const char* dow : dows) {
        grid.appendChild(make_el(doc, "div", "dow", dow));
    }
    for (int i = 0; i < offset; i++) {
        grid.appendChild(make_el(doc, "div", "day-cell empty", ""));
    }

    auto span = Span({.start = month_start, .end = month_end});
    auto days = date_fns::eachDayOfInterval(span);
    for (size_t i = 0; i < days.size(); i++) {
        date_fns::Date_or_string_or_number item = days[i];
        bool is_ref = date_fns::isSameDay(item, ref_day);
        bool is_wknd = date_fns::isWeekend(item);
        std::string cls = is_ref ? "day-cell today" : (is_wknd ? "day-cell weekend" : "day-cell");
        dom::HTMLElement cell = make_el(doc, "div", cls, "");
        cell.appendChild(make_el(doc, "span", "day-num", date_fns::formatDate(item, "d")));
        cell.appendChild(make_el(doc, "span", "sub", is_ref ? "REF" : date_fns::formatDate(item, "EEE")));
        grid.appendChild(cell);
    }

    card.appendChild(grid);
    app.appendChild(card);
}
