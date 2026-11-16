// date-fns from Zig, in Node: dates made, moved, compared and formatted. Every expect must hold.
const std = @import("std");
const b = @import("bindings.zig");

extern "env" fn check(ok: i32, line: u32) void;

fn expect(ok: bool, src: std.builtin.SourceLocation) void {
    check(@intFromBool(ok), src.line);
}

// A date where a DateArg is wanted.
fn day(d: b.Date) b.DateArg {
    return .{ .Date = d };
}

// A date as text, in the local time zone (as parseISO reads a bare date).
fn fmt(d: b.Date, pattern: []const u8) b.JsString {
    return b.formatDate(day(d), pattern, .{});
}

fn iso(text: []const u8) b.Date {
    return b.parseISO(text, .{});
}

export fn run() void {
    // A Friday, local midnight: what parseISO gives for a date alone.
    const friday = iso("2024-03-15");
    expect(fmt(friday, "yyyy-MM-dd").eql("2024-03-15"), @src());
    expect(fmt(friday, "EEEE").eql("Friday"), @src());
    expect(fmt(friday, "EEE, d MMM yyyy").eql("Fri, 15 Mar 2024"), @src());
    expect(b.getYear(day(friday), .{}) == 2024, @src());
    expect(b.getMonth(day(friday), .{}) == 2, @src());
    expect(b.getDate(day(friday), .{}) == 15, @src());
    expect(b.getDay(day(friday), .{}) == 5, @src());
    expect(b.getQuarter(day(friday), .{}) == 1, @src());
    expect(!b.isWeekend(day(friday), .{}), @src());
    expect(b.isFriday(day(friday), .{}), @src());
    expect(!b.isThursday(day(friday), .{}), @src());

    // Moved by days: a new Date, the old one unchanged.
    const monday = b.addDays(day(friday), 3, .{});
    expect(fmt(monday, "yyyy-MM-dd").eql("2024-03-18"), @src());
    expect(fmt(friday, "yyyy-MM-dd").eql("2024-03-15"), @src());
    expect(b.isWeekend(.{ .Date = b.addDays(day(friday), 1, .{}) }, .{}), @src());
    expect(fmt(b.subDays(day(friday), 15, .{}), "yyyy-MM-dd").eql("2024-02-29"), @src());
    expect(fmt(b.startOfMonth(day(friday), .{}), "d MMM yyyy").eql("1 Mar 2024"), @src());
    expect(fmt(b.endOfMonth(day(friday), .{}), "yyyy-MM-dd HH:mm").eql("2024-03-31 23:59"), @src());
    expect(fmt(b.lastDayOfMonth(day(friday), .{}), "d").eql("31"), @src());
    expect(fmt(b.nextMonday(day(friday), .{}), "yyyy-MM-dd").eql("2024-03-18"), @src());

    // Months and years clamp to the end of a short month.
    const jan31 = iso("2024-01-31");
    expect(fmt(b.addMonths(day(jan31), 1, .{}), "yyyy-MM-dd").eql("2024-02-29"), @src());
    expect(fmt(b.addYears(day(iso("2024-02-29")), 1, .{}), "yyyy-MM-dd").eql("2025-02-28"), @src());
    expect(fmt(b.subMonths(day(friday), 3, .{}), "yyyy-MM-dd").eql("2023-12-15"), @src());
    expect(fmt(b.addWeeks(day(friday), 2, .{}), "yyyy-MM-dd").eql("2024-03-29"), @src());
    expect(b.isLeapYear(day(friday), .{}), @src());
    expect(!b.isLeapYear(day(iso("2023-06-01")), .{}), @src());
    expect(b.getDaysInMonth(day(iso("2024-02-10")), .{}) == 29, @src());
    expect(b.getDaysInMonth(day(iso("2023-02-10")), .{}) == 28, @src());

    // An options object: weeks start on Monday.
    const sunday_start = b.startOfWeek(day(friday), .{});
    expect(fmt(sunday_start, "yyyy-MM-dd").eql("2024-03-10"), @src());
    const week = b.StartOfWeekOptions.create();
    week.setWeekStartsOn(1);
    expect(week.getWeekStartsOn().? == 1, @src());
    const monday_start = b.startOfWeek(day(friday), .{ .options = week });
    expect(fmt(monday_start, "yyyy-MM-dd").eql("2024-03-11"), @src());

    // Hours and the parts of a day.
    const noon = b.setHours(day(friday), 12, .{});
    expect(b.getHours(day(noon), .{}) == 12, @src());
    expect(fmt(noon, "h:mm a").eql("12:00 PM"), @src());
    expect(b.differenceInHours(.{ .Date = b.addHours(day(friday), 36, .{}) }, day(friday), .{}) == 36, @src());
    expect(fmt(b.startOfDay(day(noon), .{}), "HH:mm").eql("00:00"), @src());
    expect(fmt(b.endOfDay(day(noon), .{}), "HH:mm:ss").eql("23:59:59"), @src());

    // Compared: dates, ISO strings and timestamps alike.
    expect(b.differenceInDays(day(monday), day(friday), .{}) == 3, @src());
    expect(b.differenceInMonths(day(iso("2024-06-15")), day(friday), .{}) == 3, @src());
    expect(b.differenceInCalendarDays(day(monday), day(friday), .{}) == 3, @src());
    expect(b.isAfter(day(monday), .{ .string = b.JsString.from("2024-03-15") }), @src());
    expect(b.isBefore(day(friday), .{ .string = b.JsString.from("2024-03-16") }), @src());
    expect(b.compareAsc(day(friday), .{ .number = monday.time() }) == -1, @src());
    expect(b.compareAsc(day(monday), day(friday)) == 1, @src());
    expect(b.compareAsc(day(friday), day(iso("2024-03-15"))) == 0, @src());
    expect(b.isEqual(day(friday), .{ .number = friday.time() }), @src());
    expect(b.isSameDay(day(friday), day(b.setHours(day(friday), 18, .{})), .{}), @src());
    expect(b.isSameMonth(day(friday), day(monday), .{}), @src());
    expect(!b.isSameMonth(day(friday), day(jan31), .{}), @src());
    expect(b.formatDistance(day(monday), day(friday), .{}).eql("3 days"), @src());
    const suffixed = b.FormatDistanceOptions.create();
    suffixed.setAddSuffix(true);
    expect(b.formatDistance(day(monday), day(friday), .{ .options = suffixed }).eql("in 3 days"), @src());
    expect(b.formatDistance(day(friday), day(monday), .{ .options = suffixed }).eql("3 days ago"), @src());

    // max, min and closestTo take arrays of dates, numbers or strings.
    const both = b.JsArrayOf(b.DateArg).of(&.{ day(friday), day(monday) });
    expect(b.max(both, .{}).time() == monday.time(), @src());
    expect(b.min(both, .{}).time() == friday.time(), @src());
    const mixed = b.JsArrayOf(b.DateArg).new();
    mixed.push(.{ .Date = friday });
    mixed.push(.{ .number = monday.time() });
    // A string argument goes through `new Date(text)`: a bare date is UTC, a date and time is local.
    mixed.push(.{ .string = b.JsString.from("2024-04-01T00:00:00") });
    expect(mixed.len() == 3 and b.max(mixed, .{}).time() == iso("2024-04-01").time(), @src());
    const near = b.closestTo(.{ .Date = iso("2024-03-17") }, both, .{});
    expect(near != null and near.?.time() == monday.time(), @src());

    // An interval, an object made here, as a duration.
    const span = b.Interval.create();
    span.setStart(day(friday));
    span.setEnd(day(b.addDays(day(friday), 10, .{})));
    const length = b.intervalToDuration(span, .{});
    expect(length.getDays().? == 10, @src());
    expect(b.formatDuration(length, .{}).eql("10 days"), @src());
    expect(b.isWithinInterval(day(monday), span, .{}), @src());
    expect(!b.isWithinInterval(day(iso("2024-04-30")), span, .{}), @src());
    // The same, read back: an interval's ends are DateArgs.
    expect(span.getStart().Date.time() == friday.time(), @src());

    // A longer interval splits into years, months and days.
    const long = b.Interval.create();
    long.setStart(day(friday));
    long.setEnd(day(iso("2025-05-17")));
    const parts = b.intervalToDuration(long, .{});
    expect(parts.getYears().? == 1 and parts.getMonths().? == 2 and parts.getDays().? == 2, @src());
    expect(b.formatDuration(parts, .{}).eql("1 year 2 months 2 days"), @src());
    const joined = b.FormatDurationOptions.create();
    joined.setDelimiter(", ");
    expect(b.formatDuration(parts, .{ .options = joined }).eql("1 year, 2 months, 2 days"), @src());

    // Every day in a range, as an array of dates.
    const week_span = b.Interval.create();
    week_span.setStart(day(friday));
    week_span.setEnd(day(monday));
    const days = b.eachDayOfInterval(week_span, .{});
    expect(days.len() == 4, @src());
    expect(fmt(days.get(0).Date, "d").eql("15") and fmt(days.get(3).Date, "d").eql("18"), @src());
    // Weekends of a month: March 2024 has five Saturdays/Sundays pairs.
    expect(b.eachWeekendOfMonth(day(friday), .{}).len() == 10, @src());

    // A duration made here, added.
    const later = b.Duration.create();
    later.setMonths(1);
    later.setDays(2);
    expect(later.getYears() == null and later.getMonths().? == 1, @src());
    const moved = b.add(day(friday), later, .{});
    expect(fmt(moved, "yyyy-MM-dd").eql("2024-04-17"), @src());

    // Parsing with a format, and ISO output.
    const parsed = b.parse("15/03/2024", "dd/MM/yyyy", day(iso("2000-01-01")), .{});
    expect(parsed.time() == friday.time(), @src());
    const stamp = b.formatISO(day(friday), .{});
    expect(std.mem.startsWith(u8, stamp.copy(&iso_buf), "2024-03-15T00:00:00"), @src());

    // Timestamps: Date.fromTime and the Unix epoch.
    const epoch = b.fromUnixTime(86400, .{});
    expect(epoch.time() == 86400 * 1000, @src());
    expect(b.getUnixTime(day(epoch)) == 86400, @src());
    expect(b.toDate(.{ .number = 1000 }, .{}).time() == 1000, @src());
    expect(b.Date.fromTime(5000).time() == 5000, @src());

    // A week number and week-year, and relative wording.
    expect(b.getWeek(day(friday), .{}) == 11, @src());
    expect(b.formatRelative(day(friday), day(friday), .{}).len() > 0, @src());

    // Nonsense is no date.
    const bad = iso("not a date");
    expect(!b.isValid(.{ .h = bad.h }), @src());
    expect(b.isValid(.{ .h = friday.h }), @src());
}

var iso_buf: [64]u8 = undefined;
