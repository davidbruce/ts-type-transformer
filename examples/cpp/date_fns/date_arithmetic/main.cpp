#include "dom/dom.hpp"
#include "date_fns/date_fns.hpp"

static dom::HTMLElement make_el(dom::Document doc, std::string_view tag, std::string_view cls, std::string_view text) {
    dom::HTMLElement el = doc.createElement(tag);
    if (!cls.empty()) el.className(cls);
    if (!text.empty()) el.textContent(text);
    return el;
}

static std::string fmt_signed(int v) {
    return (v > 0 ? "+" : "") + std::to_string(v);
}

static void add_kv(dom::Document doc, dom::HTMLElement row, std::string_view label, std::string_view val) {
    dom::HTMLElement box = make_el(doc, "div", "kv", "");
    box.appendChild(make_el(doc, "span", "label", label));
    box.appendChild(make_el(doc, "strong", "", val));
    row.appendChild(box);
}

TS_EXPORT(start) void start() {
    auto doc = dom::get_document();
    auto app = *doc.getElementById("app");

    auto head = make_el(doc, "div", "head", "");
    head.appendChild(make_el(doc, "h1", "", "Date-fns Arithmetic & Durations (C++)"));
    head.appendChild(make_el(doc, "span", "badge", "Base 2024-03-15"));
    app.appendChild(head);

    auto base_d = date_fns::parseISO("2024-03-15");
    auto jan31 = date_fns::parseISO("2024-01-31");
    auto dur = date_fns::Duration::create().months(1.0).days(2.0);

    struct OpRow { const char* name; date_fns::Date res; const char* fmt; };
    OpRow ops[] = {
        { "addDays(base, 3)", date_fns::addDays(base_d, 3.0), "yyyy-MM-dd" },
        { "subDays(base, 15)", date_fns::subDays(base_d, 15.0), "yyyy-MM-dd" },
        { "addWeeks(base, 2)", date_fns::addWeeks(base_d, 2.0), "yyyy-MM-dd" },
        { "addMonths(jan31, 1)", date_fns::addMonths(jan31, 1.0), "yyyy-MM-dd" },
        { "subMonths(base, 3)", date_fns::subMonths(base_d, 3.0), "yyyy-MM-dd" },
        { "addYears(base, 1)", date_fns::addYears(base_d, 1.0), "yyyy-MM-dd" },
        { "startOfWeek(base)", date_fns::startOfWeek(base_d), "yyyy-MM-dd" },
        { "endOfMonth(base)", date_fns::endOfMonth(base_d), "yyyy-MM-dd HH:mm:ss" },
        { "add(base, { months: 1, days: 2 })", date_fns::add(base_d, dur), "yyyy-MM-dd" },
        { "base (immutable check)", base_d, "yyyy-MM-dd" },
    };

    dom::HTMLElement card = make_el(doc, "div", "card", "");
    card.appendChild(make_el(doc, "div", "label", "Immutable Date Stepping"));

    dom::HTMLElement tbl = make_el(doc, "table", "", "");
    dom::HTMLElement thead = make_el(doc, "thead", "", "");
    dom::HTMLElement hrow = make_el(doc, "tr", "", "");
    for (const char* h : { "Operation", "Result", "\xCE\x94 Days (vs base)", "\xCE\x94 Hours" }) {
        hrow.appendChild(make_el(doc, "th", "", h));
    }
    thead.appendChild(hrow);
    tbl.appendChild(thead);

    dom::HTMLElement tbody = make_el(doc, "tbody", "", "");
    for (const auto& op : ops) {
        dom::HTMLElement tr = make_el(doc, "tr", "", "");
        dom::HTMLElement td_op = make_el(doc, "td", "", "");
        td_op.appendChild(make_el(doc, "code", "", op.name));
        tr.appendChild(td_op);
        tr.appendChild(make_el(doc, "td", "", date_fns::formatDate(op.res, op.fmt)));
        int dd = static_cast<int>(date_fns::differenceInCalendarDays(op.res, base_d));
        int dh = static_cast<int>(date_fns::differenceInHours(op.res, base_d));
        tr.appendChild(make_el(doc, "td", "", fmt_signed(dd)));
        tr.appendChild(make_el(doc, "td", "", fmt_signed(dh)));
        tbody.appendChild(tr);
    }
    tbl.appendChild(tbody);
    card.appendChild(tbl);
    app.appendChild(card);

    dom::HTMLElement footer = make_el(doc, "div", "card", "");
    footer.appendChild(make_el(doc, "div", "label", "Duration & Unit Conversions"));
    dom::HTMLElement srow = make_el(doc, "div", "summary-row", "");
    add_kv(doc, srow, "formatDuration(dur)", date_fns::formatDuration(dur));
    add_kv(doc, srow, "formatISODuration(dur)", date_fns::formatISODuration(dur));
    add_kv(doc, srow, "hoursToMinutes(2)", std::to_string(static_cast<int>(date_fns::hoursToMinutes(2.0))) + " min");
    add_kv(doc, srow, "daysToWeeks(14)", std::to_string(static_cast<int>(date_fns::daysToWeeks(14.0))) + " wk");
    footer.appendChild(srow);
    app.appendChild(footer);
}
