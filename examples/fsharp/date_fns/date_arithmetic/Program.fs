module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open DateFns
open type DateFns.Globals

let private el (tag: string) (cls: string) (text: string option) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    match text with
    | Some t -> node.textContent <- t
    | None -> ()
    node

let private dayArg (d: JS.Date) : DateArg<JS.Date> = U3.Case1 d

let private signed (v: int) =
    if v > 0 then sprintf "+%d" v else sprintf "%d" v

let private app = document.getElementById "app"

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Date-fns Arithmetic & Durations (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "Base 2024-03-15")) |> ignore
app.appendChild head |> ignore

let baseDate: JS.Date = parseISO "2024-03-15"
let jan31: JS.Date = parseISO "2024-01-31"
let dur = Duration.Create (months = 1.0, days = 2.0)

let ops: (string * JS.Date * string) list =
    [ "addDays(base, 3)", addDays (dayArg baseDate, 3.0), "yyyy-MM-dd"
      "subDays(base, 15)", subDays (dayArg baseDate, 15.0), "yyyy-MM-dd"
      "addWeeks(base, 2)", addWeeks (dayArg baseDate, 2.0), "yyyy-MM-dd"
      "addMonths(jan31, 1)", addMonths (dayArg jan31, 1.0), "yyyy-MM-dd"
      "subMonths(base, 3)", subMonths (dayArg baseDate, 3.0), "yyyy-MM-dd"
      "addYears(base, 1)", addYears (dayArg baseDate, 1.0), "yyyy-MM-dd"
      "startOfWeek(base)", startOfWeek (dayArg baseDate), "yyyy-MM-dd"
      "endOfMonth(base)", endOfMonth (dayArg baseDate), "yyyy-MM-dd HH:mm:ss"
      "add(base, { months: 1, days: 2 })", add (dayArg baseDate, dur), "yyyy-MM-dd"
      "base (immutable check)", baseDate, "yyyy-MM-dd" ]

let tblCard = el "div" "card" None
tblCard.appendChild (el "div" "label" (Some "Immutable Date Stepping")) |> ignore

let tbl = el "table" "" None
let thead = el "thead" "" None
let hrow = el "tr" "" None
for h in [ "Operation"; "Result"; "Δ Days (vs base)"; "Δ Hours" ] do
    hrow.appendChild (el "th" "" (Some h)) |> ignore
thead.appendChild hrow |> ignore
tbl.appendChild thead |> ignore

let tbody = el "tbody" "" None
for opName, res, fmt in ops do
    let tr = el "tr" "" None
    let tdOp = el "td" "" None
    tdOp.appendChild (el "code" "" (Some opName)) |> ignore
    tr.appendChild tdOp |> ignore
    tr.appendChild (el "td" "" (Some (formatDate (res, fmt)))) |> ignore
    let dd = int (differenceInCalendarDays (res, baseDate))
    let dh = int (differenceInHours (res, baseDate))
    tr.appendChild (el "td" "" (Some (signed dd))) |> ignore
    tr.appendChild (el "td" "" (Some (signed dh))) |> ignore
    tbody.appendChild tr |> ignore

tbl.appendChild tbody |> ignore
tblCard.appendChild tbl |> ignore
app.appendChild tblCard |> ignore

let footer = el "div" "card" None
footer.appendChild (el "div" "label" (Some "Duration & Unit Conversions")) |> ignore
let srow = el "div" "summary-row" None
let addKv (lbl: string) (v: string) =
    let box = el "div" "kv" None
    box.appendChild (el "span" "label" (Some lbl)) |> ignore
    box.appendChild (el "strong" "" (Some v)) |> ignore
    srow.appendChild box |> ignore

addKv "formatDuration(dur)" (formatDuration dur)
addKv "formatISODuration(dur)" (formatISODuration dur)
addKv "hoursToMinutes(2)" (sprintf "%d min" (int (hoursToMinutes 2.0)))
addKv "daysToWeeks(14)" (sprintf "%d wk" (int (daysToWeeks 14.0)))
footer.appendChild srow |> ignore
app.appendChild footer |> ignore
