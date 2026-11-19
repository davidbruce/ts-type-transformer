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
head.appendChild (el "h1" "" (Some "Date-fns Format & Parse Cookbook (F#)")) |> ignore
head.appendChild (el "span" "badge" (Some "2024-03-15T14:30:45")) |> ignore
app.appendChild head |> ignore

let sample: JS.Date = parseISO "2024-03-15T14:30:45"
let parsed: JS.Date = parse ("15/03/2024", "dd/MM/yyyy", dayArg sample)

let fmtCard = el "div" "card" None
fmtCard.appendChild (el "div" "label" (Some "Format Tokens & ISO Output")) |> ignore

let tbl = el "table" "" None
let thead = el "thead" "" None
let hrow = el "tr" "" None
for h in [ "API / Pattern"; "Output"; "Notes" ] do
    hrow.appendChild (el "th" "" (Some h)) |> ignore
thead.appendChild hrow |> ignore
tbl.appendChild thead |> ignore

let tbody = el "tbody" "" None
let rows =
    [ "formatDate(\"yyyy-MM-dd\")", formatDate (sample, "yyyy-MM-dd"), "ISO date"
      "formatDate(\"EEEE, do MMMM yyyy\")", formatDate (sample, "EEEE, do MMMM yyyy"), "Ordinal full date"
      "formatDate(\"EEE, d MMM yy\")", formatDate (sample, "EEE, d MMM yy"), "Compact short date"
      "formatDate(\"HH:mm:ss\")", formatDate (sample, "HH:mm:ss"), "24-hour clock"
      "formatDate(\"h:mm a\")", formatDate (sample, "h:mm a"), "12-hour clock"
      "formatDate(\"'Q'Q · 'W'I\")", formatDate (sample, "'Q'Q · 'W'I"), "Quarter & ISO week"
      "formatDate(\"DDD 'of' yyyy\")", formatDate (sample, "DDD 'of' yyyy"), "Day of year"
      "lightFormat(\"yyyy/MM/dd HH:mm\")", lightFormat (DateOrStringOrNumber.Date sample, "yyyy/MM/dd HH:mm"), "Zero-locale light formatter"
      "formatISO(sample)", formatISO sample, "ISO 8601 timestamp"
      "parse(\"15/03/2024\", \"dd/MM/yyyy\")", formatDate (parsed, "yyyy-MM-dd"), "Custom parse roundtrip" ]

for api, output, note in rows do
    let tr = el "tr" "" None
    let tdApi = el "td" "" None
    tdApi.appendChild (el "code" "" (Some api)) |> ignore
    tr.appendChild tdApi |> ignore
    tr.appendChild (el "td" "" (Some output)) |> ignore
    tr.appendChild (el "td" "" (Some note)) |> ignore
    tbody.appendChild tr |> ignore

tbl.appendChild tbody |> ignore
fmtCard.appendChild tbl |> ignore
app.appendChild fmtCard |> ignore

let valCard = el "div" "card" None
valCard.appendChild (el "div" "label" (Some "Validation & Calendar Existence")) |> ignore

let vtbl = el "table" "" None
let vhead = el "thead" "" None
let vhrow = el "tr" "" None
for h in [ "Input"; "Check"; "Result" ] do
    vhrow.appendChild (el "th" "" (Some h)) |> ignore
vhead.appendChild vhrow |> ignore
vtbl.appendChild vhead |> ignore

let vbody = el "tbody" "" None
let goodIso: JS.Date = parseISO "2024-03-15"
let badIso: JS.Date = parseISO "not-a-date"
let checks =
    [ "\"2024-03-15\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", (isValid goodIso && isMatch ("2024-03-15", "yyyy-MM-dd")), "VALID"
      "\"not-a-date\"", "isValid(parseISO) && isMatch(\"yyyy-MM-dd\")", (isValid badIso && isMatch ("not-a-date", "yyyy-MM-dd")), "INVALID"
      "(2024, 1, 29)", "isExists(2024, 1, 29) leap Feb 29", isExists (2024.0, 1.0, 29.0), "EXISTS"
      "(2023, 1, 29)", "isExists(2023, 1, 29) non-leap Feb 29", isExists (2023.0, 1.0, 29.0), "INVALID" ]

for input, checkName, ok, labelText in checks do
    let tr = el "tr" "" None
    let tdIn = el "td" "" None
    tdIn.appendChild (el "code" "" (Some input)) |> ignore
    tr.appendChild tdIn |> ignore
    tr.appendChild (el "td" "" (Some checkName)) |> ignore
    let tdRes = el "td" "" None
    let pillCls = if ok then "pill" else "pill-muted"
    tdRes.appendChild (el "span" pillCls (Some labelText)) |> ignore
    tr.appendChild tdRes |> ignore
    vbody.appendChild tr |> ignore

vtbl.appendChild vbody |> ignore
valCard.appendChild vtbl |> ignore
app.appendChild valCard |> ignore
