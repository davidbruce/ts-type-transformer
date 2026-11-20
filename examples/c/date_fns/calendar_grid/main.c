#include "dom/dom.h"
#include "date_fns/date_fns.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static void add_metric(Document doc, HTMLElement metrics, const char* label, const char* val) {
    HTMLElement m = make_el(doc, "div", "metric", 0);
    HTMLElement_appendChild(m, make_el(doc, "div", "label", label));
    HTMLElement_appendChild(m, make_el(doc, "div", "val", val));
    HTMLElement_appendChild(metrics, m);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope_open();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Date-fns Calendar Grid (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "March 2024"));
    HTMLElement_appendChild(app, head);

    Date ref_day = parseISO("2024-03-15");
    DateArg ref_arg = DateArg_of(ref_day);
    Date month_start = startOfMonth(ref_arg);
    Date month_end = endOfMonth(ref_arg);
    int offset = (int)getDay(month_start);

    char b_days[16], b_q[16], b_w[16], b_doy[32];
    snprintf(b_days, sizeof b_days, "%d", (int)getDaysInMonth(ref_day));
    snprintf(b_q, sizeof b_q, "Q%d", (int)getQuarter(ref_day));
    snprintf(b_w, sizeof b_w, "W%d", (int)getISOWeek_with_Date(ref_day));
    snprintf(b_doy, sizeof b_doy, "%s \xC2\xB7 #%d",
        isLeapYear(ref_day) ? "Yes" : "No",
        (int)getDayOfYear_with_Date(ref_day));

    HTMLElement metrics = make_el(doc, "div", "metrics", 0);
    add_metric(doc, metrics, "Days in Month", b_days);
    add_metric(doc, metrics, "Quarter", b_q);
    add_metric(doc, metrics, "ISO Week", b_w);
    add_metric(doc, metrics, "Leap Year / DOY", b_doy);
    HTMLElement_appendChild(app, metrics);

    HTMLElement card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(card, make_el(doc, "div", "label", "March 2024 \xE2\x80\x94 Reference 2024-03-15"));

    HTMLElement grid = make_el(doc, "div", "cal-grid", 0);
    const char* dows[] = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
    for (int i = 0; i < 7; i++) {
        HTMLElement_appendChild(grid, make_el(doc, "div", "dow", dows[i]));
    }
    for (int i = 0; i < offset; i++) {
        HTMLElement_appendChild(grid, make_el(doc, "div", "day-cell empty", 0));
    }

    Interval span = Interval_create();
    Interval_set_start(span, DateArg_of(month_start));
    Interval_set_end(span, DateArg_of(month_end));
    JsArray_DateArg days = eachDayOfInterval(span);
    uint32_t count = JsArray_DateArg_length(days);
    for (uint32_t i = 0; i < count; i++) {
        Date d = JsArray_DateArg_get(days, i).as.Date;
        bool is_ref = isSameDay_with_Date_and_Date(d, ref_day);
        bool is_wknd = isWeekend(d);
        const char* cls = is_ref ? "day-cell today" : (is_wknd ? "day-cell weekend" : "day-cell");

        HTMLElement cell = make_el(doc, "div", cls, 0);
        HTMLElement num = make_el(doc, "span", "day-num", 0);
        HTMLElement_set_textContent(num, formatDate_with_Date(d, "d"));
        HTMLElement_appendChild(cell, num);

        if (is_ref) {
            HTMLElement_appendChild(cell, make_el(doc, "span", "sub", "REF"));
        } else {
            HTMLElement sub = make_el(doc, "span", "sub", 0);
            HTMLElement_set_textContent(sub, formatDate_with_Date(d, "EEE"));
            HTMLElement_appendChild(cell, sub);
        }
        HTMLElement_appendChild(grid, cell);
    }

    HTMLElement_appendChild(card, grid);
    HTMLElement_appendChild(app, card);
    js_scope_close();
}
