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

static uint32_t append_i32(char* dst, uint32_t pos, uint32_t cap, int32_t v) {
    char tmp[16];
    uint32_t len = 0;
    if (v < 0) {
        if (pos + 1 < cap) dst[pos++] = '-';
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
    for (uint32_t i = 0; i < len && pos + 1 < cap; i++) dst[pos++] = tmp[len - 1 - i];
    dst[pos] = '\0';
    return pos;
}

static void add_metric(Document doc, HTMLElement metrics, const char* label, const char* val) {
    HTMLElement m = make_el(doc, "div", "metric", 0);
    append_and_release(m, make_el(doc, "div", "label", label));
    append_and_release(m, make_el(doc, "div", "val", val));
    append_and_release(metrics, m);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Date-fns Calendar Grid (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "March 2024"));
    append_and_release(app, head);

    Date ref_day = parseISO("2024-03-15");
    DateArg ref_arg = DateArg_of_Date(ref_day);
    Date month_start = startOfMonth(ref_arg);
    Date month_end = endOfMonth(ref_arg);
    int32_t offset = (int32_t)getDay(month_start);

    char b_days[16]; b_days[0] = '\0';
    append_i32(b_days, 0, sizeof(b_days), (int32_t)getDaysInMonth(ref_day));

    char b_q[16];
    uint32_t qp = append_cstr(b_q, 0, sizeof(b_q), "Q");
    append_i32(b_q, qp, sizeof(b_q), (int32_t)getQuarter(ref_day));

    char b_w[16];
    uint32_t wp = append_cstr(b_w, 0, sizeof(b_w), "W");
    append_i32(b_w, wp, sizeof(b_w), (int32_t)getISOWeek_with_Date(ref_day, (GetISOWeekOptions){ 0 }));

    char b_doy[32];
    uint32_t dp = append_cstr(b_doy, 0, sizeof(b_doy), isLeapYear(ref_day) ? "Yes \xC2\xB7 #" : "No \xC2\xB7 #");
    append_i32(b_doy, dp, sizeof(b_doy), (int32_t)getDayOfYear_with_Date(ref_day, (GetDayOfYearOptions){ 0 }));

    HTMLElement metrics = make_el(doc, "div", "metrics", 0);
    add_metric(doc, metrics, "Days in Month", b_days);
    add_metric(doc, metrics, "Quarter", b_q);
    add_metric(doc, metrics, "ISO Week", b_w);
    add_metric(doc, metrics, "Leap Year / DOY", b_doy);
    append_and_release(app, metrics);

    HTMLElement card = make_el(doc, "div", "card", 0);
    append_and_release(card, make_el(doc, "div", "label", "March 2024 \xE2\x80\x94 Reference 2024-03-15"));

    HTMLElement grid = make_el(doc, "div", "cal-grid", 0);
    const char* dows[] = { "Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat" };
    for (int i = 0; i < 7; i++) {
        append_and_release(grid, make_el(doc, "div", "dow", dows[i]));
    }
    for (int32_t i = 0; i < offset; i++) {
        append_and_release(grid, make_el(doc, "div", "day-cell empty", 0));
    }

    Interval span = Interval_create();
    Interval_set_start(span, DateArg_of_Date(month_start));
    Interval_set_end(span, DateArg_of_Date(month_end));
    JsArray_DateArg days = eachDayOfInterval(span);
    uint32_t count = JsArray_DateArg_length(days);
    for (uint32_t i = 0; i < count; i++) {
        Date d = JsArray_DateArg_get(days, i).as.Date;
        bool is_ref = isSameDay_with_Date_and_Date(d, ref_day, (IsSameDayOptions){ 0 });
        bool is_wknd = isWeekend(d);
        const char* cls = is_ref ? "day-cell today" : (is_wknd ? "day-cell weekend" : "day-cell");

        HTMLElement cell = make_el(doc, "div", cls, 0);
        JsString day_str = formatDate_with_Date(d, "d", (FormatDateOptions){ 0 });
        char day_buf[8];
        JsString_copy(day_str, day_buf, sizeof(day_buf));
        js_release(day_str);
        append_and_release(cell, make_el(doc, "span", "day-num", day_buf));

        if (is_ref) {
            append_and_release(cell, make_el(doc, "span", "sub", "REF"));
        } else {
            JsString sub_str = formatDate_with_Date(d, "EEE", (FormatDateOptions){ 0 });
            char sub_buf[16];
            JsString_copy(sub_str, sub_buf, sizeof(sub_buf));
            js_release(sub_str);
            append_and_release(cell, make_el(doc, "span", "sub", sub_buf));
        }
        append_and_release(grid, cell);
        js_release(d);
    }

    append_and_release(card, grid);
    append_and_release(app, card);

    js_release(days);
    js_release(span);
    js_release(month_end);
    js_release(month_start);
    js_release(ref_day);
    js_release(app);
    js_release(doc);
}
