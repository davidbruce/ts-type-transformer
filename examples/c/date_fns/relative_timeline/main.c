#include "dom/dom.h"
#include "date_fns/date_fns.h"
#include <stdio.h>

static HTMLElement make_el(Document doc, const char* tag, const char* cls, const char* text) {
    HTMLElement el = Document_createElement(doc, tag);
    if (cls && cls[0]) HTMLElement_set_className(el, cls);
    if (text) HTMLElement_set_textContent(el, text);
    return el;
}

__attribute__((export_name("start"))) void start(void) {
    js_scope();
    Document doc = get_document();
    HTMLElement app = Document_getElementById(doc, "app");

    HTMLElement head = make_el(doc, "div", "head", 0);
    HTMLElement_appendChild(head, make_el(doc, "h1", 0, "Date-fns Relative Timeline (C)"));
    HTMLElement_appendChild(head, make_el(doc, "span", "badge", "Anchor 2024-03-15"));
    HTMLElement_appendChild(app, head);

    Date anchor = df_parseISO("2024-03-15");
    Df_FormatDistanceOptions opts = Df_FormatDistanceOptions_create(.addSuffix = true);

    const char* isos[6] = {
        "2024-01-10", "2024-02-29", "2024-03-12",
        "2024-03-15", "2024-03-18", "2024-06-15",
    };
    const char* titles[6] = {
        "Spec Draft", "Leap Day Freeze", "Beta Tag",
        "Release Anchor", "Patch Window", "Q2 Audit",
    };

    HTMLElement tl_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(tl_card, make_el(doc, "div", "label", "Milestones Relative to 2024-03-15"));
    HTMLElement timeline = make_el(doc, "div", "timeline", 0);

    JsArray_Df_DateArg all_dates = JsArray_Df_DateArg_new();
    for (int i = 0; i < 6; i++) {
        Date d = df_parseISO(isos[i]);
        JsArray_Df_DateArg_push(all_dates, d);

        bool same = df_isSameDay_with_Date_and_Date(d, anchor);
        HTMLElement row = make_el(doc, "div", same ? "milestone anchor" : "milestone", 0);

        HTMLElement date_el = make_el(doc, "span", "muted", 0);
        HTMLElement_set_textContent(date_el, df_formatDate_with_Date(d, "yyyy-MM-dd (EEE)"));
        HTMLElement_appendChild(row, date_el);
        HTMLElement_appendChild(row, make_el(doc, "span", "title", titles[i]));

        if (same) {
            HTMLElement_appendChild(row, make_el(doc, "span", 0, "today (anchor)"));
        } else {
            HTMLElement rel_el = make_el(doc, "span", 0, 0);
            HTMLElement_set_textContent(rel_el, df_formatDistance_with_Date_and_Date(d, anchor, opts));
            HTMLElement_appendChild(row, rel_el);
        }

        char dist_buf[64], strict_buf[96];
        JsString_copy(df_formatDistanceStrict_with_Date_and_Date(d, anchor), dist_buf, sizeof dist_buf);
        snprintf(strict_buf, sizeof strict_buf, "%s \xC2\xB7 cmp %d", dist_buf, (int)df_compareAsc_with_Date_and_Date(d, anchor));
        HTMLElement_appendChild(row, make_el(doc, "span", "muted", strict_buf));

        const char* status = same ? "ANCHOR" : (df_isBefore_with_Date_and_Date(d, anchor) ? "PAST" : (df_isAfter_with_Date_and_Date(d, anchor) ? "FUTURE" : "OTHER"));
        HTMLElement_appendChild(row, make_el(doc, "span", "pill", status));
        HTMLElement_appendChild(timeline, row);
    }
    HTMLElement_appendChild(tl_card, timeline);
    HTMLElement_appendChild(app, tl_card);

    Date earliest = df_min(all_dates);
    Date latest = df_max(all_dates);
    int span_days = (int)df_differenceInCalendarDays_with_Date_and_Date(latest, earliest);

    char e_buf[32], l_buf[32], s1[64], s2[64], s3[64];
    JsString_copy(df_formatDate_with_Date(earliest, "yyyy-MM-dd"), e_buf, sizeof e_buf);
    JsString_copy(df_formatDate_with_Date(latest, "yyyy-MM-dd"), l_buf, sizeof l_buf);
    snprintf(s1, sizeof s1, "Earliest: %s", e_buf);
    snprintf(s2, sizeof s2, "Latest: %s", l_buf);
    snprintf(s3, sizeof s3, "Total Span: %d days", span_days);

    HTMLElement sum_card = make_el(doc, "div", "card", 0);
    HTMLElement_appendChild(sum_card, make_el(doc, "div", "label", "Extremes & Total Span"));
    HTMLElement summary = make_el(doc, "div", "summary", 0);
    HTMLElement_appendChild(summary, make_el(doc, "span", 0, s1));
    HTMLElement_appendChild(summary, make_el(doc, "span", 0, s2));
    HTMLElement_appendChild(summary, make_el(doc, "span", 0, s3));
    HTMLElement_appendChild(sum_card, summary);
    HTMLElement_appendChild(app, sum_card);
}
