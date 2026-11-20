// date-fns from Gleam, in Node: dates made, moved, compared and formatted.
// Every check must hold.
import bindings/areas/main as date_fns
import bindings/areas/main/duration
import bindings/areas/main/format_iso_options
import bindings/areas/main/interval
import bindings/areas/main/iso_format_options
import bindings/runtime
import bindings/types as t
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{None, Some}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() {
  // A Friday, local midnight: what parseISO gives for a date alone.
  let friday = date_fns.parse_iso("2024-03-15", None)
  let day = t.DateOrStringOrNumberDate
  let arg = t.DateArgDateType
  let fmt = fn(d) { date_fns.format_date(day(d), "yyyy-MM-dd", None) }
  let add_days = fn(d, n) {
    date_fns.add_days(arg(d), n, None)
  }
  let date_only = format_iso_options.create()
  iso_format_options.set_representation(date_only, Some(t.ISOStringRepresentationDate))
  let passed =
    check(fmt(friday) == "2024-03-15", "format_date")
    + check(date_fns.format_date(day(friday), "EEEE", None) == "Friday", "the day's name")
    + check(date_fns.get_year(day(friday), None) == 2024.0, "get_year")
    + check(date_fns.get_month(day(friday), None) == 2.0, "get_month is zero-based")
    + check(date_fns.get_date(day(friday), None) == 15.0, "get_date")
    + check(date_fns.get_day(day(friday), None) == 5.0, "get_day")
    + check(!date_fns.is_weekend(day(friday), None), "friday is not a weekend")
    + check(date_fns.is_leap_year(day(friday), None), "2024 is a leap year")
    + check(date_fns.get_days_in_month(day(friday), None) == 31.0, "days in March")
    + check(date_fns.get_quarter(day(friday), None) == 1.0, "quarter")

  // Moved by days: a new Date, the old one unchanged.
  let monday = add_days(friday, 3.0)
  let passed =
    passed
    + check(fmt(monday) == "2024-03-18", "add_days")
    + check(fmt(friday) == "2024-03-15", "the old date is unchanged")
    + check(date_fns.is_weekend(day(add_days(friday, 1.0)), None), "saturday is a weekend")
    + check(fmt(date_fns.sub_days(arg(friday), 15.0, None)) == "2024-02-29", "sub_days into a leap day")
    + check(fmt(date_fns.add_months(arg(friday), 1.0, None)) == "2024-04-15", "add_months")
    + check(fmt(date_fns.next_friday(arg(friday), None)) == "2024-03-22", "next_friday")
  let first = date_fns.start_of_month(arg(friday), None)
  let last = date_fns.end_of_month(arg(friday), None)
  let passed =
    passed
    + check(date_fns.format_date(day(first), "d MMM yyyy", None) == "1 Mar 2024", "start_of_month")
    + check(date_fns.format_date(day(last), "yyyy-MM-dd HH:mm", None) == "2024-03-31 23:59", "end_of_month")
    + check(fmt(date_fns.start_of_week(arg(friday), None)) == "2024-03-10", "start_of_week (Sunday)")
    + check(fmt(date_fns.start_of_year(arg(friday), None)) == "2024-01-01", "start_of_year")
    + check(date_fns.format_date(day(date_fns.set_hours(arg(friday), 13.0, None)), "HH:mm", None) == "13:00", "set_hours")
    + check(fmt(date_fns.set_month(arg(friday), 0.0, None)) == "2024-01-15", "set_month")
    + check(date_fns.difference_in_hours(day(add_days(friday, 1.0)), day(friday), None) == 24.0, "difference_in_hours")

  // Compared: dates, ISO strings and timestamps alike.
  let time = date_fns.get_time(day(monday))
  let passed =
    passed
    + check(date_fns.difference_in_days(day(monday), day(friday), None) == 3.0, "difference_in_days")
    + check(date_fns.is_after(day(monday), t.DateOrStringOrNumberString("2024-03-15")), "is_after a string")
    + check(date_fns.is_before(day(friday), t.DateOrStringOrNumberNumber(time)), "is_before a timestamp")
    + check(date_fns.is_same_day(day(friday), t.DateOrStringOrNumberString("2024-03-15"), None), "is_same_day")
    + check(!date_fns.is_same_day(day(friday), day(monday), None), "not the same day")
    + check(date_fns.compare_asc(day(friday), t.DateOrStringOrNumberNumber(time)) == -1.0, "compare_asc")
    + check(date_fns.format_distance(day(monday), day(friday), None) == "3 days", "format_distance")
    + check(date_fns.format_distance_strict(day(monday), day(friday), None) == "3 days", "format_distance_strict")
  let latest = date_fns.max([arg(friday), arg(monday)], None)
  let earliest = date_fns.min([arg(friday), arg(monday)], None)
  let passed =
    passed
    + check(date_fns.get_time(day(latest)) == time, "max")
    + check(fmt(earliest) == "2024-03-15", "min")
    + check(date_fns.get_unix_time(day(monday)) == time /. 1000.0, "get_unix_time")
    + check(date_fns.get_time(day(date_fns.from_unix_time(time /. 1000.0, None))) == time, "from_unix_time")
    + check(date_fns.format_iso(day(friday), Some(date_only)) == "2024-03-15", "format_iso date only")

  // An interval, made here, as a duration and as days.
  let span = interval.create()
  interval.set_start(span, arg(friday), t.date_arg_codec(t.date_codec()))
  interval.set_end(span, arg(add_days(friday, 10.0)), t.date_arg_codec(t.date_codec()))
  let length = date_fns.interval_to_duration(span, None)
  let passed =
    passed
    + check(duration.get_days(length) == Some(10.0), "interval_to_duration")
    + check(date_fns.format_duration(length, None) == "10 days", "format_duration")
    + check(date_fns.format_iso_duration(length) == "P0Y0M10DT0H0M0S", "format_iso_duration")
    + check(date_fns.is_within_interval(day(add_days(friday, 5.0)), span, None), "is_within_interval")
    + check(!date_fns.is_within_interval(day(add_days(friday, 20.0)), span, None), "outside the interval")
    + check(
      list.length(date_fns.each_day_of_interval(span, None, runtime.codec_dynamic())) == 11,
      "each_day_of_interval",
    )

  // A duration made here, added.
  let later = duration.create()
  duration.set_months(later, Some(1.0))
  duration.set_days(later, Some(2.0))
  let moved = date_fns.add(arg(friday), later, None)
  let passed =
    passed
    + check(fmt(moved) == "2024-04-17", "add a duration")
    + check(fmt(date_fns.sub(arg(friday), later, None)) == "2024-02-13", "sub a duration")

  // Nonsense is no date.
  let bad = date_fns.parse_iso("not a date", None)
  let passed =
    passed
    + check(!date_fns.is_valid(runtime.dynamic(bad)), "nonsense is no date")
    + check(date_fns.is_valid(runtime.dynamic(friday)), "a date is valid")

  io.println(int.to_string(passed) <> " checks passed")
}
