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

static void copy_and_release(JsString s, char* buf, uint32_t cap) {
    uint32_t n = JsString_copy(s, buf, cap - 1);
    buf[n] = '\0';
    js_release(s);
}

__attribute__((export_name("start"))) void start(void) {
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    append_and_release(head, make_el(doc, "h1", 0, "Date-fns Relative Timeline (C)"));
    append_and_release(head, make_el(doc, "span", "badge", "Anchor 2024-03-15"));
    append_and_release(app, head);

    Date anchor = parseISO("2024-03-15");
    FormatDistanceOptions opts = FormatDistanceOptions_create();
    FormatDistanceOptions_set_addSuffix(opts, js_some_bool(true));

    const char* isos[6] = {
        "2024-01-10",
        "2024-02-29",
        "2024-03-12",
        "2024-03-15",
        "2024-03-18",
        "2024-06-15",
    };
    const char* titles[6] = {
        "Spec Draft",
        "Leap Day Freeze",
        "Beta Tag",
        "Release Anchor",
        "Patch Window",
        "Q2 Audit",
    };

    HTMLElement tl_card = make_el(doc, "div", "card", 0);
    append_and_release(tl_card, make_el(doc, "div", "label", "Milestones Relative to 2024-03-15"));
    HTMLElement timeline = make_el(doc, "div", "timeline", 0);

    JsArray_DateArg all_dates = JsArray_DateArg_new();
    for (int i = 0; i < 6; i++) {
        Date d = parseISO(isos[i]);
        JsArray_DateArg_push(all_dates, DateArg_of_Date(d));

        bool same = isSameDay_with_Date_and_Date(d, anchor, (IsSameDayOptions){ 0 });
        HTMLElement row = make_el(doc, "div", same ? "milestone anchor" : "milestone", 0);

        char date_buf[48];
        copy_and_release(formatDate_with_Date(d, "yyyy-MM-dd (EEE)", (FormatDateOptions){ 0 }), date_buf, sizeof(date_buf));
        append_and_release(row, make_el(doc, "span", "muted", date_buf));
        append_and_release(row, make_el(doc, "span", "title", titles[i]));

        if (same) {
            append_and_release(row, make_el(doc, "span", 0, "today (anchor)"));
        } else {
            char rel_buf[64];
            copy_and_release(formatDistance_with_Date_and_Date(d, anchor, opts), rel_buf, sizeof(rel_buf));
            append_and_release(row, make_el(doc, "span", 0, rel_buf));
        }

        char strict_buf[96];
        copy_and_release(formatDistanceStrict_with_Date_and_Date(d, anchor, (FormatDistanceStrictOptions){ 0 }), strict_buf, sizeof(strict_buf));
        int32_t cmp = (int32_t)compareAsc_with_Date_and_Date(d, anchor);
        uint32_t sl = 0;
        while (strict_buf[sl]) sl++;
        sl = append_cstr(strict_buf, sl, sizeof(strict_buf), " \xC2\xB7 cmp ");
        append_i32(strict_buf, sl, sizeof(strict_buf), cmp);
        append_and_release(row, make_el(doc, "span", "muted", strict_buf));

        const char* status = same ? "ANCHOR" : (isBefore_with_Date_and_Date(d, anchor) ? "PAST" : (isAfter_with_Date_and_Date(d, anchor) ? "FUTURE" : "OTHER"));
        append_and_release(row, make_el(doc, "span", "pill", status));

        append_and_release(timeline, row);
        js_release(d);
    }
    append_and_release(tl_card, timeline);
    append_and_release(app, tl_card);

    Date earliest = min(all_dates, (MinOptions){ 0 });
    Date latest = max(all_dates, (MaxOptions){ 0 });
    int32_t span_days = (int32_t)differenceInCalendarDays_with_Date_and_Date(latest, earliest, (DifferenceInCalendarDaysOptions){ 0 });

    char e_buf[32], l_buf[32];
    copy_and_release(formatDate_with_Date(earliest, "yyyy-MM-dd", (FormatDateOptions){ 0 }), e_buf, sizeof(e_buf));
    copy_and_release(formatDate_with_Date(latest, "yyyy-MM-dd", (FormatDateOptions){ 0 }), l_buf, sizeof(l_buf));

    char s1[64], s2[64], s3[64];
    uint32_t p1 = append_cstr(s1, 0, sizeof(s1), "Earliest: ");
    append_cstr(s1, p1, sizeof(s1), e_buf);
    uint32_t p2 = append_cstr(s2, 0, sizeof(s2), "Latest: ");
    append_cstr(s2, p2, sizeof(s2), l_buf);
    uint32_t p3 = append_cstr(s3, 0, sizeof(s3), "Total Span: ");
    p3 = append_i32(s3, p3, sizeof(s3), span_days);
    append_cstr(s3, p3, sizeof(s3), " days");

    HTMLElement sum_card = make_el(doc, "div", "card", 0);
    append_and_release(sum_card, make_el(doc, "div", "label", "Extremes & Total Span"));
    HTMLElement summary = make_el(doc, "div", "summary", 0);
    append_and_release(summary, make_el(doc, "span", 0, s1));
    append_and_release(summary, make_el(doc, "span", 0, s2));
    append_and_release(summary, make_el(doc, "span", 0, s3));
    append_and_release(sum_card, summary);
    append_and_release(app, sum_card);

    js_release(earliest);
    js_release(latest);
    js_release(all_dates);
    js_release(opts);
    js_release(anchor);
    js_release(app);
    js_release(doc);
}
