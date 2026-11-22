module Program

open Fable.Core
open Fable.Core.JsInterop
open Dom
open type Dom.Globals
open DateFns
open type DateFns.Globals

let private el (tag: string) (cls: string) (text: string) =
    let node = document.createElement tag
    if cls <> "" then node.className <- cls
    if text <> "" then node.textContent <- text
    node

let private wrap (tag: string) (cls: string) (children: Dom.HTMLElement list) =
    let node = el tag cls ""
    for ch in children do
        node.appendChild ch |> ignore
    node

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Date-fns Calendar Grid (F#)"; el "span" "badge" "March 2024" ]) |> ignore

let refDay: JS.Date = parseISO "2024-03-15"
let monthStart: JS.Date = startOfMonth !^refDay
let monthEnd: JS.Date = endOfMonth !^refDay
let offset = int (getDay monthStart)

let metric (lbl: string) (value: string) =
    wrap "div" "metric" [ el "div" "label" lbl; el "div" "val" value ]

app.appendChild
    (wrap "div" "metrics" [
        metric "Days in Month" (sprintf "%d" (int (getDaysInMonth refDay)))
        metric "Quarter" (sprintf "Q%d" (int (getQuarter refDay)))
        metric "ISO Week" (sprintf "W%d" (int (getISOWeek refDay)))
        metric "Leap Year / DOY" (sprintf "%s · #%d" (if isLeapYear refDay then "Yes" else "No") (int (getDayOfYear refDay)))
    ])
|> ignore

let grid = el "div" "cal-grid" ""
for dow in [ "Sun"; "Mon"; "Tue"; "Wed"; "Thu"; "Fri"; "Sat" ] do
    grid.appendChild (el "div" "dow" dow) |> ignore

for _ in 0 .. offset - 1 do
    grid.appendChild (el "div" "day-cell empty" "") |> ignore

let days = eachDayOfInterval (Interval.Create (monthStart, monthEnd))
for item in days do
    let isRef = isSameDay (item, refDay)
    let isWknd = isWeekend item
    let cls =
        if isRef then "day-cell today"
        elif isWknd then "day-cell weekend"
        else "day-cell"
    let sub = if isRef then "REF" else formatDate (item, "EEE")
    grid.appendChild (wrap "div" cls [ el "span" "day-num" (formatDate (item, "d")); el "span" "sub" sub ]) |> ignore

app.appendChild (wrap "div" "card" [ el "div" "label" "March 2024 — Reference 2024-03-15"; grid ]) |> ignore
