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

let private makeSpan (sIso: string) (eIso: string) : Interval<DateArg<JS.Date>, DateArg<JS.Date>> =
    let s: JS.Date = parseISO sIso
    let e: JS.Date = parseISO eIso
    Interval.Create (s, e)

let private app = (document.getElementById "app").Value

app.appendChild (wrap "div" "head" [ el "h1" "" "Date-fns Interval Overlap & Clamp (F#)"; el "span" "badge" "March 2024 · 31 days" ]) |> ignore

let winSpan = makeSpan "2024-03-01" "2024-03-31"
let monthDays = eachDayOfInterval winSpan

let sprintA = makeSpan "2024-03-08" "2024-03-18"
let tracks =
    [ "Sprint A (Ref)", sprintA, true
      "Design Review", makeSpan "2024-03-03" "2024-03-10", false
      "QA Hardening", makeSpan "2024-03-15" "2024-03-25", false
      "Post-Release", makeSpan "2024-03-22" "2024-03-29", false ]

let gantt =
    wrap "div" "gantt" [
        yield wrap "div" "gantt-head" [
            el "div" "" "Track"
            wrap "div" "slots" [ for d in monthDays -> el "div" "slot-lbl" (formatDate (d, "d")) ]
            el "div" "" "vs Ref"
        ]
        for name, span, isRef in tracks do
            let overlaps = areIntervalsOverlapping (span, sprintA)
            let badgeText = if isRef then "REF" elif overlaps then "OVERLAP" else "DISJOINT"
            let badgeCls = if isRef || overlaps then "pill" else "pill-muted"
            let slots =
                wrap "div" "slots" [
                    for d in monthDays ->
                        let inTrack = isWithinInterval (d, span)
                        let inRef = isWithinInterval (d, sprintA)
                        let cls =
                            if inTrack && not isRef && inRef then "slot overlap"
                            elif inTrack then "slot active"
                            else "slot"
                        el "div" cls ""
                ]
            yield wrap "div" "gantt-row" [
                wrap "div" "track-meta" [ el "span" "track-name" name; el "span" "track-dur" (formatDuration (intervalToDuration span)) ]
                slots
                el "span" badgeCls badgeText
            ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "31-Day Gantt & Overlap Matrix (dark = active, grey = overlaps Sprint A)"; gantt ]) |> ignore

let tbl =
    wrap "table" "" [
        wrap "thead" "" [ wrap "tr" "" [ for h in [ "Probe Date"; "Clamped Output"; "Rule" ] -> el "th" "" h ] ]
        wrap "tbody" "" [
            for probeIso, rule in [ "2024-03-02", "Clamped to start"; "2024-03-14", "Inside interval"; "2024-03-28", "Clamped to end" ] ->
                let probe: JS.Date = parseISO probeIso
                let clamped = clamp (probe, sprintA)
                wrap "tr" "" [ el "td" "" probeIso; el "td" "" (formatDate (clamped, "yyyy-MM-dd")); el "td" "" rule ]
        ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Clamp Probes against Sprint A [2024-03-08 .. 2024-03-18]"; tbl ]) |> ignore
