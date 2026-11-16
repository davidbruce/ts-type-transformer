// date-fns from F#, in Node: dates made, moved, compared and formatted.
// Every check must hold.
module Test

open Fable.Core
open Bindings
open type Bindings.Globals

let mutable passed = 0

let check (ok: bool) (what: string) =
    if ok then passed <- passed + 1 else failwithf "check failed: %s" what

// A Friday, local midnight: what parseISO gives for a date alone.
let friday: JS.Date = parseISO "2024-03-15"
check (formatDate (friday, "yyyy-MM-dd") = "2024-03-15") "formatDate"
check (formatDate (friday, "EEEE") = "Friday") "the day's name"
check (getYear friday = 2024.0) "getYear"

// Moved by days: a new Date, the old one unchanged. A generic `DateArg` is
// an erased U3: a date is its first case.
let monday: JS.Date = addDays (U3.Case1 friday, 3.0)
check (formatDate (monday, "yyyy-MM-dd") = "2024-03-18") "addDays"
check (formatDate (friday, "yyyy-MM-dd") = "2024-03-15") "the old date is unchanged"

// Compared: dates and ISO strings alike.
check (differenceInDays (monday, friday) = 3.0) "differenceInDays"
check (isAfter (monday, "2024-03-15")) "isAfter a string"

printfn "%d checks passed" passed
