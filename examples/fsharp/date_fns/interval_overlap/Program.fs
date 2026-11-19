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

let private makeSpan (sIso: string) (eIso: string) : Interval<DateArg<JS.Date>, DateArg<JS.Date>> =
    let s: JS.Date = parseISO sIso
    let e: JS.Date = parseISO eIso
    Interval.Create (dayArg s, dayArg e)

let private app = document.getElementById "app"

let head = el "div" "head" None
head.appendChild (el "h1" "" (Some "Date-fns Interval Overlap & Clamp (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "March 2024 · 31 days")) |> ignore
app.appendChild head |> ignore

let winSpan = makeSpan "2024-03-01" "2024-03-31"
let monthDays = eachDayOfInterval winSpan

let sprintA = makeSpan "2024-03-08" "2024-03-18"
let tracks =
    [ "Sprint A (Ref)", sprintA, true
      "Design Review", makeSpan "2024-03-03" "2024-03-10", false
      "QA Hardening", makeSpan "2024-03-15" "2024-03-25", false
      "Post-Release", makeSpan "2024-03-22" "2024-03-29", false ]

let ganttCard = el "div" "card" None
ganttCard.appendChild (el "div" "label" (Some "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)")) |> ignore

let gantt = el "div" "gantt" None
let ghead = el "div" "gantt-head" None
ghead.appendChild (el "div" "" (Some "Track")) |> ignore
let hSlots = el "div" "slots" None
for d in monthDays do
    hSlots.appendChild (el "div" "slot-lbl" (Some (formatDate (d, "d")))) |> ignore
ghead.appendChild hSlots |> ignore
ghead.appendChild (el "div" "" (Some "vs Ref")) |> ignore
gantt.appendChild ghead |> ignore

for name, span, isRef in tracks do
    let row = el "div" "gantt-row" None
    let meta = el "div" "track-meta" None
    meta.appendChild (el "span" "track-name" (Some name)) |> ignore
    meta.appendChild (el "span" "track-dur" (Some (formatDuration (intervalToDuration span)))) |> ignore
    row.appendChild meta |> ignore

    let slots = el "div" "slots" None
    for d in monthDays do
        let inTrack = isWithinInterval (d, span)
        let inRef = isWithinInterval (d, sprintA)
        let cls =
            if inTrack && not isRef && inRef then "slot overlap"
            elif inTrack then "slot active"
            else "slot"
        slots.appendChild (el "div" cls None) |> ignore
    row.appendChild slots |> ignore

    let overlaps = areIntervalsOverlapping (span, sprintA)
    let badgeText = if isRef then "REF" elif overlaps then "OVERLAP" else "DISJOINT"
    let badgeCls = if isRef || overlaps then "pill" else "pill-muted"
    row.appendChild (el "span" badgeCls (Some badgeText)) |> ignore
    gantt.appendChild row |> ignore

ganttCard.appendChild gantt |> ignore
app.appendChild ganttCard |> ignore

let clampCard = el "div" "card" None
clampCard.appendChild (el "div" "label" (Some "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]")) |> ignore

let tbl = el "table" "" None
let thead = el "thead" "" None
let hrow = el "tr" "" None
for h in [ "Probe Date"; "Clamped Output"; "Rule" ] do
    hrow.appendChild (el "th" "" (Some h)) |> ignore
thead.appendChild hrow |> ignore
tbl.appendChild thead |> ignore

let tbody = el "tbody" "" None
for probeIso, rule in [ "2024-03-02", "Clamped to start"; "2024-03-14", "Inside interval"; "2024-03-28", "Clamped to end" ] do
    let probe: JS.Date = parseISO probeIso
    let clamped = clamp (probe, sprintA)
    let tr = el "tr" "" None
    tr.appendChild (el "td" "" (Some probeIso)) |> ignore
    tr.appendChild (el "td" "" (Some (formatDate (clamped, "yyyy-MM-dd")))) |> ignore
    tr.appendChild (el "td" "" (Some rule)) |> ignore
    tbody.appendChild tr |> ignore

tbl.appendChild tbody |> ignore
clampCard.appendChild tbl |> ignore
app.appendChild clampCard |> ignore
