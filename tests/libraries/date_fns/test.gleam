// date-fns from Gleam, in Node: dates made, moved, compared and formatted.
// Every check must hold.
import bindings/areas/main as date_fns
import bindings/types as t
import gleam/int
import gleam/io
import gleam/option.{None}

fn check(ok: Bool, what: String) -> Int {
  case ok {
    True -> 1
    False -> panic as what
  }
}

pub fn main() {
  // A Friday, local midnight: what parseISO gives for a date alone.
  let friday = date_fns.parse_iso("2024-03-15", None, t.date_codec())
  let day = t.DateOrStringOrNumberDate
  let passed =
    check(date_fns.format_date(day(friday), "yyyy-MM-dd", None) == "2024-03-15", "format_date")
    + check(date_fns.format_date(day(friday), "EEEE", None) == "Friday", "the day's name")
    + check(date_fns.get_year(day(friday), None) == 2024.0, "get_year")

  // Moved by days: a new Date, the old one unchanged. A generic `DateArg`
  // takes its codec, and a date as its `DateType` case.
  let monday =
    date_fns.add_days(t.DateArgDateType(friday), 3.0, None, t.date_codec(), t.date_codec())
  let passed =
    passed
    + check(date_fns.format_date(day(monday), "yyyy-MM-dd", None) == "2024-03-18", "add_days")
    + check(date_fns.format_date(day(friday), "yyyy-MM-dd", None) == "2024-03-15", "the old date is unchanged")

  // Compared: dates and ISO strings alike.
  let passed =
    passed
    + check(date_fns.difference_in_days(day(monday), day(friday), None) == 3.0, "difference_in_days")
    + check(date_fns.is_after(day(monday), t.DateOrStringOrNumberString("2024-03-15")), "is_after a string")

  io.println(int.to_string(passed) <> " checks passed")
}
