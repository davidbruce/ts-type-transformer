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
head.appendChild (el "h1" "" (Some "Date-fns Relative Timeline (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "Anchor 2024-03-15")) |> ignore
app.appendChild head |> ignore

let anchor: JS.Date = parseISO "2024-03-15"
let opts = createEmpty<FormatDistanceOptions>
opts.addSuffix <- Some true

let rawMilestones =
    [ "2024-01-10", "Spec Draft"
      "2024-02-29", "Leap Day Freeze"
      "2024-03-12", "Beta Tag"
      "2024-03-15", "Release Anchor"
      "2024-03-18", "Patch Window"
      "2024-06-15", "Q2 Audit" ]

let milestones =
    rawMilestones |> List.map (fun (iso, title) -> (parseISO iso: JS.Date), title)

let tlCard = el "div" "card" None
tlCard.appendChild (el "div" "label" (Some "Milestones Relative to 2024-03-15")) |> ignore
let timeline = el "div" "timeline" None

for d, title in milestones do
    let same = isSameDay (d, anchor)
    let cls = if same then "milestone anchor" else "milestone"
    let row = el "div" cls None
    row.appendChild (el "span" "muted" (Some (formatDate (d, "yyyy-MM-dd (EEE)")))) |> ignore
    row.appendChild (el "span" "title" (Some title)) |> ignore
    let rel = if same then "today (anchor)" else formatDistance (d, anchor, opts)
    row.appendChild (el "span" "" (Some rel)) |> ignore
    let strict = formatDistanceStrict (d, anchor)
    let cmp = int (compareAsc (d, anchor))
    row.appendChild (el "span" "muted" (Some (sprintf "%s · cmp %d" strict cmp))) |> ignore
    let status =
        if same then "ANCHOR"
        elif isBefore (d, anchor) then "PAST"
        elif isAfter (d, anchor) then "FUTURE"
        else "OTHER"
    row.appendChild (el "span" "pill" (Some status)) |> ignore
    timeline.appendChild row |> ignore

tlCard.appendChild timeline |> ignore
app.appendChild tlCard |> ignore

let dateArgs = ResizeArray [ for d, _ in milestones -> dayArg d ]
let earliest: JS.Date = min dateArgs
let latest: JS.Date = max dateArgs
let totalSpan = int (differenceInCalendarDays (latest, earliest))

let sumCard = el "div" "card" None
sumCard.appendChild (el "div" "label" (Some "Extremes & Total Span")) |> ignore
let summary = el "div" "summary" None
summary.appendChild (el "span" "" (Some (sprintf "Earliest: %s" (formatDate (earliest, "yyyy-MM-dd"))))) |> ignore
summary.appendChild (el "span" "" (Some (sprintf "Latest: %s" (formatDate (latest, "yyyy-MM-dd"))))) |> ignore
summary.appendChild (el "span" "" (Some (sprintf "Total Span: %d days" totalSpan))) |> ignore
sumCard.appendChild summary |> ignore
app.appendChild sumCard |> ignore
