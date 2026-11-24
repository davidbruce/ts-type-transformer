#include "dom/dom.h"
#include "date_fns/date_fns.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

static void add_op_row(Document doc, HTMLElement tbody, const char* op_name, Date res, Date base_d, const char* fmt) {
    int dd = (int)df_differenceInCalendarDays(res, base_d);
    int dh = (int)df_differenceInHours(res, base_d);
    char dd_buf[24], dh_buf[24];
    snprintf(dd_buf, sizeof dd_buf, dd == 0 ? "0" : "%+d", dd);
    snprintf(dh_buf, sizeof dh_buf, dh == 0 ? "0" : "%+d", dh);

    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_op = make_el(doc, "td", 0, 0);
    HTMLElement_appendChild(td_op, make_el(doc, "code", 0, op_name));
    HTMLElement_appendChild(tr, td_op);

    HTMLElement td_out = make_el(doc, "td", 0, 0);
    HTMLElement_set_textContent(td_out, df_formatDate(res, fmt));
    HTMLElement_appendChild(tr, td_out);
    HTMLElement_appendChild(tr, make_el(doc, "td", 0, dd_buf));
    HTMLElement_appendChild(tr, make_el(doc, "td", 0, dh_buf));
    HTMLElement_appendChild(tbody, tr);
}

static void add_kv(Document doc, HTMLElement row, const char* label, const char* val) {
    HTMLElement box = make_el(doc, "div", "kv", 0);
    HTMLElement_appendChild(box, make_el(doc, "span", "label", label));
    HTMLElement_appendChild(box, make_el(doc, "strong", 0, val));
    HTMLElement_appendChild(row, box);
}

static void add_kv_js(Document doc, HTMLElement row, const char* label, JsString val) {
    HTMLElement box = make_el(doc, "div", "kv", 0);
    HTMLElement_appendChild(box, make_el(doc, "span", "label", label));
    HTMLElement strong = make_el(doc, "strong", 0, 0);
    HTMLElement_set_textContent(strong, val);
    HTMLElement_appendChild(box, strong);
    HTMLElement_appendChild(row, box);
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Date-fns Arithmetic & Durations (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "Base 2024-03-15"));
    HTMLElement_appendChild(app, head);

    Date base_d = df_parseISO("2024-03-15");
    Date jan31 = df_parseISO("2024-01-31");

    Df_Duration dur = Df_Duration_create(.months = 1, .days = 2);

    HTMLElement card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(card, make_el(doc, "div", "label", "Immutable Date Stepping"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Operation"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "Result"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "\xCE\x94 Days (vs base)"));
    HTMLElement_appendChild(hrow, make_el(doc, "th", 0, "\xCE\x94 Hours"));
    HTMLElement_appendChild(thead, hrow);
    HTMLElement_appendChild(tbl, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    add_op_row(doc, tbody, "addDays(base, 3)", df_addDays(base_d, 3.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "subDays(base, 15)", df_subDays(base_d, 15.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "addWeeks(base, 2)", df_addWeeks(base_d, 2.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "addMonths(jan31, 1)", df_addMonths(jan31, 1.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "subMonths(base, 3)", df_subMonths(base_d, 3.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "addYears(base, 1)", df_addYears(base_d, 1.0), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "startOfWeek(base)", df_startOfWeek(base_d), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "endOfMonth(base)", df_endOfMonth(base_d), base_d, "yyyy-MM-dd HH:mm:ss");
    add_op_row(doc, tbody, "add(base, { months: 1, days: 2 })", df_add(base_d, dur), base_d, "yyyy-MM-dd");
    add_op_row(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd");
    HTMLElement_appendChild(tbl, tbody);
    HTMLElement_appendChild(card, tbl);
    HTMLElement_appendChild(app, card);

    HTMLElement footer = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(footer, make_el(doc, "div", "label", "Duration & Unit Conversions"));
    HTMLElement srow = make_el(doc, "div", "summary-row", 0);

    add_kv_js(doc, srow, "formatDuration(dur)", df_formatDuration(dur));
    add_kv_js(doc, srow, "formatISODuration(dur)", df_formatISODuration(dur));

    char min_buf[32], wk_buf[32];
    snprintf(min_buf, sizeof min_buf, "%d min", (int)df_hoursToMinutes(2.0));
    snprintf(wk_buf, sizeof wk_buf, "%d wk", (int)df_daysToWeeks(14.0));
    add_kv(doc, srow, "hoursToMinutes(2)", min_buf);
    add_kv(doc, srow, "daysToWeeks(14)", wk_buf);

    HTMLElement_appendChild(footer, srow);
    HTMLElement_appendChild(app, footer);
}
