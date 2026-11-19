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

static uint32_t append_cstr(char* dst, uint32_t pos, uint32_t cap, const char* src) {
    for (uint32_t i = 0; src[i] && pos + 1 < cap; i++) dst[pos++] = src[i];
    dst[pos] = '\0';
    return pos;
}

static void format_signed(int32_t v, char* buf, uint32_t cap) {
    char tmp[16];
    uint32_t len = 0;
    uint32_t pos = 0;
    if (v > 0 && pos + 1 < cap) buf[pos++] = '+';
    else if (v < 0 && pos + 1 < cap) {
        buf[pos++] = '-';
        v = -v;
    }
    if (v == 0) {
        tmp[len++] = '0';
    } else {
        while (v > 0 && len < sizeof(tmp)) {
            tmp[len++] = (char)('0' + (v % 10));
            v /= 10;
        }
    }
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) buf[pos++] = tmp[len - 1 - i];
    buf[pos] = '\0';
}

static void add_op_row(Document doc, HTMLElement tbody, const char* op_name, Date res, Date base_d, const char* fmt, bool release_res) {
    static char out_buf[64];
    JsString s = formatDate_with_Date(res, fmt, (FormatDateOptions){ 0 });
    uint32_t n = JsString_copy(s, out_buf, sizeof(out_buf) - 1);
    out_buf[n] = '\0';
    js_release(s);

    int32_t dd = (int32_t)differenceInCalendarDays_with_Date_and_Date(res, base_d, (DifferenceInCalendarDaysOptions){ 0 });
    int32_t dh = (int32_t)differenceInHours_with_Date_and_Date(res, base_d, (DifferenceInHoursOptions){ 0 });
    char dd_buf[24];
    char dh_buf[24];
    format_signed(dd, dd_buf, sizeof(dd_buf));
    format_signed(dh, dh_buf, sizeof(dh_buf));

    HTMLElement tr = make_el(doc, "tr", 0, 0);
    HTMLElement td_op = make_el(doc, "td", 0, 0);
    append_and_release(td_op, make_el(doc, "code", 0, op_name));
    append_and_release(tr, td_op);
    append_and_release(tr, make_el(doc, "td", 0, out_buf));
    append_and_release(tr, make_el(doc, "td", 0, dd_buf));
    append_and_release(tr, make_el(doc, "td", 0, dh_buf));
    append_and_release(tbody, tr);

    if (release_res) js_release(res);
}

static void add_kv(Document doc, HTMLElement row, const char* label, const char* val) {
    HTMLElement box = make_el(doc, "div", "kv", 0);
    append_and_release(box, make_el(doc, "span", "label", label));
    append_and_release(box, make_el(doc, "strong", 0, val));
    append_and_release(row, box);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Date-fns Arithmetic & Durations (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "Base 2024-03-15"));
    append_and_release(app, head);

    Date base_d = parseISO("2024-03-15");
    Date jan31 = parseISO("2024-01-31");
    DateArg day_base = DateArg_of_Date(base_d);
    DateArg day_jan = DateArg_of_Date(jan31);

    Duration dur = Duration_create();
    Duration_set_months(dur, js_some_double(1));
    Duration_set_days(dur, js_some_double(2));

    HTMLElement card = make_el(doc, "div", "card", 0);
    append_and_release(card, make_el(doc, "div", "label", "Immutable Date Stepping"));

    HTMLElement tbl = make_el(doc, "table", 0, 0);
    HTMLElement thead = make_el(doc, "thead", 0, 0);
    HTMLElement hrow = make_el(doc, "tr", 0, 0);
    append_and_release(hrow, make_el(doc, "th", 0, "Operation"));
    append_and_release(hrow, make_el(doc, "th", 0, "Result"));
    append_and_release(hrow, make_el(doc, "th", 0, "\xCE\x94 Days (vs base)"));
    append_and_release(hrow, make_el(doc, "th", 0, "\xCE\x94 Hours"));
    append_and_release(thead, hrow);
    append_and_release(tbl, thead);

    HTMLElement tbody = make_el(doc, "tbody", 0, 0);
    add_op_row(doc, tbody, "addDays(base, 3)", addDays(day_base, 3.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "subDays(base, 15)", subDays(day_base, 15.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "addWeeks(base, 2)", addWeeks(day_base, 2.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "addMonths(jan31, 1)", addMonths(day_jan, 1.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "subMonths(base, 3)", subMonths(day_base, 3.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "addYears(base, 1)", addYears(day_base, 1.0), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "startOfWeek(base)", startOfWeek(day_base), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "endOfMonth(base)", endOfMonth(day_base), base_d, "yyyy-MM-dd HH:mm:ss", true);
    add_op_row(doc, tbody, "add(base, { months: 1, days: 2 })", add(day_base, dur), base_d, "yyyy-MM-dd", true);
    add_op_row(doc, tbody, "base (immutable check)", base_d, base_d, "yyyy-MM-dd", false);
    append_and_release(tbl, tbody);
    append_and_release(card, tbl);
    append_and_release(app, card);

    HTMLElement footer = make_el(doc, "div", "card", 0);
    append_and_release(footer, make_el(doc, "div", "label", "Duration & Unit Conversions"));
    HTMLElement srow = make_el(doc, "div", "summary-row", 0);

    char dur_buf[64];
    JsString d_words = formatDuration(dur);
    uint32_t dn = JsString_copy(d_words, dur_buf, sizeof(dur_buf) - 1);
    dur_buf[dn] = '\0';
    js_release(d_words);
    add_kv(doc, srow, "formatDuration(dur)", dur_buf);

    char iso_buf[64];
    JsString d_iso = formatISODuration(dur);
    uint32_t in_len = JsString_copy(d_iso, iso_buf, sizeof(iso_buf) - 1);
    iso_buf[in_len] = '\0';
    js_release(d_iso);
    add_kv(doc, srow, "formatISODuration(dur)", iso_buf);

    char min_buf[32];
    format_signed((int32_t)hoursToMinutes(2.0), min_buf, sizeof(min_buf));
    if (min_buf[0] == '+') {
        for (uint32_t i = 0; min_buf[i]; i++) min_buf[i] = min_buf[i + 1];
    }
    uint32_t ml = 0;
    while (min_buf[ml]) ml++;
    append_cstr(min_buf, ml, sizeof(min_buf), " min");
    add_kv(doc, srow, "hoursToMinutes(2)", min_buf);

    char wk_buf[32];
    format_signed((int32_t)daysToWeeks(14.0), wk_buf, sizeof(wk_buf));
    if (wk_buf[0] == '+') {
        for (uint32_t i = 0; wk_buf[i]; i++) wk_buf[i] = wk_buf[i + 1];
    }
    uint32_t wl = 0;
    while (wk_buf[wl]) wl++;
    append_cstr(wk_buf, wl, sizeof(wk_buf), " wk");
    add_kv(doc, srow, "daysToWeeks(14)", wk_buf);

    append_and_release(footer, srow);
    append_and_release(app, footer);

    js_release(dur);
    js_release(jan31);
    js_release(base_d);
    js_release(app);
    js_release(doc);
}
