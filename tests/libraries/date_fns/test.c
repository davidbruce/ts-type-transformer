// date-fns from C, in Node: dates made, moved, compared and formatted.
// Every CHECK must hold.
#include "bindings.h"

TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

// A date alone, formatted.
static bool fmt(Date d, const char* pattern, const char* expected) {
    JsString s = formatDate_with_Date(d, pattern, (FormatDateOptions){ 0 });
    bool ok = JsString_equals(s, expected);
    js_release(s);
    return ok;
}

__attribute__((export_name("run"))) void run(void) {
    // A Friday, local midnight: what parseISO gives for a date alone.
    Date friday = parseISO("2024-03-15");
    CHECK(fmt(friday, "yyyy-MM-dd", "2024-03-15"));
    CHECK(fmt(friday, "EEEE", "Friday"));
    CHECK(getYear(friday) == 2024);
    CHECK(!isWeekend(friday));
    CHECK(getMonth(friday) == 2);
    CHECK(getDate(friday) == 15);
    CHECK(getDay(friday) == 5);
    CHECK(isFriday(friday));
    CHECK(isLeapYear(friday));
    CHECK(getDaysInMonth(friday) == 31);
    CHECK(getQuarter(friday) == 1);

    // Moved by days: a new Date, the old one unchanged.
    DateArg day_f = DateArg_of_Date(friday);
    Date monday = addDays(day_f, 3);
    CHECK(fmt(monday, "yyyy-MM-dd", "2024-03-18"));
    CHECK(fmt(friday, "yyyy-MM-dd", "2024-03-15"));
    Date saturday = addDays(day_f, 1);
    CHECK(isWeekend(saturday));
    CHECK(isSaturday(saturday));
    Date first = startOfMonth(day_f);
    CHECK(fmt(first, "d MMM yyyy", "1 Mar 2024"));
    Date last = endOfMonth(day_f);
    CHECK(fmt(last, "yyyy-MM-dd HH:mm", "2024-03-31 23:59"));
    Date earlier = subDays(day_f, 15);
    CHECK(fmt(earlier, "yyyy-MM-dd", "2024-02-29"));
    Date next_year = addYears(day_f, 1);
    CHECK(fmt(next_year, "yyyy-MM-dd", "2025-03-15"));
    Date at_noon = setHours(day_f, 12);
    CHECK(fmt(at_noon, "HH:mm", "12:00"));
    CHECK(fmt(addHours(DateArg_of_Date(at_noon), 5), "HH:mm", "17:00"));

    // Compared: dates, ISO strings and timestamps alike.
    CHECK(differenceInDays_with_Date_and_Date(monday, friday) == 3);
    CHECK(differenceInDays_with_Date_and_Date(monday, friday, (DifferenceInDaysOptions){ 0 }) == 3);
    CHECK(differenceInCalendarMonths_with_Date_and_Date(last, earlier, (DifferenceInCalendarMonthsOptions){ 0 }) == 1);
    CHECK(isAfter_with_Date(monday, "2024-03-15"));
    CHECK(isBefore_with_Date_and_Date(friday, monday));
    CHECK(isEqual_with_Date_and_Date(friday, friday));
    CHECK(compareAsc_with_Date_and_number(friday, Date_time(monday)) == -1);
    CHECK(compareDesc_with_Date_and_Date(friday, monday) == 1);
    CHECK(isSameDay_with_Date_and_Date(friday, friday, (IsSameDayOptions){ 0 }));
    CHECK(!isSameMonth_with_Date_and_Date(friday, earlier, (IsSameMonthOptions){ 0 }));
    JsString dist = formatDistance_with_Date_and_Date(monday, friday, (FormatDistanceOptions){ 0 });
    CHECK(JsString_equals(dist, "3 days"));
    js_release(dist);
    JsArray_DateArg two = JsArray_DateArg_new();
    JsArray_DateArg_push(two, day_f);
    JsArray_DateArg_push(two, DateArg_of_Date(monday));
    Date latest = max(two, (MaxOptions){ 0 });
    CHECK(Date_time(latest) == Date_time(monday));
    Date soonest = min(two, (MinOptions){ 0 });
    CHECK(Date_time(soonest) == Date_time(friday));
    js_release(two);

    // An interval, an options object made here, as a duration.
    Interval span = Interval_create();
    Interval_set_start(span, day_f);
    Interval_set_end(span, DateArg_of_Date(addDays(day_f, 10)));
    Duration length = intervalToDuration(span);
    CHECK(Duration_get_days(length).has && Duration_get_days(length).value == 10);
    JsString words = formatDuration(length);
    CHECK(JsString_equals(words, "10 days"));
    js_release(words);

    // A duration made here, added.
    Duration later = Duration_create();
    Duration_set_months(later, js_some_double(1));
    Duration_set_days(later, js_some_double(2));
    Date moved = add(day_f, later);
    CHECK(fmt(moved, "yyyy-MM-dd", "2024-04-17"));

    // An options object: a locale-free format option, week starting on Monday.
    GetWeekOptions wo = GetWeekOptions_create();
    WeekOptions_set_weekStartsOn(wo, js_some_double(1));
    CHECK(getWeek(friday, wo) == 11);

    // Days in an interval, as an array.
    JsArray_DateArg days = eachDayOfInterval(span);
    CHECK(JsArray_DateArg_length(days) == 11);
    js_release(days);

    // Nonsense is no date.
    CHECK(!isValid((JsValue){ parseISO("not a date").h }));
    CHECK(isValid((JsValue){ friday.h }));
}
