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

let private app = document.getElementById "app"

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Date-fns Calendar Grid (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "March 2024")) |> ignore
app.appendChild head |> ignore

let refDay: JS.Date = parseISO "2024-03-15"
let monthStart: JS.Date = startOfMonth (dayArg refDay)
let monthEnd: JS.Date = endOfMonth (dayArg refDay)
let offset = int (getDay monthStart)

let metrics = el "div" "metrics" None
let addMetric (lbl: string) (value: string) =
    let m = el "div" "metric" None
    m.appendChild (el "div" "label" (Some lbl)) |> ignore
    m.appendChild (el "div" "val" (Some value)) |> ignore
    metrics.appendChild m |> ignore

addMetric "Days in Month" (sprintf "%d" (int (getDaysInMonth refDay)))
addMetric "Quarter" (sprintf "Q%d" (int (getQuarter refDay)))
addMetric "ISO Week" (sprintf "W%d" (int (getISOWeek refDay)))
addMetric "Leap Year / DOY" (sprintf "%s · #%d" (if isLeapYear refDay then "Yes" else "No") (int (getDayOfYear refDay)))
app.appendChild metrics |> ignore

let card = el "div" "card" None
card.appendChild (el "div" "label" (Some "March 2024 — Reference 2024-03-15")) |> ignore

let grid = el "div" "cal-grid" None
for dow in [ "Sun"; "Mon"; "Tue"; "Wed"; "Thu"; "Fri"; "Sat" ] do
    grid.appendChild (el "div" "dow" (Some dow)) |> ignore

for _ in 0 .. offset - 1 do
    grid.appendChild (el "div" "day-cell empty" None) |> ignore

let span = Interval.Create (dayArg monthStart, dayArg monthEnd)
let days = eachDayOfInterval span
let refArg = DateOrStringOrNumber.Date refDay
for item in days do
    let isRef = isSameDay (item, refArg)
    let isWknd = isWeekend item
    let cls =
        if isRef then "day-cell today"
        elif isWknd then "day-cell weekend"
        else "day-cell"
    let cell = el "div" cls None
    cell.appendChild (el "span" "day-num" (Some (formatDate (item, "d")))) |> ignore
    let sub = if isRef then "REF" else formatDate (item, "EEE")
    cell.appendChild (el "span" "sub" (Some sub)) |> ignore
    grid.appendChild cell |> ignore

card.appendChild grid |> ignore
app.appendChild card |> ignore
