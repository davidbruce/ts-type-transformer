module Demos.DateFns.RelativeTimeline

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

app.appendChild (wrap "div" "head" [ el "h1" "" "Date-fns Relative Timeline (F#)"; el "span" "badge" "Anchor 2024-03-15" ]) |> ignore

let anchor: JS.Date = parseISO "2024-03-15"
let opts = FormatDistanceOptions.Create (addSuffix = true)

let milestones =
    [ "2024-01-10", "Spec Draft"
      "2024-02-29", "Leap Day Freeze"
      "2024-03-12", "Beta Tag"
      "2024-03-15", "Release Anchor"
      "2024-03-18", "Patch Window"
      "2024-06-15", "Q2 Audit" ]
    |> List.map (fun (iso, title) -> (parseISO iso: JS.Date), title)

let timeline =
    wrap "div" "timeline" [
        for d, title in milestones ->
            let same = isSameDay (d, anchor)
            let cls = if same then "milestone anchor" else "milestone"
            let rel = if same then "today (anchor)" else formatDistance (d, anchor, opts)
            let strict = formatDistanceStrict (d, anchor)
            let cmp = int (compareAsc (d, anchor))
            let status =
                if same then "ANCHOR"
                elif isBefore (d, anchor) then "PAST"
                elif isAfter (d, anchor) then "FUTURE"
                else "OTHER"
            wrap "div" cls [
                el "span" "muted" (formatDate (d, "yyyy-MM-dd (EEE)"))
                el "span" "title" title
                el "span" "" rel
                el "span" "muted" (sprintf "%s · cmp %d" strict cmp)
                el "span" "pill" status
            ]
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Milestones Relative to 2024-03-15"; timeline ]) |> ignore

let dateArgs: ResizeArray<DateArg<JS.Date>> = ResizeArray [ for d, _ in milestones -> !^d ]
let earliest: JS.Date = min dateArgs
let latest: JS.Date = max dateArgs
let totalSpan = int (differenceInCalendarDays (latest, earliest))

let summary =
    wrap "div" "summary" [
        el "span" "" (sprintf "Earliest: %s" (formatDate (earliest, "yyyy-MM-dd")))
        el "span" "" (sprintf "Latest: %s" (formatDate (latest, "yyyy-MM-dd")))
        el "span" "" (sprintf "Total Span: %d days" totalSpan)
    ]

app.appendChild (wrap "div" "card" [ el "div" "label" "Extremes & Total Span"; summary ]) |> ignore
