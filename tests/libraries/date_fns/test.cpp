// date-fns from C++, in Node: dates made, moved, compared, formatted, parsed.
// Every CHECK must hold.
#include "bindings.hpp"

extern "C" TS_IMPORT(check) void check(int ok, int line);
#define CHECK(c) check((c), __LINE__)

static std::string ymd(const Date& d) { return formatDate(d, "yyyy-MM-dd"); }

extern "C" __attribute__((export_name("run"))) void run() {
    // A Friday, local midnight: what parseISO gives for a date alone.
    Date friday = parseISO("2024-03-15");
    CHECK(ymd(friday) == "2024-03-15");
    CHECK(formatDate(friday, "EEEE") == "Friday");
    CHECK(formatDate(friday, "EEE, d MMM yyyy") == "Fri, 15 Mar 2024");
    CHECK(getYear(friday) == 2024);
    CHECK(getMonth(friday) == 2);
    CHECK(getDay(friday) == 5);
    CHECK(getDaysInMonth(friday) == 31);
    CHECK(!isWeekend(friday));
    CHECK(isLeapYear(friday));
    CHECK(!isLeapYear(parseISO("2023-01-01")));

    // Moved by days: a new Date, the old one unchanged.
    Date monday = addDays(friday, 3);
    CHECK(ymd(monday) == "2024-03-18");
    CHECK(ymd(friday) == "2024-03-15");
    CHECK(isWeekend(addDays(friday, 1)));
    CHECK(ymd(subDays(friday, 15)) == "2024-02-29");
    CHECK(ymd(addMonths(friday, 1)) == "2024-04-15");
    CHECK(ymd(addMonths(parseISO("2024-01-31"), 1)) == "2024-02-29");
    Date first = startOfMonth(friday);
    CHECK(formatDate(first, "d MMM yyyy") == "1 Mar 2024");
    CHECK(ymd(endOfMonth(friday)) == "2024-03-31");
    CHECK(formatDate(endOfDay(friday), "HH:mm:ss") == "23:59:59");
    CHECK(ymd(startOfWeek(friday)) == "2024-03-10");

    // A date by number, or by string, where a DateArg is wanted.
    CHECK(ymd(addDays(friday.time(), 1)) == "2024-03-16");
    CHECK(ymd(addDays(std::string("2024-03-15"), 1)) == "2024-03-16");

    // Compared: dates, ISO strings and timestamps alike.
    CHECK(differenceInDays(monday, friday) == 3);
    CHECK(differenceInMonths(DateArg(parseISO("2024-06-15")), friday) == 3);
    CHECK(differenceInHours(addHours(friday, 5), friday) == 5);
    CHECK(isAfter(monday, std::string("2024-03-15")));
    CHECK(isBefore(friday, monday));
    CHECK(isEqual(friday, friday.time()));
    CHECK(isSameDay(friday, endOfDay(friday)));
    CHECK(!isSameDay(friday, monday));
    CHECK(compareAsc(friday, monday.time()) == -1);
    CHECK(compareAsc(monday, friday) == 1);
    CHECK(formatDistance(monday, friday) == "3 days");
    FormatDistanceOptions suffix = FormatDistanceOptions::create();
    suffix.addSuffix(true);
    CHECK(suffix.addSuffix() == true);
    CHECK(formatDistance(friday, monday, suffix) == "3 days ago");
    CHECK(formatDistanceStrict(monday, friday) == "3 days");

    // max/min of an array of dates.
    Date latest = max(ts::Array<DateArg>{friday, monday});
    CHECK(latest.time() == monday.time());
    Date earliest = min(ts::Array<DateArg>{friday, monday});
    CHECK(earliest.time() == friday.time());

    // An interval, an object made here: as a duration, a list of days, a range.
    Interval span = Interval::create();
    span.start(friday);
    span.end(addDays(friday, 10));
    Duration length = intervalToDuration(span);
    CHECK(length.days() == 10.0);
    CHECK(formatDuration(length) == "10 days");
    CHECK(isWithinInterval(addDays(friday, 4), span));
    CHECK(!isWithinInterval(addDays(friday, 11), span));

    Interval week = Interval::create();
    week.start(friday);
    week.end(monday);
    ts::Array<DateArg> days = eachDayOfInterval(week);
    CHECK(days.size() == 4);
    DateArg last = days[3];
    CHECK(std::holds_alternative<Date>(last) && ymd(std::get<Date>(last)) == "2024-03-18");

    Interval other = Interval::create();
    other.start(addDays(friday, 5));
    other.end(addDays(friday, 20));
    CHECK(areIntervalsOverlapping(span, other));

    // A duration made here, added.
    Duration later = Duration::create();
    later.months(1.0);
    later.days(2.0);
    CHECK(!later.years());
    Date moved = add(friday, later);
    CHECK(ymd(moved) == "2024-04-17");
    CHECK(formatDuration(later) == "1 month 2 days");
    CHECK(formatISODuration(later) == "P0Y1M2DT0H0M0S");

    // Duration options: a delimiter.
    FormatDurationOptions dash = FormatDurationOptions::create();
    dash.delimiter(", ");
    CHECK(formatDuration(later, dash) == "1 month, 2 days");

    // Set some fields.
    DateValues values = DateValues::create();
    values.year(2020);
    values.month(1);
    values.date(29);
    Date fixed = set(friday, values);
    CHECK(ymd(fixed) == "2020-02-29");

    // ISO and custom formats out, parsed back in.
    CHECK(lightFormat(friday, "yyyy-MM-dd") == "2024-03-15");
    CHECK(formatISO(friday, FormatISOOptions::create()).substr(0, 10) == "2024-03-15");
    Date parsed = parse("03/15/2024 14:30", "MM/dd/yyyy HH:mm", Date::from_time(0));
    CHECK(formatDate(parsed, "yyyy-MM-dd'T'HH:mm") == "2024-03-15T14:30");
    CHECK(isMatch("2024-03-15", "yyyy-MM-dd"));
    CHECK(!isMatch("15 March", "yyyy-MM-dd"));

    // Unix time both ways; a Date made from milliseconds.
    Date epoch = fromUnixTime(86400);
    CHECK(epoch.time() == 86400000.0);
    CHECK(getUnixTime(epoch) == 86400);
    CHECK(Date::from_time(1000).time() == 1000.0);
    CHECK(Date::from_time(0).to_iso_string() == "1970-01-01T00:00:00.000Z");

    // closestTo returns an optional date.
    std::optional<Date> near = closestTo(friday, ts::Array<DateArg>{monday, addDays(friday, 30)});
    CHECK(near.has_value() && near->time() == monday.time());

    // Nonsense is no date.
    // `isValid(date: any)` takes a ts::Value, which a Date does not convert to
    // (FINDING 6): rewrap the handle.
    auto any = [](const Date& d) {
        ts::Value v;
        static_cast<ts::Handle&>(v) = d;
        return v;
    };
    CHECK(!isValid(any(parseISO("not a date"))));
    CHECK(isValid(any(friday)));
    CHECK(!isValid(ts::Value::number(NAN)));
    CHECK(isValid(ts::Value::number(5)));
}
