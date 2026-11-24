module Demos.DateFns.DateArithmetic

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

let private signed (v: int) =
    if v > 0 then sprintf "+%d" v else sprintf "%d" v

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Date-fns Arithmetic & Durations (F#)"; el "span" "badge" "Base 2024-03-15" ]) |> ignore

let baseDate: JS.Date = parseISO "2024-03-15"
let jan31: JS.Date = parseISO "2024-01-31"
let dur = Duration.Create (months = 1.0, days = 2.0)

let ops: (string * JS.Date * string) list =
    [ "addDays(base, 3)", addDays (baseDate, 3.0), "yyyy-MM-dd"
      "subDays(base, 15)", subDays (baseDate, 15.0), "yyyy-MM-dd"
      "addWeeks(base, 2)", addWeeks (baseDate, 2.0), "yyyy-MM-dd"
      "addMonths(jan31, 1)", addMonths (jan31, 1.0), "yyyy-MM-dd"
      "subMonths(base, 3)", subMonths (baseDate, 3.0), "yyyy-MM-dd"
      "addYears(base, 1)", addYears (baseDate, 1.0), "yyyy-MM-dd"
      "startOfWeek(base)", startOfWeek baseDate, "yyyy-MM-dd"
      "endOfMonth(base)", endOfMonth baseDate, "yyyy-MM-dd HH:mm:ss"
      "add(base, { months: 1, days: 2 })", add (baseDate, dur), "yyyy-MM-dd"
      "base (immutable check)", baseDate, "yyyy-MM-dd" ]

let bodyRows =
    ops
    |> List.map (fun (opName, res, fmt) ->
        let dd = int (differenceInCalendarDays (res, baseDate))
        let dh = int (differenceInHours (res, baseDate))
        wrap "tr" "" [
            wrap "td" "" [ el "code" "" opName ]
            el "td" "" (formatDate (res, fmt))
            el "td" "" (signed dd)
            el "td" "" (signed dh)
        ])

let tbl =
    wrap "table" "" [
        wrap "thead" "" [ wrap "tr" "" [ for h in [ "Operation"; "Result"; "Δ Days (vs base)"; "Δ Hours" ] -> el "th" "" h ] ]
        wrap "tbody" "" bodyRows
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Immutable Date Stepping"; tbl ]) |> ignore

let kv (lbl: string) (v: string) =
    wrap "div" "kv" [ el "span" "label" lbl; el "strong" "" v ]

let srow =
    wrap "div" "summary-row" [
        kv "formatDuration(dur)" (formatDuration dur)
        kv "formatISODuration(dur)" (formatISODuration dur)
        kv "hoursToMinutes(2)" (sprintf "%d min" (int (hoursToMinutes 2.0)))
        kv "daysToWeeks(14)" (sprintf "%d wk" (int (daysToWeeks 14.0)))
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Duration & Unit Conversions"; srow ]) |> ignore
