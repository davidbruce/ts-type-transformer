// date-fns from C, in Node: dates made, moved, compared and formatted.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A date alone, formatted.
static bool fmt(Date d, const char* pattern, const char* expected) {
    JsString s = date_fns_formatDate_with_Date(d, pattern, (DateFns_FormatDateOptions){ 0 });
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

__attribute__((export_name("run"))) void run(void) {
    // A Friday, local midnight: what parseISO gives for a date alone.
    Date friday = date_fns_parseISO("2024-03-15");
    CHECK(fmt(friday, "yyyy-MM-dd", "2024-03-15"));
    CHECK(fmt(friday, "EEEE", "Friday"));
    CHECK(date_fns_getYear(friday) == 2024);
    CHECK(!date_fns_isWeekend(friday));
    CHECK(date_fns_getMonth(friday) == 2);
    CHECK(date_fns_getDate(friday) == 15);
    CHECK(date_fns_getDay(friday) == 5);
    CHECK(date_fns_isFriday(friday));
    CHECK(date_fns_isLeapYear(friday));
    CHECK(date_fns_getDaysInMonth(friday) == 31);
    CHECK(date_fns_getQuarter(friday) == 1);

    // Moved by days: a new Date, the old one unchanged.
    DateFns_DateArg day_f = DateFns_DateArg_of_Date(friday);
    Date monday = date_fns_addDays(day_f, 3);
    CHECK(fmt(monday, "yyyy-MM-dd", "2024-03-18"));
    CHECK(fmt(friday, "yyyy-MM-dd", "2024-03-15"));
    Date saturday = date_fns_addDays(day_f, 1);
    CHECK(date_fns_isWeekend(saturday));
    CHECK(date_fns_isSaturday(saturday));
    Date first = date_fns_startOfMonth(day_f);
    CHECK(fmt(first, "d MMM yyyy", "1 Mar 2024"));
    Date last = date_fns_endOfMonth(day_f);
    CHECK(fmt(last, "yyyy-MM-dd HH:mm", "2024-03-31 23:59"));
    Date earlier = date_fns_subDays(day_f, 15);
    CHECK(fmt(earlier, "yyyy-MM-dd", "2024-02-29"));
    Date next_year = date_fns_addYears(day_f, 1);
    CHECK(fmt(next_year, "yyyy-MM-dd", "2025-03-15"));
    Date at_noon = date_fns_setHours(day_f, 12);
    CHECK(fmt(at_noon, "HH:mm", "12:00"));
    CHECK(fmt(date_fns_addHours(DateFns_DateArg_of_Date(at_noon), 5), "HH:mm", "17:00"));

    // Compared: dates, ISO strings and timestamps alike.
    CHECK(date_fns_differenceInDays_with_Date_and_Date(monday, friday) == 3);
    CHECK(date_fns_differenceInDays_with_Date_and_Date(monday, friday, (DateFns_DifferenceInDaysOptions){ 0 }) == 3);
    CHECK(date_fns_differenceInCalendarMonths_with_Date_and_Date(last, earlier, (DateFns_DifferenceInCalendarMonthsOptions){ 0 }) == 1);
    CHECK(date_fns_isAfter_with_Date(monday, "2024-03-15"));
    CHECK(date_fns_isBefore_with_Date_and_Date(friday, monday));
    CHECK(date_fns_isEqual_with_Date_and_Date(friday, friday));
    CHECK(date_fns_compareAsc_with_Date_and_number(friday, Date_time(monday)) == -1);
    CHECK(date_fns_compareDesc_with_Date_and_Date(friday, monday) == 1);
    CHECK(date_fns_isSameDay_with_Date_and_Date(friday, friday, (DateFns_IsSameDayOptions){ 0 }));
    CHECK(!date_fns_isSameMonth_with_Date_and_Date(friday, earlier, (DateFns_IsSameMonthOptions){ 0 }));
    JsString dist = date_fns_formatDistance_with_Date_and_Date(monday, friday, (DateFns_FormatDistanceOptions){ 0 });
    CHECK(JsString_equals(dist, "3 days"));
    js_release(dist);
    JsArray_DateFns_DateArg two = JsArray_DateFns_DateArg_new();
    JsArray_DateFns_DateArg_push(two, day_f);
    JsArray_DateFns_DateArg_push(two, DateFns_DateArg_of_Date(monday));
    Date latest = date_fns_max(two, (DateFns_MaxOptions){ 0 });
    CHECK(Date_time(latest) == Date_time(monday));
    Date soonest = date_fns_min(two, (DateFns_MinOptions){ 0 });
    CHECK(Date_time(soonest) == Date_time(friday));
    js_release(two);

    // An interval, an options object made here, as a duration.
    DateFns_Interval span = DateFns_Interval_create();
    DateFns_Interval_set_start(span, day_f);
    DateFns_Interval_set_end(span, DateFns_DateArg_of_Date(date_fns_addDays(day_f, 10)));
    DateFns_Duration length = date_fns_intervalToDuration(span);
    CHECK(DateFns_Duration_get_days(length).has && DateFns_Duration_get_days(length).value == 10);
    JsString words = date_fns_formatDuration(length);
    CHECK(JsString_equals(words, "10 days"));
    js_release(words);

    // A duration made here, added.
    DateFns_Duration later = DateFns_Duration_create();
    DateFns_Duration_set_months(later, js_some_double(1));
    DateFns_Duration_set_days(later, js_some_double(2));
    Date moved = date_fns_add(day_f, later);
    CHECK(fmt(moved, "yyyy-MM-dd", "2024-04-17"));

    // An options object: a locale-free format option, week starting on Monday.
    DateFns_GetWeekOptions wo = DateFns_GetWeekOptions_create();
    DateFns_WeekOptions_set_weekStartsOn(wo, js_some_double(1));
    CHECK(date_fns_getWeek(friday, wo) == 11);

    // Days in an interval, as an array.
    JsArray_DateFns_DateArg days = date_fns_eachDayOfInterval(span);
    CHECK(JsArray_DateFns_DateArg_length(days) == 11);
    js_release(days);

    // Nonsense is no date.
    CHECK(!date_fns_isValid((JsValue){ date_fns_parseISO("not a date").h }));
    CHECK(date_fns_isValid((JsValue){ friday.h }));
}
