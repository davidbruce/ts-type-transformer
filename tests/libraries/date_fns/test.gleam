// date-fns from Gleam, in Node: dates made, moved, compared and formatted.
// Every check must hold.
import bindings as date_fns
import bindings/duration
import bindings/format_iso_options
import bindings/interval
import bindings/iso_format_options
import bindings/runtime
import bindings/types as t
import gleam/int
import gleam/io
import gleam/list
import gleam/option.{Some}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() {
  // A Friday, local midnight: what parseISO gives for a date alone.
  let friday = date_fns.parse_iso("2024-03-15")
  let fmt = fn(d) { date_fns.format_date(d, "yyyy-MM-dd") }
  let date_only = format_iso_options.create()
  iso_format_options.set_representation(
    date_only,
    Some(t.ISOStringRepresentationDate),
  )
  let passed =
    check(fmt(friday) == "2024-03-15", "format_date")
    + check(date_fns.format_date(friday, "EEEE") == "Friday", "the day's name")
    + check(date_fns.get_year(friday) == 2024.0, "get_year")
    + check(date_fns.get_month(friday) == 2.0, "get_month is zero-based")
    + check(date_fns.get_date(friday) == 15.0, "get_date")
    + check(date_fns.get_day(friday) == 5.0, "get_day")
    + check(!date_fns.is_weekend(friday), "friday is not a weekend")
    + check(date_fns.is_leap_year(friday), "2024 is a leap year")
    + check(date_fns.get_days_in_month(friday) == 31.0, "days in March")
    + check(date_fns.get_quarter(friday) == 1.0, "quarter")

  // Moved by days: a new Date, the old one unchanged.
  let monday = date_fns.add_days(friday, 3.0)
  let passed =
    passed
    + check(fmt(monday) == "2024-03-18", "add_days")
    + check(fmt(friday) == "2024-03-15", "the old date is unchanged")
    + check(
      date_fns.is_weekend(date_fns.add_days(friday, 1.0)),
      "saturday is a weekend",
    )
    + check(
      fmt(date_fns.sub_days(friday, 15.0)) == "2024-02-29",
      "sub_days into a leap day",
    )
    + check(fmt(date_fns.add_months(friday, 1.0)) == "2024-04-15", "add_months")
    + check(fmt(date_fns.next_friday(friday)) == "2024-03-22", "next_friday")
  let first = date_fns.start_of_month(friday)
  let last = date_fns.end_of_month(friday)
  let passed =
    passed
    + check(
      date_fns.format_date(first, "d MMM yyyy") == "1 Mar 2024",
      "start_of_month",
    )
    + check(
      date_fns.format_date(last, "yyyy-MM-dd HH:mm") == "2024-03-31 23:59",
      "end_of_month",
    )
    + check(
      fmt(date_fns.start_of_week(friday)) == "2024-03-10",
      "start_of_week (Sunday)",
    )
    + check(
      fmt(date_fns.start_of_year(friday)) == "2024-01-01",
      "start_of_year",
    )
    + check(
      date_fns.format_date(date_fns.set_hours(friday, 13.0), "HH:mm") == "13:00",
      "set_hours",
    )
    + check(fmt(date_fns.set_month(friday, 0.0)) == "2024-01-15", "set_month")
    + check(
      date_fns.difference_in_hours(date_fns.add_days(friday, 1.0), friday)
        == 24.0,
      "difference_in_hours",
    )

  // Compared: dates, ISO strings and timestamps alike. A string or a
  // timestamp beside a Date goes in by `_of`, each a `DateArg`.
  let day = t.DateArgDate
  let time = date_fns.get_time(monday)
  let passed =
    passed
    + check(
      date_fns.difference_in_days(monday, friday) == 3.0,
      "difference_in_days",
    )
    + check(
      date_fns.is_after_of(day(monday), t.DateArgString("2024-03-15")),
      "is_after a string",
    )
    + check(
      date_fns.is_before_of(day(friday), t.DateArgNumber(time)),
      "is_before a timestamp",
    )
    + check(
      date_fns.is_same_day_of(day(friday), t.DateArgString("2024-03-15")),
      "is_same_day",
    )
    + check(!date_fns.is_same_day(friday, monday), "not the same day")
    + check(
      date_fns.compare_asc_of(day(friday), t.DateArgNumber(time)) == -1.0,
      "compare_asc",
    )
    + check(
      date_fns.format_distance(monday, friday) == "3 days",
      "format_distance",
    )
    + check(
      date_fns.format_distance_strict(monday, friday) == "3 days",
      "format_distance_strict",
    )
  let latest = date_fns.max([day(friday), day(monday)])
  let earliest = date_fns.min([day(friday), day(monday)])
  let passed =
    passed
    + check(date_fns.get_time(latest) == time, "max")
    + check(fmt(earliest) == "2024-03-15", "min")
    + check(date_fns.get_unix_time(monday) == time /. 1000.0, "get_unix_time")
    + check(
      date_fns.get_time(date_fns.from_unix_time(time /. 1000.0)) == time,
      "from_unix_time",
    )
    + check(
      date_fns.format_iso_with(friday, Some(date_only)) == "2024-03-15",
      "format_iso date only",
    )

  // An interval, made here, as a duration and as days.
  let span = interval.create()
  interval.set_start(span, friday)
  interval.set_end(span, date_fns.add_days(friday, 10.0))
  let length = date_fns.interval_to_duration(span)
  let passed =
    passed
    + check(duration.get_days(length) == Some(10.0), "interval_to_duration")
    + check(date_fns.format_duration(length) == "10 days", "format_duration")
    + check(
      date_fns.format_iso_duration(length) == "P0Y0M10DT0H0M0S",
      "format_iso_duration",
    )
    + check(
      date_fns.is_within_interval(date_fns.add_days(friday, 5.0), span),
      "is_within_interval",
    )
    + check(
      !date_fns.is_within_interval(date_fns.add_days(friday, 20.0), span),
      "outside the interval",
    )
    + check(
      list.length(date_fns.each_day_of_interval(span)) == 11,
      "each_day_of_interval",
    )

  // A duration made here, added.
  let later = duration.create()
  duration.set_months(later, Some(1.0))
  duration.set_days(later, Some(2.0))
  let moved = date_fns.add(friday, later)
  let passed =
    passed
    + check(fmt(moved) == "2024-04-17", "add a duration")
    + check(fmt(date_fns.sub(friday, later)) == "2024-02-13", "sub a duration")

  // Nonsense is no date.
  let bad = date_fns.parse_iso("not a date")
  let passed =
    passed
    + check(!date_fns.is_valid(runtime.dynamic(bad)), "nonsense is no date")
    + check(date_fns.is_valid(runtime.dynamic(friday)), "a date is valid")

  io.println(int.to_string(passed) <> " checks passed")
}
