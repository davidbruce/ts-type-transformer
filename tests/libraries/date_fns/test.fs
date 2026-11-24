// date-fns from F#, in Node: dates made, moved, compared and formatted.
// Every check must hold.
module Test

open Fable.Core
open Fable.Core.JsInterop
open Bindings
open type Bindings.Globals

let mutable passed = 0

let asDate (d: JS.Date) = d

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A Friday, local midnight: what parseISO gives for a date alone.
let friday: JS.Date = parseISO "2024-03-15"
check (formatDate (friday, "yyyy-MM-dd") = "2024-03-15") "formatDate"
check (formatDate (friday, "EEEE") = "Friday") "the day's name"
check (getYear friday = 2024.0) "getYear"
check (getMonth friday = 2.0) "getMonth is zero based"
check (getDate friday = 15.0) "getDate"
check (getDay friday = 5.0) "getDay"
check (isFriday friday && not (isWeekend friday)) "isFriday, isWeekend"

// Moved by days: a new Date, the old one unchanged.
let monday: JS.Date = addDays (friday, 3.0)
check (formatDate (monday, "yyyy-MM-dd") = "2024-03-18") "addDays"
check (formatDate (friday, "yyyy-MM-dd") = "2024-03-15") "the old date is unchanged"
check (isMonday monday) "isMonday"
let saturday: JS.Date = addDays (friday, 1.0)
check (isWeekend saturday && isSaturday saturday) "isWeekend"
let first: JS.Date = startOfMonth friday
check (formatDate (first, "d MMM yyyy") = "1 Mar 2024") "startOfMonth"
let last: JS.Date = endOfMonth friday
check (formatDate (last, "yyyy-MM-dd HH:mm:ss") = "2024-03-31 23:59:59") "endOfMonth"
check (isLastDayOfMonth (asDate (lastDayOfMonth friday))) "lastDayOfMonth"
check (formatDate (asDate (addMonths (asDate (parseISO "2024-01-31"), 1.0)), "yyyy-MM-dd") = "2024-02-29") "addMonths clamps to the month's end"
check (formatDate (asDate (subMonths (friday, 3.0)), "yyyy-MM-dd") = "2023-12-15") "subMonths across a year"
check (formatDate (asDate (addYears (friday, 1.0)), "yyyy-MM-dd") = "2025-03-15") "addYears"
check (formatDate (asDate (addWeeks (friday, 2.0)), "yyyy-MM-dd") = "2024-03-29") "addWeeks"
check (formatDate (asDate (addHours (friday, 36.0)), "yyyy-MM-dd HH") = "2024-03-16 12") "addHours"
check (formatDate (asDate (nextMonday friday), "yyyy-MM-dd") = "2024-03-18") "nextMonday"
check (formatDate (asDate (startOfWeek friday), "yyyy-MM-dd") = "2024-03-10") "startOfWeek is a Sunday"
check (formatDate (asDate (setMonth (friday, 0.0)), "yyyy-MM-dd") = "2024-01-15") "setMonth"
let values = DateValues.Create (year = 2000.0, month = 1.0, date = 29.0)
check (formatDate (asDate (set (friday, values)), "yyyy-MM-dd") = "2000-02-29") "set"

// Compared: dates and ISO strings alike.
check (differenceInDays (monday, friday) = 3.0) "differenceInDays"
check (differenceInHours (monday, friday) = 72.0) "differenceInHours"
check (differenceInMonths (asDate (parseISO "2024-06-15"), friday) = 3.0) "differenceInMonths"
check (differenceInCalendarDays (monday, friday) = 3.0) "differenceInCalendarDays"
check (isAfter (monday, "2024-03-15")) "isAfter a string"
check (isBefore (friday, monday)) "isBefore"
check (isEqual (friday,asDate (parseISO "2024-03-15"))) "isEqual"
check (isSameDay (friday, "2024-03-15")) "isSameDay"
check (isSameMonth (friday, monday)) "isSameMonth"
check (compareAsc (friday, monday) = -1.0 && compareDesc (friday, monday) = 1.0) "compareAsc, compareDesc"
check (compareAsc (friday, float (friday.getTime ())) = 0.0) "a timestamp is a date too"
check (formatDistance (monday, friday) = "3 days") "formatDistance"
check (formatDistanceStrict (monday, friday) = "3 days") "formatDistanceStrict"
let latest: JS.Date = max (ResizeArray [ DateArg.Date friday; DateArg.Date monday ])
check (latest.getTime () = monday.getTime()) "max"
let earliest: JS.Date = min (ResizeArray [ DateArg.Date friday; DateArg.Date monday ])
check (earliest.getTime () = friday.getTime()) "min"
check (differenceInCalendarDays (DateArg.Date friday, DateArg.String "2024-03-01") = 14.0) "explicit cases"

// Calendar facts.
check (getDaysInMonth friday = 31.0 && getDaysInMonth (asDate (parseISO "2024-02-10")) = 29.0) "getDaysInMonth"
check (isLeapYear friday && not (isLeapYear (asDate (parseISO "2023-01-01")))) "isLeapYear"
check (getQuarter friday = 1.0) "getQuarter"
check (getDayOfYear friday = 75.0) "getDayOfYear"
check (getISOWeek friday = 11.0) "getISOWeek"

// Formatting.
check ((formatISO friday).StartsWith "2024-03-15T00:00:00") "formatISO"
check (formatDate (asDate (parseISO "2024-03-15T10:20:30"), "HH:mm:ss, do MMMM yyyy") = "10:20:30, 15th March 2024") "format tokens"
check (formatDate (asDate (parseISO "2024-03-15T15:04:05"), "h:mm a") = "3:04 PM") "12 hour clock"
check (lightFormat (friday, "yyyy/MM/dd") = "2024/03/15") "lightFormat"
check (formatISODuration (Duration.Create (years = 1.0, months = 2.0, days = 3.0)) = "P1Y2M3DT0H0M0S") "formatISODuration"
check (getUnixTime (DateArg.Number 1500000000000.0) = 1500000000.0) "getUnixTime"

// An interval: dates in it, the span as a duration.
let span: Interval =
    Interval.Create (DateArg.Date friday, DateArg.Date (asDate (addDays (friday, 10.0))))
let length = intervalToDuration span
check (length.days = Some 10.0) "intervalToDuration"
check (formatDuration length = "10 days") "formatDuration"
check (isWithinInterval (DateArg.Date monday, span)) "isWithinInterval"
check (not (isWithinInterval (DateArg.String "2024-04-30", span))) "outside the interval"
let days = eachDayOfInterval span
check (days.Count = 11) "eachDayOfInterval"
let other = Interval.Create (DateArg.Date monday, DateArg.Date (addDays (monday, 20.0): JS.Date))
check (areIntervalsOverlapping (span, other)) "areIntervalsOverlapping"

// An interval made by date-fns.
let made = interval ("2024-03-15", "2024-03-20")
check (not (isNull (box made))) "interval"

// A duration made here, added.
let later = Duration.Create (months = 1.0, days = 2.0)
let moved: JS.Date = add (friday, later)
check (formatDate (moved, "yyyy-MM-dd") = "2024-04-17") "add a duration"
let back: JS.Date = sub (friday, later)
check (formatDate (back, "yyyy-MM-dd") = "2024-02-13") "sub a duration"
check (formatDuration (Duration.Create (hours = 2.0, minutes = 30.0)) = "2 hours 30 minutes") "formatDuration"

// Unit conversions.
check (hoursToMinutes 2.0 = 120.0 && minutesToSeconds 3.0 = 180.0) "hoursToMinutes, minutesToSeconds"
check (daysToWeeks 14.0 = 2.0) "daysToWeeks"
check (millisecondsToSeconds 5000.0 = 5.0) "millisecondsToSeconds"

// Parsing by a format; nonsense is no date.
let parsed: JS.Date = parse ("15/03/2024", "dd/MM/yyyy", friday)
check (formatDate (parsed, "yyyy-MM-dd") = "2024-03-15") "parse"
check (isMatch ("2024-03-15", "yyyy-MM-dd")) "isMatch"
check (not (isMatch ("nope", "yyyy-MM-dd"))) "isMatch rejects"
check (isValid friday) "isValid"
check (not (isValid (asDate (parseISO "not a date")))) "nonsense is no date"
check (isDate friday && not (isDate (box "2024-03-15"))) "isDate"
check (isExists (2024.0, 1.0, 29.0) && not (isExists (2023.0, 1.0, 29.0))) "isExists"

// Options: a plain object with named fields.
let withSuffix = createEmpty<FormatDistanceOptions> // FINDING 9: option types that inherit have no Create
withSuffix.addSuffix <- Some true
check (formatDistance (monday, friday, withSuffix) = "in 3 days") "formatDistance addSuffix"
check (formatDistance (friday, monday, withSuffix) = "3 days ago") "formatDistance addSuffix, past"

printfn "%d checks passed" passed
